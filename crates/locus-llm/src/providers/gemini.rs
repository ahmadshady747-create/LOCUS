use crate::provider::{BoxStream, CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub const DEFAULT_GEMINI_MODEL: &str = "gemini-2.0-flash";
pub const GEMINI_15_FLASH: &str = "gemini-1.5-flash";

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string()),
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn build_payload(prompt: &str, system: Option<&str>) -> Value {
        let mut contents = Vec::new();
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{"text": prompt}]
        }));

        let mut payload = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": 0.3,
                "maxOutputTokens": 8192
            }
        });

        if let Some(sys) = system {
            payload["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys}]
            });
        }

        payload
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<CompletionResponse, LlmError> {
        let start = Instant::now();
        let url = format!("{}/{}:generateContent?key={}", self.base_url, self.model, self.api_key);
        let payload = Self::build_payload(prompt, system);

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();

        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            warn!("Gemini API error {}: {}", status, error_text);
            return match status.as_u16() {
                401 | 403 => Err(LlmError::AuthFailed(format!("Invalid Gemini API Key: {}", error_text))),
                429 => Err(LlmError::RateLimited(format!("Gemini quota exceeded: {}", error_text))),
                404 => Err(LlmError::ModelNotFound(format!("Model '{}' not found: {}", self.model, error_text))),
                _ => Err(LlmError::ProviderUnavailable(format!("HTTP {}: {}", status, error_text))),
            };
        }

        let json: Value = resp.json().await.map_err(|e| LlmError::InvalidRequest(e.to_string()))?;
        
        let content = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = json["candidates"][0]["finishReason"]
            .as_str()
            .map(|s| s.to_string());

        let prompt_tokens = json["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = json["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as usize;

        Ok(CompletionResponse {
            content,
            model_used: self.model.clone(),
            provider_stamp: ProviderType::GeminiFlash,
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
        let url = format!("{}/{}:streamGenerateContent?alt=sse&key={}", self.base_url, self.model, self.api_key);
        let payload = Self::build_payload(prompt, system);

        let resp = self.client.post(&url).json(&payload).send().await?;
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
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                                if let Some(part) = v["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    accumulated.push_str(part);
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
        let url = format!("{}/{}:generateContent?key={}", self.base_url, self.model, self.api_key);
        let test_payload = serde_json::json!({
            "contents": [{"parts": [{"text": "ping"}]}],
            "generationConfig": {"maxOutputTokens": 2}
        });

        let resp = self.client.post(&url).json(&test_payload).send().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        if resp.status().is_success() {
            Ok(LatencyMetric {
                latency_ms,
                is_healthy: true,
                message: format!("Gemini Flash ({}) responding OK", self.model),
                provider_type: ProviderType::GeminiFlash,
            })
        } else {
            let status = resp.status();
            Err(match status.as_u16() {
                401 | 403 => LlmError::AuthFailed("Invalid Gemini API Key".to_string()),
                429 => LlmError::RateLimited("Gemini quota rate limit reached".to_string()),
                _ => LlmError::ProviderUnavailable(format!("HTTP {}", status)),
            })
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::GeminiFlash
    }
}
