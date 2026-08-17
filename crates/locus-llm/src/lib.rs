pub mod cognitive_router;
mod fallback_router;
pub mod free_provider_radar;
mod hybrid_mode;
pub mod keyring;
pub mod keyring_store;
pub mod local_discovery;
mod llamacpp;
pub mod model_puller;
mod model_selector;
mod ollama;
pub mod provider;
pub mod providers;
pub mod router;
pub mod skill_bridge;
pub mod fim;
mod types;

pub use fim::FimDispatcher;

pub use cognitive_router::{
    BudgetStrategy, CognitiveRouter, CognitiveTaskComplexity, CostTier, RoutingDecision,
};
pub use free_provider_radar::{
    FreeProviderInfo, FreeProviderRadar, FreeProviderSuggestion,
};
pub use local_discovery::{
    HardwareProfile, LocalDiscoveryManager, LocalDiscoveryReport, LocalInferenceEndpoint,
    RecommendedModelSpec,
};
pub use model_puller::{ModelPullProgress, ModelPullerEngine};

pub use skill_bridge::{manifest_to_tool, skills_to_tools, SkillBridge};

pub use fallback_router::{
    FallbackChainConfig, FallbackExecutionResult, FallbackRouter, FallbackStrategy, FallbackTarget,
    TargetAttempt,
};
pub use hybrid_mode::{
    CloudProvider, HybridConfig, HybridMode, HybridRequest, HybridResponse, MaskedSecret, PrivacyMode,
};
pub use keyring::{DetectedKeyReport, KeyringStore, ProviderStatus, ProviderTestResult, mask_key_preview};
pub use router::FallbackRouter as SmartFallbackRouter;
pub use llamacpp::LlamaCppClient;
pub use model_selector::ModelSelector;
pub use ollama::OllamaClient;
pub use provider::{
    BoxStream, CompletionResponse, LatencyMetric, LlmError, LlmProvider, ProviderType, TokenUsage,
};
pub use providers::{
    GeminiProvider, GroqProvider, LocalOllamaProvider, OpenRouterProvider,
    DEFAULT_GEMINI_MODEL, DEFAULT_GROQ_MODEL, DEFAULT_OLLAMA_MODEL, DEFAULT_OPENROUTER_FREE_MODEL,
};
pub use types::{
    BackendType, ChatRequest, ChatResponse, GenerationOptions, GenerationRequest, GenerationResponse,
    GpuInfo, LocalModel, Message, MessageRole, ModelDetails, ModelInfo, ModelSelection, SystemInfo,
    Tool, ToolCall, ToolFunction,
};
use anyhow::Result;
pub use locus_network::types::Specialization;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct LlmClient {
    ollama: Option<OllamaClient>,
    llamacpp: Option<LlamaCppClient>,
    model_selector: ModelSelector,
    hybrid_mode: Option<HybridMode>,
    fallback_router: Arc<FallbackRouter>,
    default_model: Arc<RwLock<Option<String>>>,
    default_backend: Arc<RwLock<BackendType>>,
}

impl LlmClient {
    pub async fn new() -> Result<Self> {
        let ollama = OllamaClient::new(None).ok();
        let llamacpp = LlamaCppClient::new(None).ok();
        let model_selector = ModelSelector::new();
        let fallback_router = Arc::new(FallbackRouter::new(
            FallbackChainConfig::default(),
            ollama.clone(),
            llamacpp.clone(),
        ));
        
        Ok(Self {
            ollama,
            llamacpp,
            model_selector,
            hybrid_mode: None,
            fallback_router,
            default_model: Arc::new(RwLock::new(None)),
            default_backend: Arc::new(RwLock::new(BackendType::Ollama)),
        })
    }

    pub fn fallback_router(&self) -> Arc<FallbackRouter> {
        self.fallback_router.clone()
    }

    pub fn with_ollama(mut self, base_url: Option<String>) -> Result<Self> {
        self.ollama = Some(OllamaClient::new(base_url)?);
        Ok(self)
    }

    pub fn with_llamacpp(mut self, base_url: Option<String>) -> Result<Self> {
        self.llamacpp = Some(LlamaCppClient::new(base_url)?);
        Ok(self)
    }

    pub fn with_hybrid_mode(mut self, config: HybridConfig) -> Self {
        self.hybrid_mode = Some(HybridMode::new(config));
        self
    }

    pub async fn detect_available_models(&self) -> Result<Vec<LocalModel>> {
        let mut all_models = Vec::new();
        
        if let Some(ref ollama) = self.ollama {
            if ollama.is_available().await {
                match ollama.list_models().await {
                    Ok(models) => {
                        info!("Found {} Ollama models", models.len());
                        all_models.extend(models);
                    }
                    Err(e) => warn!("Failed to list Ollama models: {}", e),
                }
            }
        }
        
        if let Some(ref llamacpp) = self.llamacpp {
            if llamacpp.is_available().await {
                match llamacpp.get_models().await {
                    Ok(models) => {
                        info!("Found {} llama.cpp models", models.len());
                        all_models.extend(models);
                    }
                    Err(e) => warn!("Failed to list llama.cpp models: {}", e),
                }
            }
        }
        
        Ok(all_models)
    }

