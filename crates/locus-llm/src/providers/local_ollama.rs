use crate::provider::{BoxStream, CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5-coder:7b";
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

#[derive(Debug, Clone)]
pub struct LocalOllamaProvider {
    client: Client,
    model: String,
    base_url: String,
}

impl LocalOllamaProvider {
    pub fn new(model: Option<String>, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_default();

        Self {
            client,
            model: model.unwrap_or_else(|| DEFAULT_OLLAMA_MODEL.to_string()),
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl LlmProvider for LocalOllamaProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<CompletionResponse, LlmError> {
        let start = Instant::now();
        let url = format!("{}/api/generate", self.base_url);

        let mut payload = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.2
            }
        });

        if let Some(sys) = system {
            payload["system"] = serde_json::json!(sys);
        }

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();

        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            warn!("Ollama API error {}: {}", status, error_text);
            return match status.as_u16() {
                404 => Err(LlmError::ModelNotFound(format!("Model '{}' not found in Ollama: {}", self.model, error_text))),
                _ => Err(LlmError::ProviderUnavailable(format!("HTTP {}: {}", status, error_text))),
            };
        }

        let json: Value = resp.json().await.map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let content = json["response"].as_str().unwrap_or("").to_string();
        let prompt_tokens = json["prompt_eval_count"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = json["eval_count"].as_u64().unwrap_or(0) as usize;
        let finish_reason = if json["done"].as_bool().unwrap_or(false) {
            Some("stop".to_string())
        } else {
            None
        };

        Ok(CompletionResponse {
            content,
            model_used: self.model.clone(),
            provider_stamp: ProviderType::LocalOllama,
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
        let url = format!("{}/api/generate", self.base_url);

        let mut payload = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": true,
            "options": {
                "temperature": 0.2
            }
        });

        if let Some(sys) = system {
            payload["system"] = serde_json::json!(sys);
        }

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();

        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(LlmError::ProviderUnavailable(error_text));
        }

        let byte_stream = resp.bytes_stream().map_err(LlmError::from);

        let transformed = byte_stream.map(|chunk_res| {
            match chunk_res {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let mut accumulated = String::new();

                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(v) = serde_json::from_str::<Value>(line) {
                            if let Some(part) = v["response"].as_str() {
                                accumulated.push_str(part);
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
        let url = format!("{}/api/tags", self.base_url);

        let resp = self.client.get(&url).send().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        if resp.status().is_success() {
            Ok(LatencyMetric {
                latency_ms,
                is_healthy: true,
                message: format!("Local Ollama daemon running at {}", self.base_url),
                provider_type: ProviderType::LocalOllama,
            })
        } else {
            Err(LlmError::ProviderUnavailable(format!("HTTP {}", resp.status())))
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::LocalOllama
    }
}
