//! Tauri IPC commands for Local Tools Runner and Circuit Breaker.

use locus_plugins::{
    discover_local_tools, CircuitState, LocalToolManifest, LocalToolRunner, ToolExecutionOutput,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;

static TOOL_RUNNER: Lazy<LocalToolRunner> = Lazy::new(LocalToolRunner::new);

#[tauri::command]
pub fn plugins_list_local_tools(workspace_path: Option<String>) -> Result<Vec<LocalToolManifest>, String> {
    let ws_path = workspace_path.map(PathBuf::from);
    Ok(discover_local_tools(ws_path.as_deref()))
}

#[tauri::command]
pub async fn plugins_run_local_tool(
    tool_id: String,
    args: Vec<String>,
    workspace_path: Option<String>,
) -> Result<ToolExecutionOutput, String> {
    let ws_path = workspace_path.map(PathBuf::from);
    let tools = discover_local_tools(ws_path.as_deref());

    let target_tool = tools
        .into_iter()
        .find(|t| t.id == tool_id || t.name.to_lowercase() == tool_id.to_lowercase())
        .ok_or_else(|| format!("Local tool '{}' not found", tool_id))?;

    TOOL_RUNNER
        .execute_tool_safe(&target_tool, &args, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugins_get_circuit_status() -> Result<HashMap<String, CircuitState>, String> {
    Ok(TOOL_RUNNER.circuit_breaker.get_status())
}

#[tauri::command]
pub fn plugins_reset_circuit(tool_id: String) -> Result<bool, String> {
    Ok(TOOL_RUNNER.circuit_breaker.reset(&tool_id))
}
