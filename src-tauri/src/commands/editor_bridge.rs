//! Tauri IPC commands for the Universal Silent Editor Bridge.

use locus_fs::{DetectedEditor, EditorBridgeEngine, EditorBridgeStatus, EditorSyncReport};
use once_cell::sync::Lazy;
use std::path::Path;

static EDITOR_BRIDGE: Lazy<EditorBridgeEngine> = Lazy::new(EditorBridgeEngine::new);

#[tauri::command]
pub fn editor_bridge_status() -> Result<EditorBridgeStatus, String> {
    Ok(EDITOR_BRIDGE.get_status())
}

#[tauri::command]
pub fn editor_bridge_sync_file(path: String, content: String) -> Result<EditorSyncReport, String> {
    EditorBridgeEngine::atomic_write_file(Path::new(&path), &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn editor_bridge_open_in_editor(
    path: String,
    line: Option<usize>,
    column: Option<usize>,
    preferred_editor: Option<String>,
) -> Result<bool, String> {
    EditorBridgeEngine::open_in_editor(
        Path::new(&path),
        line,
        column,
        preferred_editor.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn editor_bridge_detect_editors() -> Result<Vec<DetectedEditor>, String> {
    Ok(EditorBridgeEngine::detect_installed_editors())
}
