//! Tauri IPC commands for Zero-Shot Micro-SAST Security Gate.

use locus_agents::{SecurityGate, SecurityScanResult};

#[tauri::command]
pub fn security_scan_snippet(
    code_snippet: String,
    language: Option<String>,
) -> Result<SecurityScanResult, String> {
    Ok(SecurityGate::validate_snippet(
        &code_snippet,
        language.as_deref(),
    ))
}

#[tauri::command]
pub fn security_scan_diff(diff: String) -> Result<SecurityScanResult, String> {
    Ok(SecurityGate::validate_diff(&diff))
}