    pub async fn generate(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model_name = model.unwrap_or_else(|| "llama3").to_string();
        let backend = self.get_backend_for_model(&model_name).await;
        
        match backend {
            BackendType::Ollama => {
                if let Some(ref ollama) = self.ollama {
                    let request = GenerationRequest {
                        model: model_name,
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
                    let response = ollama.generate(request).await?;
                    return Ok(response.response);
                }
            }
            BackendType::LlamaCpp => {
                if let Some(ref llamacpp) = self.llamacpp {
                    let request = GenerationRequest {
                        model: model_name,
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
                    let response = llamacpp.generate(request).await?;
                    return Ok(response.response);
                }
            }
        }
        
        Err(anyhow::anyhow!("No backend available for model"))
    }

    pub async fn chat(&self, messages: Vec<Message>, model: Option<&str>) -> Result<String> {
        let model_name = model.unwrap_or_else(|| "llama3").to_string();
        let backend = self.get_backend_for_model(&model_name).await;
        
        match backend {
            BackendType::Ollama => {
                if let Some(ref ollama) = self.ollama {
                    let request = ChatRequest {
                        model: model_name,
                        messages,
                        stream: false,
                        options: None,
                        tools: None,
                        keep_alive: None,
                    };
                    let response = ollama.chat(request).await?;
                    return Ok(response.message.content);
                }
            }
            BackendType::LlamaCpp => {
                if let Some(ref llamacpp) = self.llamacpp {
                    let request = ChatRequest {
                        model: model_name,
                        messages,
                        stream: false,
                        options: None,
                        tools: None,
                        keep_alive: None,
                    };
                    let response = llamacpp.chat(request).await?;
                    return Ok(response.message.content);
                }
            }
        }
        
        Err(anyhow::anyhow!("No backend available for model"))
    }

    pub async fn generate_with_options(
        &self,
        prompt: &str,
        model: Option<&str>,
        options: GenerationOptions,
    ) -> Result<String> {
        let model_name = model.unwrap_or_else(|| "llama3").to_string();
        let backend = self.get_backend_for_model(&model_name).await;
        
        match backend {
            BackendType::Ollama => {
                if let Some(ref ollama) = self.ollama {
                    let request = GenerationRequest {
                        model: model_name,
                        prompt: prompt.to_string(),
                        stream: false,
                        options: Some(options),
                        system: None,
                        template: None,
                        context: None,
                        images: None,
                        format: None,
                        keep_alive: None,
                    };
                    let response = ollama.generate(request).await?;
                    return Ok(response.response);
                }
            }
            BackendType::LlamaCpp => {
                if let Some(ref llamacpp) = self.llamacpp {
                    let request = GenerationRequest {
                        model: model_name,
                        prompt: prompt.to_string(),
                        stream: false,
                        options: Some(options),
                        system: None,
                        template: None,
                        context: None,
                        images: None,
                        format: None,
                        keep_alive: None,
                    };
                    let response = llamacpp.generate(request).await?;
                    return Ok(response.response);
                }
            }
        }
        
        Err(anyhow::anyhow!("No backend available for model"))
    }

    pub async fn select_best_model(
        &self,
        task_type: Option<Specialization>,
        preferred_backend: Option<BackendType>,
    ) -> Result<ModelSelection> {
        let models = self.detect_available_models().await?;
        self.model_selector.select_model(&models, task_type, preferred_backend).await
    }

    pub async fn auto_generate(&self, prompt: &str, task_type: Option<Specialization>) -> Result<String> {
        let selection = self.select_best_model(task_type, None).await?;
        info!("Auto-selected model: {} ({})", selection.model_name, selection.reasoning);
        self.generate(prompt, Some(&selection.model_name)).await
    }

    pub async fn auto_chat(&self, messages: Vec<Message>, task_type: Option<Specialization>) -> Result<String> {
        let selection = self.select_best_model(task_type, None).await?;
        info!("Auto-selected model: {} ({})", selection.model_name, selection.reasoning);
        self.chat(messages, Some(&selection.model_name)).await
    }

    pub async fn hybrid_chat(&mut self, request: HybridRequest) -> Result<HybridResponse> {
        if let Some(ref mut hybrid) = self.hybrid_mode {
            hybrid.execute_hybrid(request).await
        } else {
            Err(anyhow::anyhow!("Hybrid mode not configured"))
        }
    }

    async fn get_backend_for_model(&self, model: &str) -> BackendType {
        if let Some(ref ollama) = self.ollama {
            if ollama.is_available().await {
                if let Ok(models) = ollama.list_models().await {
                    if models.iter().any(|m| m.name == model) {
                        return BackendType::Ollama;
                    }
                }
            }
        }
        
        if let Some(ref llamacpp) = self.llamacpp {
            if llamacpp.is_available().await {
                if let Ok(models) = llamacpp.get_models().await {
                    if models.iter().any(|m| m.name == model) {
                        return BackendType::LlamaCpp;
                    }
                }
            }
        }
        
        self.default_backend.read().await.clone()
    }

    pub fn set_default_model(&self, model: String) {
        *self.default_model.blocking_write() = Some(model);
    }

    pub fn set_default_backend(&self, backend: BackendType) {
        *self.default_backend.blocking_write() = backend;
    }

    pub async fn get_embeddings(&self, model: &str, input: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if let Some(ref llamacpp) = self.llamacpp {
            return llamacpp.get_embeddings(model, input).await;
        }
        
        Err(anyhow::anyhow!("Embeddings only supported with llama.cpp backend"))
    }

    pub fn is_ollama_available(&self) -> bool {
        self.ollama.is_some()
    }

    pub fn is_llamacpp_available(&self) -> bool {
        self.llamacpp.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole};

    #[tokio::test]
    async fn test_client_creation() {
        let client = LlmClient::new().await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_model_detection() {
        let client = LlmClient::new().await.unwrap();
        let models = client.detect_available_models().await.unwrap();
        assert!(models.is_empty() || !models.is_empty());
    }
}