use crate::provider::{BoxStream, CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub const DEFAULT_OPENROUTER_FREE_MODEL: &str = "meta-llama/llama-3.3-70b-instruct:free";
pub const OPENROUTER_DEEPSEEK_FREE: &str = "deepseek/deepseek-r1:free";
pub const OPENROUTER_GEMINI_FREE: &str = "google/gemini-2.0-flash-exp:free";

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_OPENROUTER_FREE_MODEL.to_string()),
            base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn build_messages(prompt: &str, system: Option<&str>) -> Vec<Value> {
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));
        messages
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<CompletionResponse, LlmError> {
        let start = Instant::now();
        let payload = serde_json::json!({
            "model": self.model,
            "messages": Self::build_messages(prompt, system),
            "temperature": 0.3,
            "max_tokens": 8192
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/locus-ai/locus")
            .header("X-Title", "LOCUS AI Assistant")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            warn!("OpenRouter API error {}: {}", status, error_text);
            return match status.as_u16() {
                401 | 403 => Err(LlmError::AuthFailed(format!("OpenRouter auth failed: {}", error_text))),
                429 => Err(LlmError::RateLimited(format!("OpenRouter rate limit on free tier: {}", error_text))),
                404 => Err(LlmError::ModelNotFound(format!("Model '{}' not found: {}", self.model, error_text))),
                _ => Err(LlmError::ProviderUnavailable(format!("HTTP {}: {}", status, error_text))),
            };
        }

        let json: Value = resp.json().await.map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = json["choices"][0]["finish_reason"]
            .as_str()
            .map(|s| s.to_string());

        let prompt_tokens = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(CompletionResponse {
            content,
            model_used: self.model.clone(),
            provider_stamp: ProviderType::OpenRouter,
            token_usage: TokenUsage::new(prompt_tokens, completion_tokens),
            latency_ms: start.elapsed().as_millis() as u64,
            finish_reason,
        })
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<BoxStream<Result<String, LlmError>>, LlmError> {
        let payload = serde_json::json!({
            "model": self.model,
            "messages": Self::build_messages(prompt, system),
            "temperature": 0.3,
            "max_tokens": 8192,
            "stream": true
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/locus-ai/locus")
            .header("X-Title", "LOCUS AI Assistant")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(LlmError::AuthFailed(error_text)),
                429 => Err(LlmError::RateLimited(error_text)),
                _ => Err(LlmError::ProviderUnavailable(error_text)),
            };
        }

        let byte_stream = resp.bytes_stream().map_err(LlmError::from);

        let transformed = byte_stream.map(|chunk_res| {
            match chunk_res {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut accumulated = String::new();

                    for line in text.lines() {
                        if line == "data: [DONE]" {
                            continue;
                        }
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                                    accumulated.push_str(delta);
                                }
                            }
                        }
                    }
                    Ok(accumulated)
                }
                Err(e) => Err(e),
            }
        });

        Ok(Box::pin(transformed))
    }

    async fn health_check(&self) -> Result<LatencyMetric, LlmError> {
        let start = Instant::now();
        let payload = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 2
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/locus-ai/locus")
            .header("X-Title", "LOCUS AI Assistant")
            .json(&payload)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if resp.status().is_success() {
            Ok(LatencyMetric {
                latency_ms,
                is_healthy: true,
                message: format!("OpenRouter Free Tier ({}) responding OK", self.model),
                provider_type: ProviderType::OpenRouter,
            })
        } else {
            let status = resp.status();
            Err(match status.as_u16() {
                401 | 403 => LlmError::AuthFailed("Invalid OpenRouter API Key".to_string()),
                429 => LlmError::RateLimited("OpenRouter free tier rate limit exceeded".to_string()),
                _ => LlmError::ProviderUnavailable(format!("HTTP {}", status)),
            })
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenRouter
    }
}
