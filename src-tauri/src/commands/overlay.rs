//! Tauri IPC commands for Ambient Overlay and Spotlight window control.

use locus_core::{AmbientController, AmbientTelemetry};
use once_cell::sync::Lazy;
use tauri::{AppHandle, Manager};

static GLOBAL_AMBIENT_CONTROLLER: Lazy<AmbientController> = Lazy::new(AmbientController::new);

/// Get a reference to the global ambient controller.
pub fn get_global_ambient_controller() -> &'static AmbientController {
    &GLOBAL_AMBIENT_CONTROLLER
}

#[tauri::command]
pub async fn toggle_spotlight(app: AppHandle) -> Result<bool, String> {
    let controller = get_global_ambient_controller();

    if let Some(window) = app.get_webview_window("spotlight") {
        let is_vis = window.is_visible().map_err(|e| e.to_string())?;
        if is_vis {
            window.hide().map_err(|e| e.to_string())?;
            controller.dismiss();
            Ok(false)
        } else {
            let _ = controller.trigger_wake();
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            window.set_always_on_top(true).map_err(|e| e.to_string())?;
            Ok(true)
        }
    } else {
        Err("Spotlight window not found".to_string())
    }
}

#[tauri::command]
pub fn get_ambient_telemetry() -> Result<AmbientTelemetry, String> {
    let controller = get_global_ambient_controller();
    Ok(controller.get_telemetry())
}

#[tauri::command]
pub fn ambient_controller_dismiss() -> Result<bool, String> {
    let controller = get_global_ambient_controller();
    controller.dismiss();
    Ok(true)
}

#[tauri::command]
pub fn parse_omnibar_input(
    input: String,
    clipboard: Option<String>,
) -> Result<locus_core::OmniIntent, String> {
    Ok(locus_core::OmniIntent::parse(&input, clipboard))
}

#[tauri::command]
pub async fn query_omni_search(
    query: String,
    root_path: Option<String>,
) -> Result<Vec<locus_context::OmniSearchResult>, String> {
    let root = match root_path {
        Some(ref p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };

    Ok(locus_context::OmniSearchEngine::search_local(&query, &root, 20))
}

static GLOBAL_CHAT_INDEX: Lazy<std::sync::RwLock<locus_context::ChatMemoryIndex>> =
    Lazy::new(|| std::sync::RwLock::new(locus_context::ChatMemoryIndex::new()));

#[tauri::command]
pub async fn search_chat_memory(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<locus_context::ChatMemoryMatch>, String> {
    let index = GLOBAL_CHAT_INDEX
        .read()
        .map_err(|e| format!("Failed to acquire chat index lock: {}", e))?;
    Ok(index.search(&query, limit.unwrap_or(10)))
}

#[tauri::command]
pub async fn inject_text_to_active(
    text: String,
    restore_clipboard: Option<bool>,
) -> Result<locus_core::InjectionReport, String> {
    let report = locus_core::SafeTextInjector::inject_text(&text, restore_clipboard.unwrap_or(true));
    Ok(report)
}

#[tauri::command]
pub async fn execute_ambient_agent(
    prompt: String,
    target_code: Option<String>,
) -> Result<locus_agents::AmbientActionResult, String> {
    locus_agents::AmbientAgentEngine::execute_ambient_action(&prompt, target_code.as_deref()).await
}

#[tauri::command]
pub fn run_quick_formal_verify(
    target: String,
    code_context: Option<String>,
) -> Result<locus_core::QuickVerifyReport, String> {
    Ok(locus_core::QuickVerifierBridge::verify_expression_or_function(&target, code_context.as_deref()))
}



