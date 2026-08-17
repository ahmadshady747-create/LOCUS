//! Tauri IPC commands for Sovereign Ergonomics Suite.

use locus_context::{resolve_mention_query, MentionCandidate};
use locus_core::{process_terminal_failure, types::DiffHunk, TerminalFailureReport};
use locus_fs::{apply_selected_hunks, parse_diff_into_hunks, PatchResult};
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn fs_parse_diff_hunks(original: String, modified: String) -> Result<Vec<DiffHunk>, String> {
    Ok(parse_diff_into_hunks(&original, &modified))
}

#[tauri::command]
pub fn fs_apply_selected_hunks(
    file_path: String,
    original_content: String,
    hunks: Vec<DiffHunk>,
    selected_ids: Vec<String>,
) -> Result<PatchResult, String> {
    let p = Path::new(&file_path);
    let ext = p.extension().and_then(|e| e.to_str());

    apply_selected_hunks(&original_content, &hunks, &selected_ids, ext)
}

#[tauri::command]
pub fn terminal_process_failure(
    command: String,
    exit_code: i32,
    stderr: String,
) -> Result<TerminalFailureReport, String> {
    Ok(process_terminal_failure(&command, exit_code, &stderr))
}

#[tauri::command]
pub fn context_query_mentions(
    query: String,
    workspace_root: Option<String>,
    filter_type: Option<String>,
) -> Result<Vec<MentionCandidate>, String> {
    let root = match workspace_root {
        Some(r) => PathBuf::from(r),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    Ok(resolve_mention_query(
        &query,
        &root,
        filter_type.as_deref(),
    ))
}
