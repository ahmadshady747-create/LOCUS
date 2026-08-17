use crate::state::AppState;
use locus_llm::{BackendType, GenerationOptions, LocalModel, Message, MessageRole, ModelSelection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub async fn llm_detect_models(state: State<'_, AppState>) -> Result<Vec<LocalModel>, String> {
    let llm = state.llm.read().await;
    llm.detect_available_models().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<i32>,
    pub num_ctx: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenerateResponse {
    pub response: String,
    pub model: String,
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_stamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_fallback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

#[tauri::command]
pub async fn llm_generate(
    state: State<'_, AppState>,
    request: GenerateRequest,
) -> Result<GenerateResponse, String> {
    let llm = state.llm.read().await;

    let model = request.model.clone().unwrap_or_else(|| "llama3".to_string());

    let options = GenerationOptions {
        temperature: request.temperature,
        top_p: request.top_p,
        num_predict: request.max_tokens,
        num_ctx: request.num_ctx,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let response = llm
        .generate_with_options(&request.prompt, Some(&model), options)
        .await
        .map_err(|e| e.to_string())?;
    let duration = start.elapsed().as_millis() as u64;

    let backend = llm
        .detect_available_models()
        .await
        .ok()
        .and_then(|models| {
            models
                .iter()
                .find(|m| m.name == model)
                .map(|m| format!("{:?}", m.backend))
        })
        .unwrap_or_else(|| "unknown".to_string());

    let provider_stamp = if model.contains("gemini") {
        Some("🤖 Google Gemini".to_string())
    } else if model.contains("groq") {
        Some("⚡ Groq Ultra-Fast".to_string())
    } else if model.contains("openrouter") {
        Some("🌐 OpenRouter Free".to_string())
    } else {
        Some(format!("🔒 Local ({})", model))
    };

    Ok(GenerateResponse {
        response,
        model,
        backend,
        provider_stamp,
        latency_ms: Some(duration),
        was_fallback: Some(false),
        fallback_reason: None,
    })
}

#[derive(Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageDto>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
}

#[tauri::command]
pub async fn llm_chat(
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<GenerateResponse, String> {
    let llm = state.llm.read().await;

    let messages: Vec<Message> = request
        .messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User,
            },
            content: m.content.clone(),
            images: None,
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    let model = request.model.clone().unwrap_or_else(|| "llama3".to_string());
    let start = std::time::Instant::now();
    let response = llm
        .chat(messages, Some(&model))
        .await
        .map_err(|e| e.to_string())?;
    let duration = start.elapsed().as_millis() as u64;

    let provider_stamp = if model.contains("gemini") {
        Some("🤖 Google Gemini".to_string())
    } else if model.contains("groq") || model.contains("llama-3.3") {
        Some("⚡ Groq (Llama 3.3)".to_string())
    } else if model.contains("deepseek") {
        Some("🧠 DeepSeek AI".to_string())
    } else if model.contains("openrouter") {
        Some("🌐 OpenRouter Free".to_string())
    } else {
        Some(format!("🔒 Local ({})", model))
    };

    Ok(GenerateResponse {
        response,
        model,
        backend: "local".to_string(),
        provider_stamp,
        latency_ms: Some(duration),
        was_fallback: Some(false),
        fallback_reason: None,
    })
}

#[tauri::command]
pub async fn llm_select_best_model(
    state: State<'_, AppState>,
    task_type: Option<String>,
) -> Result<ModelSelection, String> {
    let llm = state.llm.read().await;

    let specialization = match task_type.as_deref() {
        Some("codegen") => Some(locus_llm::Specialization::CodeGeneration),
        Some("review") => Some(locus_llm::Specialization::CodeReview),
        Some("test") => Some(locus_llm::Specialization::Testing),
        Some("embed") => Some(locus_llm::Specialization::Embeddings),
        _ => None,
    };

    llm.select_best_model(specialization, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_set_default_model(
    state: State<'_, AppState>,
    model: String,
    backend: String,
) -> Result<(), String> {
    let llm = state.llm.read().await;
    llm.set_default_model(model.clone());

    let backend_type = match backend.as_str() {
        "ollama" => BackendType::Ollama,
        "llamacpp" | "llama.cpp" => BackendType::LlamaCpp,
        _ => BackendType::Ollama,
    };
    llm.set_default_backend(backend_type);

    Ok(())
}

#[derive(Deserialize)]
pub struct HybridRequest {
    pub messages: Vec<ChatMessageDto>,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct HybridResponseDto {
    pub content: String,
    pub model_used: String,
    pub privacy_mode: String,
}

#[tauri::command]
pub async fn llm_hybrid_chat(
    state: State<'_, AppState>,
    request: HybridRequest,
) -> Result<HybridResponseDto, String> {
    *state.hybrid_enabled.write().await = request.enabled;

    let llm = state.llm.read().await;
    if !*state.hybrid_enabled.read().await {
        // Fall back to local chat when hybrid is disabled
        let messages: Vec<Message> = request
            .messages
            .iter()
            .map(|m| Message {
                role: if m.role == "user" {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content: m.content.clone(),
                images: None,
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();

        let response = llm.chat(messages, None).await.map_err(|e| e.to_string())?;
        return Ok(HybridResponseDto {
            content: response,
            model_used: "local".to_string(),
            privacy_mode: "full_local".to_string(),
        });
    }

    Err("Hybrid mode requires configuration. Set HYBRID_MODE=1 and provide API key.".to_string())
}

// === Simplified Tauri API ===

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    request: SendMessageRequest,
) -> Result<String, String> {
    let llm = state.llm.read().await;

    let messages = vec![Message {
        role: MessageRole::User,
        content: request.message,
        images: None,
        tool_calls: None,
        tool_call_id: None,
    }];

    let model = request.model.clone().unwrap_or_else(|| "llama3".to_string());
    let response = llm
        .chat(messages, Some(&model))
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

#[tauri::command]
pub async fn switch_model(
    state: State<'_, AppState>,
    model: String,
) -> Result<(), String> {
    let llm = state.llm.read().await;
    llm.set_default_model(model.clone());
    llm.set_default_backend(BackendType::Ollama);
    Ok(())
}

#[tauri::command]
pub async fn toggle_hybrid_mode(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    *state.hybrid_enabled.write().await = enabled;
    Ok(())
}

// === Keyring & Cloud Providers Commands ===

#[tauri::command]
pub async fn llm_save_api_key(
    provider: String,
    api_key: String,
) -> Result<(), String> {
    locus_llm::KeyringStore::save_key(&provider, &api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    locus_llm::KeyringStore::save_key(&provider, &api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_get_api_key_status() -> Result<Vec<locus_llm::ProviderStatus>, String> {
    Ok(locus_llm::KeyringStore::list_configured_providers())
}

#[tauri::command]
pub async fn get_configured_providers() -> Result<Vec<locus_llm::ProviderStatus>, String> {
    Ok(locus_llm::KeyringStore::list_configured_providers())
}

#[tauri::command]
pub async fn llm_delete_api_key(provider: String) -> Result<(), String> {
    locus_llm::KeyringStore::delete_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_api_key(provider: String) -> Result<(), String> {
    locus_llm::KeyringStore::delete_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn llm_test_api_key(
    provider: String,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<locus_llm::ProviderTestResult, String> {
    Ok(locus_llm::KeyringStore::test_provider(
        &provider,
        api_key.as_deref(),
        base_url.as_deref(),
    )
    .await)
}

#[tauri::command]
pub async fn test_provider_connection(
    provider: String,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<locus_llm::ProviderTestResult, String> {
    Ok(locus_llm::KeyringStore::test_provider(
        &provider,
        api_key.as_deref(),
        base_url.as_deref(),
    )
    .await)
}

// === Auto-Fallback Chain Router Commands ===

#[tauri::command]
pub async fn llm_get_fallback_chain(
    state: State<'_, AppState>,
) -> Result<locus_llm::FallbackChainConfig, String> {
    let llm = state.llm.read().await;
    Ok(llm.fallback_router().get_config().await)
}

#[tauri::command]
pub async fn llm_set_fallback_chain(
    state: State<'_, AppState>,
    config: locus_llm::FallbackChainConfig,
) -> Result<(), String> {
    let llm = state.llm.read().await;
    llm.fallback_router().update_config(config).await;
    Ok(())
}

#[tauri::command]
pub async fn llm_set_fallback_strategy(
    state: State<'_, AppState>,
    strategy: locus_llm::FallbackStrategy,
) -> Result<(), String> {
    let llm = state.llm.read().await;
    llm.fallback_router().set_strategy(strategy).await;
    Ok(())
}

#[tauri::command]
pub async fn llm_auto_detect_keys(
    state: State<'_, AppState>,
) -> Result<Vec<locus_llm::DetectedKeyReport>, String> {
    let ws_guard = state.workspace_root.read().await;
    let ws_dir = ws_guard.as_ref().and_then(|p| p.to_str());
    Ok(locus_llm::KeyringStore::auto_detect_and_import_keys(ws_dir))
}

#[tauri::command]
pub async fn auto_detect_api_keys(
    state: State<'_, AppState>,
) -> Result<Vec<locus_llm::DetectedKeyReport>, String> {
    llm_auto_detect_keys(state).await
}

#[tauri::command]
pub async fn llm_get_key_pool(provider: String) -> Result<Vec<locus_llm::keyring::KeySlotStatus>, String> {
    Ok(locus_llm::KeyringStore::get_key_pool_status(&provider))
}

#[tauri::command]
pub async fn llm_save_key_pool(provider: String, keys: String) -> Result<(), String> {
    locus_llm::KeyringStore::save_key(&provider, &keys).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Cognitive Router & Dynamic Cost-to-Power Routing IPC Commands
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RouteTaskRequest {
    pub prompt: String,
    pub file_count: Option<usize>,
    pub context_tokens: Option<usize>,
    pub strategy: Option<locus_llm::BudgetStrategy>,
}

#[tauri::command]
pub async fn cognitive_router_route(
    state: State<'_, AppState>,
    request: RouteTaskRequest,
) -> Result<locus_llm::RoutingDecision, String> {
    let complexity = locus_llm::CognitiveRouter::classify_prompt(
        &request.prompt,
        request.file_count.unwrap_or(1),
        request.context_tokens.unwrap_or(500),
    );

    let strategy = request
        .strategy
        .unwrap_or_else(locus_llm::CognitiveRouter::get_persisted_strategy);

    let llm = state.llm.read().await;
    let local_models = llm
        .detect_available_models()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.name)
        .collect::<Vec<_>>();

    let configured_providers = locus_llm::KeyringStore::list_configured_providers()
        .into_iter()
        .filter(|p| p.is_configured)
        .map(|p| p.provider_id)
        .collect::<Vec<_>>();

    Ok(locus_llm::CognitiveRouter::route(
        complexity,
        strategy,
        &configured_providers,
        &local_models,
    ))
}

#[tauri::command]
pub async fn cognitive_router_classify(
    prompt: String,
    file_count: Option<usize>,
    context_tokens: Option<usize>,
) -> Result<locus_llm::CognitiveTaskComplexity, String> {
    Ok(locus_llm::CognitiveRouter::classify_prompt(
        &prompt,
        file_count.unwrap_or(1),
        context_tokens.unwrap_or(500),
    ))
}

#[tauri::command]
pub async fn cognitive_router_get_strategy() -> Result<locus_llm::BudgetStrategy, String> {
    Ok(locus_llm::CognitiveRouter::get_persisted_strategy())
}

#[tauri::command]
pub async fn cognitive_router_set_strategy(
    strategy: locus_llm::BudgetStrategy,
) -> Result<(), String> {
    locus_llm::CognitiveRouter::save_persisted_strategy(strategy)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Local Discovery & Streaming Model Puller IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn local_discovery_probe_hardware() -> Result<locus_llm::HardwareProfile, String> {
    Ok(locus_llm::LocalDiscoveryManager::probe_hardware())
}

#[tauri::command]
pub async fn local_discovery_scan_endpoints() -> Result<Vec<locus_llm::LocalInferenceEndpoint>, String> {
    Ok(locus_llm::LocalDiscoveryManager::scan_endpoints().await)
}

#[tauri::command]
pub async fn local_discovery_get_report() -> Result<locus_llm::LocalDiscoveryReport, String> {
    Ok(locus_llm::LocalDiscoveryManager::generate_report().await)
}

#[tauri::command]
pub async fn model_puller_start_pull(
    model_name: String,
    endpoint_url: Option<String>,
) -> Result<String, String> {
    locus_llm::ModelPullerEngine::start_pull(model_name, endpoint_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn model_puller_get_progress(
    job_id: String,
) -> Result<Option<locus_llm::ModelPullProgress>, String> {
    Ok(locus_llm::ModelPullerEngine::get_progress(&job_id).await)
}

#[tauri::command]
pub async fn model_puller_cancel_pull(job_id: String) -> Result<(), String> {
    locus_llm::ModelPullerEngine::cancel_pull(&job_id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Free Provider Radar & Quota Intelligence IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn free_provider_radar_get_suggestions() -> Result<Vec<locus_llm::FreeProviderSuggestion>, String> {
    let configured_providers = locus_llm::KeyringStore::list_configured_providers()
        .into_iter()
        .filter(|p| p.is_configured)
        .map(|p| p.provider_id)
        .collect::<Vec<_>>();

    Ok(locus_llm::FreeProviderRadar::get_active_suggestions(&configured_providers))
}

#[tauri::command]
pub async fn free_provider_radar_dismiss(provider_id: String) -> Result<(), String> {
    locus_llm::FreeProviderRadar::dismiss_provider(&provider_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn free_provider_radar_save_and_activate(
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    locus_llm::KeyringStore::save_key(&provider_id, &api_key)
        .map_err(|e| e.to_string())
}




