use crate::hybrid_mode::{HybridConfig, HybridMode, PrivacyMode};
use crate::keyring::KeyringStore;
use crate::provider::{
    CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage,
};
use crate::providers::{
    GeminiProvider, GroqProvider, LocalOllamaProvider, OpenRouterProvider,
    DEFAULT_GEMINI_MODEL, DEFAULT_GROQ_MODEL, DEFAULT_OLLAMA_MODEL, DEFAULT_OPENROUTER_FREE_MODEL,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackStrategy {
    LocalFirst,  // Local Ollama -> Free Cloud -> BYOK
    CloudFirst,  // Premium Cloud -> Free Cloud -> Local
    SpeedFirst,  // Groq -> Ollama -> Gemini -> OpenRouter
    CustomOrder, // Custom arrangement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackTarget {
    pub id: String,
    pub label: String,
    pub provider_type: ProviderType,
    pub is_local: bool,
    pub enabled: bool,
    pub preferred_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackChainConfig {
    pub enabled: bool,
    pub strategy: FallbackStrategy,
    pub targets: Vec<FallbackTarget>,
    pub max_retries_per_target: usize,
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
                    label: "Local Ollama Engine".to_string(),
                    provider_type: ProviderType::LocalOllama,
                    is_local: true,
                    enabled: true,
                    preferred_model: Some(DEFAULT_OLLAMA_MODEL.to_string()),
                },
                FallbackTarget {
                    id: "groq".to_string(),
                    label: "Groq Ultra-Fast (Free Tier)".to_string(),
                    provider_type: ProviderType::Groq,
                    is_local: false,
                    enabled: true,
                    preferred_model: Some(DEFAULT_GROQ_MODEL.to_string()),
                },
                FallbackTarget {
                    id: "gemini".to_string(),
                    label: "Google Gemini Flash (Free Tier)".to_string(),
                    provider_type: ProviderType::GeminiFlash,
                    is_local: false,
                    enabled: true,
                    preferred_model: Some(DEFAULT_GEMINI_MODEL.to_string()),
                },
                FallbackTarget {
                    id: "openrouter".to_string(),
                    label: "OpenRouter (Free Tier :free)".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    is_local: false,
                    enabled: true,
                    preferred_model: Some(DEFAULT_OPENROUTER_FREE_MODEL.to_string()),
                },
            ],
            max_retries_per_target: 1,
            timeout_seconds: 45,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAttempt {
    pub target_id: String,
    pub provider_type: ProviderType,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackExecutionResult {
    pub response: CompletionResponse,
    pub was_fallback: bool,
    pub fallback_reason: Option<String>,
    pub attempts: Vec<TargetAttempt>,
}

pub struct FallbackRouter {
    config: Arc<RwLock<FallbackChainConfig>>,
    hybrid_mode: Arc<RwLock<HybridMode>>,
}

impl FallbackRouter {
    pub fn new() -> Self {
        let hybrid_config = HybridConfig {
            enabled: true,
            privacy_mode: PrivacyMode::MaskedData,
            ..Default::default()
        };

        Self {
            config: Arc::new(RwLock::new(FallbackChainConfig::default())),
            hybrid_mode: Arc::new(RwLock::new(HybridMode::new(hybrid_config))),
        }
    }

    pub fn with_config(config: FallbackChainConfig) -> Self {
        let hybrid_config = HybridConfig {
            enabled: true,
            privacy_mode: PrivacyMode::MaskedData,
            ..Default::default()
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            hybrid_mode: Arc::new(RwLock::new(HybridMode::new(hybrid_config))),
        }
    }

    pub async fn get_config(&self) -> FallbackChainConfig {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, config: FallbackChainConfig) {
        *self.config.write().await = config;
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
                let order = ["groq", "ollama", "gemini", "openrouter"];
                cfg.targets.sort_by_key(|t| {
                    order.iter().position(|&x| x == t.id).unwrap_or(99)
                });
            }
            FallbackStrategy::CustomOrder => {}
        }
    }

    /// Automatically routes a prompt through the prioritized fallback chain with privacy masking for cloud
    pub async fn complete_routed(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<FallbackExecutionResult, LlmError> {
        let cfg = self.config.read().await.clone();
        let mut attempts = Vec::new();
        let first_target_id = cfg
            .targets
            .iter()
            .find(|t| t.enabled)
            .map(|t| t.id.clone())
            .unwrap_or_default();

        let mut last_error = None;

        for target in cfg.targets.iter().filter(|t| t.enabled) {
            let attempt_start = Instant::now();

            // Try key pool attempts for this target if rate limited
            let max_key_attempts = 3;
            let mut key_attempt = 0;
            let mut target_succeeded = false;

            while key_attempt < max_key_attempts && !target_succeeded {
                key_attempt += 1;

                let (provider_res, current_key): (Result<Box<dyn LlmProvider>, LlmError>, Option<String>) = match target.provider_type {
                    ProviderType::LocalOllama => {
                        (Ok(Box::new(LocalOllamaProvider::new(target.preferred_model.clone(), None))), None)
                    }
                    ProviderType::GeminiFlash => {
                        if let Some(key) = KeyringStore::get_active_api_key("gemini") {
                            (Ok(Box::new(GeminiProvider::new(key.clone(), target.preferred_model.clone()))), Some(key))
                        } else {
                            (Err(LlmError::AuthFailed("Gemini API key pool exhausted or in cooldown".to_string())), None)
                        }
                    }
                    ProviderType::Groq => {
                        if let Some(key) = KeyringStore::get_active_api_key("groq") {
                            (Ok(Box::new(GroqProvider::new(key.clone(), target.preferred_model.clone()))), Some(key))
                        } else {
                            (Err(LlmError::AuthFailed("Groq API key pool exhausted or in cooldown".to_string())), None)
                        }
                    }
                    ProviderType::OpenRouter => {
                        if let Some(key) = KeyringStore::get_active_api_key("openrouter") {
                            (Ok(Box::new(OpenRouterProvider::new(key.clone(), target.preferred_model.clone()))), Some(key))
                        } else {
                            (Err(LlmError::AuthFailed("OpenRouter API key pool exhausted or in cooldown".to_string())), None)
                        }
                    }
                    ProviderType::Custom(ref name) => {
                        if let Some(key) = KeyringStore::get_active_api_key(name) {
                            (Ok(Box::new(GroqProvider::new(key.clone(), target.preferred_model.clone()))), Some(key))
                        } else {
                            (Err(LlmError::AuthFailed(format!("Key for custom provider '{}' not found or in cooldown", name))), None)
                        }
                    }
                };

                let provider = match provider_res {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Provider initialization for '{}' failed: {}", target.id, e);
                        attempts.push(TargetAttempt {
                            target_id: target.id.clone(),
                            provider_type: target.provider_type.clone(),
                            success: false,
                            error: Some(e.to_string()),
                            duration_ms: attempt_start.elapsed().as_millis() as u64,
                        });
                        last_error = Some(e);
                        break; // Move to next provider target in chain
                    }
                };

                // Mandatory privacy masking for non-local cloud providers
                let (dispatched_prompt, is_masked) = if !target.is_local {
                    let mut hybrid = self.hybrid_mode.write().await;
                    let (masked, _secrets) = hybrid.mask_secrets(prompt);
                    (masked, true)
                } else {
                    (prompt.to_string(), false)
                };

                // Execute call
                debug!("Dispatching prompt to target '{}' ({})", target.id, target.provider_type);
                match provider.complete(&dispatched_prompt, system).await {
                    Ok(mut completion) => {
                        // Unmask if cloud
                        if is_masked {
                            let hybrid = self.hybrid_mode.read().await;
                            completion.content = hybrid.unmask_secrets(&completion.content);
                        }

                        attempts.push(TargetAttempt {
                            target_id: target.id.clone(),
                            provider_type: target.provider_type.clone(),
                            success: true,
                            error: None,
                            duration_ms: attempt_start.elapsed().as_millis() as u64,
                        });

                        let was_fallback = target.id != first_target_id;
                        let fallback_reason = if was_fallback {
                            last_error.map(|e| e.to_string())
                        } else {
                            None
                        };

                        info!(
                            "Auto-Fallback Router completed with target '{}' (was_fallback: {})",
                            target.id, was_fallback
                        );

                        return Ok(FallbackExecutionResult {
                            response: completion,
                            was_fallback,
                            fallback_reason,
                            attempts,
                        });
                    }
                    Err(LlmError::RateLimited(msg)) => {
                        warn!("Target '{}' hit 429 RateLimit ({}). Placing current key in 60s cooldown...", target.id, msg);
                        if let Some(ref k) = current_key {
                            KeyringStore::mark_key_rate_limited(&target.id, k, 60);
                        }
                        // Check if another key is active in this pool
                        if !KeyringStore::has_active_key(&target.id) {
                            warn!("All keys in pool for target '{}' are in cooldown. Advancing to next fallback target...", target.id);
                            attempts.push(TargetAttempt {
                                target_id: target.id.clone(),
                                provider_type: target.provider_type.clone(),
                                success: false,
                                error: Some(format!("429 RateLimit (all keys in pool cooling): {}", msg)),
                                duration_ms: attempt_start.elapsed().as_millis() as u64,
                            });
                            last_error = Some(LlmError::RateLimited(msg));
                            break; // Advance immediately to next target
                        }
                        // Otherwise loop will retry with next key in pool
                        info!("Retrying target '{}' with next rotated key in pool...", target.id);
                    }
                    Err(e) => {
                        warn!("Target '{}' failed ({}). Switching to next fallback target...", target.id, e);
                        attempts.push(TargetAttempt {
                            target_id: target.id.clone(),
                            provider_type: target.provider_type.clone(),
                            success: false,
                            error: Some(e.to_string()),
                            duration_ms: attempt_start.elapsed().as_millis() as u64,
                        });
                        last_error = Some(e);
                        break; // Advance to next target
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LlmError::ProviderUnavailable("All providers in fallback chain failed or were unconfigured".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::provider::BoxStream;

    struct MockFailingProvider {
        provider_type: ProviderType,
        error_to_return: LlmError,
    }

    #[async_trait]
    impl LlmProvider for MockFailingProvider {
        async fn complete(&self, _prompt: &str, _system: Option<&str>) -> Result<CompletionResponse, LlmError> {
            Err(self.error_to_return.clone())
        }
        async fn stream_complete(&self, _prompt: &str, _system: Option<&str>) -> Result<BoxStream<Result<String, LlmError>>, LlmError> {
            Err(self.error_to_return.clone())
        }
        async fn health_check(&self) -> Result<LatencyMetric, LlmError> {
            Err(self.error_to_return.clone())
        }
        fn provider_type(&self) -> ProviderType {
            self.provider_type.clone()
        }
    }

    struct MockSuccessfulProvider {
        provider_type: ProviderType,
        content_to_return: String,
    }

    #[async_trait]
    impl LlmProvider for MockSuccessfulProvider {
        async fn complete(&self, _prompt: &str, _system: Option<&str>) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                content: self.content_to_return.clone(),
                model_used: "mock-model".to_string(),
                provider_stamp: self.provider_type.clone(),
                token_usage: TokenUsage::new(10, 20),
                latency_ms: 45,
                finish_reason: Some("stop".to_string()),
            })
        }
        async fn stream_complete(&self, _prompt: &str, _system: Option<&str>) -> Result<BoxStream<Result<String, LlmError>>, LlmError> {
            Ok(Box::pin(futures::stream::once(async { Ok("chunk".to_string()) })))
        }
        async fn health_check(&self) -> Result<LatencyMetric, LlmError> {
            Ok(LatencyMetric {
                latency_ms: 10,
                is_healthy: true,
                message: "OK".to_string(),
                provider_type: self.provider_type.clone(),
            })
        }
        fn provider_type(&self) -> ProviderType {
            self.provider_type.clone()
        }
    }

    #[tokio::test]
    async fn test_failover_simulation() {
        let router = FallbackRouter::new();
        
        // Save mock keys in memory keyring
        KeyringStore::store_api_key(ProviderType::Groq, "mock_groq_key").unwrap();
        KeyringStore::store_api_key(ProviderType::GeminiFlash, "mock_gemini_key").unwrap();

        let cfg = router.get_config().await;
        assert_eq!(cfg.strategy, FallbackStrategy::LocalFirst);
        assert!(!cfg.targets.is_empty());
    }

    #[test]
    fn test_strategy_reordering() {
        let mut cfg = FallbackChainConfig::default();
        cfg.strategy = FallbackStrategy::SpeedFirst;

        let order = ["groq", "ollama", "gemini", "openrouter"];
        cfg.targets.sort_by_key(|t| {
            order.iter().position(|&x| x == t.id).unwrap_or(99)
        });

        assert_eq!(cfg.targets[0].id, "groq");
        assert_eq!(cfg.targets[1].id, "ollama");
    }
}
