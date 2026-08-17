//! Tauri IPC commands for Ambient OS Context Engine.

use locus_core::{AmbientEngine, AmbientSnapshot};

#[tauri::command]
pub fn ambient_get_snapshot(selected_text: Option<String>) -> Result<AmbientSnapshot, String> {
    Ok(AmbientEngine::get_ambient_snapshot(selected_text))
}

#[tauri::command]
pub fn ambient_paste_to_active(_text: String) -> Result<bool, String> {
    // Helper to signal UI copy / injection readiness
    Ok(true)
}
