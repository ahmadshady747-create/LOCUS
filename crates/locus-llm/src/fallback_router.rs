use crate::keyring_store::KeyringStore;
use crate::llamacpp::LlamaCppClient;
use crate::ollama::OllamaClient;
use crate::types::{BackendType, GenerationRequest, GenerationResponse};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FallbackStrategy {
    LocalFirst,
    CloudFirst,
    SpeedFirst,
    CustomOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FallbackTarget {
    pub id: String,
    pub label: String,
    pub is_local: bool,
    pub enabled: bool,
    pub preferred_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackChainConfig {
    pub enabled: bool,
    pub strategy: FallbackStrategy,
    pub targets: Vec<FallbackTarget>,
    pub max_retries_per_target: u32,
    pub timeout_seconds: u64,
}

impl Default for FallbackChainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: FallbackStrategy::LocalFirst,
            targets: vec![
                FallbackTarget {
                    id: "ollama".to_string(),
                    label: "Local Ollama".to_string(),
                    is_local: true,
                    enabled: true,
                    preferred_model: Some("llama3".to_string()),
                },
                FallbackTarget {
                    id: "llamacpp".to_string(),
                    label: "Local llama.cpp".to_string(),
                    is_local: true,
                    enabled: true,
                    preferred_model: None,
                },
                FallbackTarget {
                    id: "deepseek".to_string(),
                    label: "DeepSeek Coder".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("deepseek-coder".to_string()),
                },
                FallbackTarget {
                    id: "openai".to_string(),
                    label: "OpenAI GPT-4o".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("gpt-4o".to_string()),
                },
                FallbackTarget {
                    id: "anthropic".to_string(),
                    label: "Anthropic Claude".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("claude-3-5-sonnet-20241022".to_string()),
                },
                FallbackTarget {
                    id: "gemini".to_string(),
                    label: "Google Gemini".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("gemini-1.5-pro".to_string()),
                },
                FallbackTarget {
                    id: "groq".to_string(),
                    label: "Groq High-Speed".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("llama-3.3-70b-versatile".to_string()),
                },
                FallbackTarget {
                    id: "mistral".to_string(),
                    label: "Mistral Codestral".to_string(),
                    is_local: false,
                    enabled: true,
                    preferred_model: Some("codestral-latest".to_string()),
                },
            ],
            max_retries_per_target: 1,
            timeout_seconds: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackExecutionResult {
    pub content: String,
    pub target_used: String,
    pub model_used: String,
    pub was_fallback: bool,
    pub attempts: Vec<TargetAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAttempt {
    pub target_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct FallbackRouter {
    config: Arc<RwLock<FallbackChainConfig>>,
    ollama: Option<OllamaClient>,
    llamacpp: Option<LlamaCppClient>,
}

impl FallbackRouter {
    pub fn new(
        config: FallbackChainConfig,
        ollama: Option<OllamaClient>,
        llamacpp: Option<LlamaCppClient>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            ollama,
            llamacpp,
        }
    }

    pub async fn get_config(&self) -> FallbackChainConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: FallbackChainConfig) {
        let mut cfg = self.config.write().await;
        *cfg = new_config;
        info!("Updated Auto-Fallback Chain Router configuration");
    }

    pub async fn set_strategy(&self, strategy: FallbackStrategy) {
        let mut cfg = self.config.write().await;
        cfg.strategy = strategy.clone();

        match strategy {
            FallbackStrategy::LocalFirst => {
                cfg.targets.sort_by_key(|t| if t.is_local { 0 } else { 1 });
            }
            FallbackStrategy::CloudFirst => {
                cfg.targets.sort_by_key(|t| if t.is_local { 1 } else { 0 });
            }
            FallbackStrategy::SpeedFirst => {
                let speed_order = ["groq", "ollama", "llamacpp", "deepseek", "gemini", "mistral", "openai", "anthropic"];
                cfg.targets.sort_by_key(|t| {
                    speed_order.iter().position(|&x| x == t.id).unwrap_or(99)
                });
            }
            FallbackStrategy::CustomOrder => {}
        }
        info!("Adjusted Auto-Fallback priority order according to strategy {:?}", strategy);
    }

    pub async fn execute_routed_generate(
        &self,
        prompt: &str,
        requested_model: Option<&str>,
    ) -> Result<FallbackExecutionResult> {
        let config = self.get_config().await;
        let mut attempts = Vec::new();

        let active_targets: Vec<FallbackTarget> = config
            .targets
            .into_iter()
            .filter(|t| t.enabled)
            .collect();

        if active_targets.is_empty() {
            return Err(anyhow!("No active providers configured in Fallback Router"));
        }

        let first_target_id = active_targets[0].id.clone();

        for target in &active_targets {
            let attempt_start = std::time::Instant::now();
            let target_id = target.id.as_str();

            let target_model = requested_model
                .or(target.preferred_model.as_deref())
                .unwrap_or("default");

            match target_id {
                "ollama" => {
                    if let Some(ref ollama) = self.ollama {
                        let req = GenerationRequest {
                            model: target_model.to_string(),
                            prompt: prompt.to_string(),
                            stream: false,
                            options: None,
                            system: None,
                            template: None,
                            context: None,
                            images: None,
                            format: None,
                            keep_alive: None,
                        };

                        match tokio::time::timeout(
                            Duration::from_secs(config.timeout_seconds),
                            ollama.generate(req),
                        )
                        .await
                        {
                            Ok(Ok(res)) => {
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: true,
                                    error: None,
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });

                                return Ok(FallbackExecutionResult {
                                    content: res.response,
                                    target_used: target.id.clone(),
                                    model_used: res.model,
                                    was_fallback: target.id != first_target_id,
                                    attempts,
                                });
                            }
                            Ok(Err(e)) => {
                                warn!("Target 'ollama' failed: {}", e);
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: false,
                                    error: Some(e.to_string()),
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });
                            }
                            Err(_) => {
                                warn!("Target 'ollama' timed out after {}s", config.timeout_seconds);
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: false,
                                    error: Some("Timeout".to_string()),
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    }
                }

                "llamacpp" => {
                    if let Some(ref llamacpp) = self.llamacpp {
                        let req = GenerationRequest {
                            model: target_model.to_string(),
                            prompt: prompt.to_string(),
                            stream: false,
                            options: None,
                            system: None,
                            template: None,
                            context: None,
                            images: None,
                            format: None,
                            keep_alive: None,
                        };

                        match tokio::time::timeout(
                            Duration::from_secs(config.timeout_seconds),
                            llamacpp.generate(req),
                        )
                        .await
                        {
                            Ok(Ok(res)) => {
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: true,
                                    error: None,
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });

                                return Ok(FallbackExecutionResult {
                                    content: res.response,
                                    target_used: target.id.clone(),
                                    model_used: res.model,
                                    was_fallback: target.id != first_target_id,
                                    attempts,
                                });
                            }
                            Ok(Err(e)) => {
                                warn!("Target 'llamacpp' failed: {}", e);
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: false,
                                    error: Some(e.to_string()),
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });
                            }
                            Err(_) => {
                                warn!("Target 'llamacpp' timed out");
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: false,
                                    error: Some("Timeout".to_string()),
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    }
                }

                cloud_id => {
                    // Check keyring for cloud API key
                    if let Ok(Some(api_key)) = KeyringStore::get_key(cloud_id) {
                        let client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(config.timeout_seconds))
                            .build()
                            .unwrap_or_default();

                        let res_content: Result<String> = match cloud_id {
                            "openai" => {
                                let payload = serde_json::json!({
                                    "model": target_model,
                                    "messages": [{"role": "user", "content": prompt}],
                                });
                                let resp = client
                                    .post("https://api.openai.com/v1/chat/completions")
                                    .header("Authorization", format!("Bearer {}", api_key))
                                    .json(&payload)
                                    .send()
                                    .await?;
                                if resp.status().is_success() {
                                    let json: serde_json::Value = resp.json().await?;
                                    Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
                                } else {
                                    Err(anyhow!("HTTP {}", resp.status()))
                                }
                            }
                            "deepseek" => {
                                let payload = serde_json::json!({
                                    "model": target_model,
                                    "messages": [{"role": "user", "content": prompt}],
                                });
                                let resp = client
                                    .post("https://api.deepseek.com/v1/chat/completions")
                                    .header("Authorization", format!("Bearer {}", api_key))
                                    .json(&payload)
                                    .send()
                                    .await?;
                                if resp.status().is_success() {
                                    let json: serde_json::Value = resp.json().await?;
                                    Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
                                } else {
                                    Err(anyhow!("HTTP {}", resp.status()))
                                }
                            }
                            "groq" => {
                                let payload = serde_json::json!({
                                    "model": target_model,
                                    "messages": [{"role": "user", "content": prompt}],
                                });
                                let resp = client
                                    .post("https://api.groq.com/openai/v1/chat/completions")
                                    .header("Authorization", format!("Bearer {}", api_key))
                                    .json(&payload)
                                    .send()
                                    .await?;
                                if resp.status().is_success() {
                                    let json: serde_json::Value = resp.json().await?;
                                    Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
                                } else {
                                    Err(anyhow!("HTTP {}", resp.status()))
                                }
                            }
                            "mistral" => {
                                let payload = serde_json::json!({
                                    "model": target_model,
                                    "messages": [{"role": "user", "content": prompt}],
                                });
                                let resp = client
                                    .post("https://api.mistral.ai/v1/chat/completions")
                                    .header("Authorization", format!("Bearer {}", api_key))
                                    .json(&payload)
                                    .send()
                                    .await?;
                                if resp.status().is_success() {
                                    let json: serde_json::Value = resp.json().await?;
                                    Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
                                } else {
                                    Err(anyhow!("HTTP {}", resp.status()))
                                }
                            }
                            "gemini" => {
                                let url = format!(
                                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                                    target_model, api_key
                                );
                                let payload = serde_json::json!({
                                    "contents": [{"parts": [{"text": prompt}]}],
                                });
                                let resp = client.post(&url).json(&payload).send().await?;
                                if resp.status().is_success() {
                                    let json: serde_json::Value = resp.json().await?;
                                    let text = json["candidates"][0]["content"]["parts"][0]["text"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    Ok(text)
                                } else {
                                    Err(anyhow!("HTTP {}", resp.status()))
                                }
                            }
                            _ => Err(anyhow!("Provider '{}' not supported", cloud_id)),
                        };

                        match res_content {
                            Ok(content) => {
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: true,
                                    error: None,
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });

                                return Ok(FallbackExecutionResult {
                                    content,
                                    target_used: target.id.clone(),
                                    model_used: target_model.to_string(),
                                    was_fallback: target.id != first_target_id,
                                    attempts,
                                });
                            }
                            Err(e) => {
                                warn!("Cloud target '{}' failed: {}", target.id, e);
                                attempts.push(TargetAttempt {
                                    target_id: target.id.clone(),
                                    success: false,
                                    error: Some(e.to_string()),
                                    duration_ms: attempt_start.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    } else {
                        debug!("Skipping cloud target '{}': No API key in OS keyring", cloud_id);
                        attempts.push(TargetAttempt {
                            target_id: target.id.clone(),
                            success: false,
                            error: Some("No API key configured".to_string()),
                            duration_ms: 0,
                        });
                    }
                }
            }
        }

        Err(anyhow!(
            "All targets in the Auto-Fallback chain failed. Attempts: {:?}",
            attempts
        ))
    }
}
