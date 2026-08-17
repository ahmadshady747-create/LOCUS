use crate::provider::{BoxStream, CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub const DEFAULT_GROQ_MODEL: &str = "llama-3.3-70b-versatile";
pub const GROQ_DEEPSEEK_R1: &str = "deepseek-r1-distill-llama-70b";

#[derive(Debug, Clone)]
pub struct GroqProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GroqProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_GROQ_MODEL.to_string()),
            base_url: "https://api.groq.com/openai/v1/chat/completions".to_string(),
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
impl LlmProvider for GroqProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<CompletionResponse, LlmError> {
        let start = Instant::now();
        let payload = serde_json::json!({
            "model": self.model,
            "messages": Self::build_messages(prompt, system),
            "temperature": 0.2,
            "max_tokens": 8192
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            warn!("Groq API error {}: {}", status, error_text);
            return match status.as_u16() {
                401 | 403 => Err(LlmError::AuthFailed(format!("Groq auth failed: {}", error_text))),
                429 => Err(LlmError::RateLimited(format!("Groq rate limit exceeded: {}", error_text))),
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
            provider_stamp: ProviderType::Groq,
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
            "temperature": 0.2,
            "max_tokens": 8192,
            "stream": true
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
            .json(&payload)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if resp.status().is_success() {
            Ok(LatencyMetric {
                latency_ms,
                is_healthy: true,
                message: format!("Groq Ultra-Fast ({}) responding OK", self.model),
                provider_type: ProviderType::Groq,
            })
        } else {
            let status = resp.status();
            Err(match status.as_u16() {
                401 | 403 => LlmError::AuthFailed("Invalid Groq API Key".to_string()),
                429 => LlmError::RateLimited("Groq rate limit reached".to_string()),
                _ => LlmError::ProviderUnavailable(format!("HTTP {}", status)),
            })
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Groq
    }
}
