use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    LocalOllama,
    GeminiFlash,
    Groq,
    OpenRouter,
    Custom(String),
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::LocalOllama => write!(f, "LocalOllama"),
            ProviderType::GeminiFlash => write!(f, "GeminiFlash"),
            ProviderType::Groq => write!(f, "Groq"),
            ProviderType::OpenRouter => write!(f, "OpenRouter"),
            ProviderType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

impl TokenUsage {
    pub fn new(prompt: usize, completion: usize) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        }
    }

    pub fn zero() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model_used: String,
    pub provider_stamp: ProviderType,
    pub token_usage: TokenUsage,
    pub latency_ms: u64,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetric {
    pub latency_ms: u64,
    pub is_healthy: bool,
    pub message: String,
    pub provider_type: ProviderType,
}

#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmError {
    #[error("Rate limit exceeded on provider: {0}")]
    RateLimited(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Network request timed out: {0}")]
    NetworkTimeout(String),

    #[error("Provider is currently unavailable or unreachable: {0}")]
    ProviderUnavailable(String),

    #[error("Model not found or unsupported: {0}")]
    ModelNotFound(String),

    #[error("Invalid request payload or configuration: {0}")]
    InvalidRequest(String),

    #[error("Streaming error encountered: {0}")]
    StreamError(String),

    #[error("LLM Provider error: {0}")]
    Other(String),
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::NetworkTimeout(err.to_string())
        } else if err.is_connect() {
            LlmError::ProviderUnavailable(format!("Connection failed: {}", err))
        } else if let Some(status) = err.status() {
            match status.as_u16() {
                401 | 403 => LlmError::AuthFailed(format!("HTTP {}: {}", status, err)),
                429 => LlmError::RateLimited(format!("HTTP 429 Too Many Requests: {}", err)),
                404 => LlmError::ModelNotFound(format!("HTTP 404 Not Found: {}", err)),
                400 | 422 => LlmError::InvalidRequest(format!("HTTP {}: {}", status, err)),
                500..=599 => LlmError::ProviderUnavailable(format!("HTTP {}: {}", status, err)),
                _ => LlmError::Other(err.to_string()),
            }
        } else {
            LlmError::Other(err.to_string())
        }
    }
}

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<CompletionResponse, LlmError>;
    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<BoxStream<Result<String, LlmError>>, LlmError>;
    async fn health_check(&self) -> Result<LatencyMetric, LlmError>;
    fn provider_type(&self) -> ProviderType;
}
