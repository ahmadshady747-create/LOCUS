use crate::state::AppState;
use locus_core::types::{FileContent, FileEventKind, ModificationOp, WorkspaceIndex};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Serialize)]
pub struct ScanResult {
    pub index: WorkspaceIndex,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn fs_scan(state: State<'_, AppState>, root: Option<PathBuf>) -> Result<ScanResult, String> {
    let start = std::time::Instant::now();
    let fs = state.fs_engine.read().await;

    let resolved_root = match root {
        Some(r) => r,
        None => {
            let guard = state.workspace_root.read().await;
            guard.clone().unwrap_or_else(|| PathBuf::from("."))
        }
    };

    let index = fs.scan_workspace().await.map_err(|e| e.to_string())?;
    *state.workspace_root.write().await = Some(resolved_root);

    Ok(ScanResult {
        index,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn fs_read_file(state: State<'_, AppState>, path: PathBuf) -> Result<FileContent, String> {
    let fs = state.fs_engine.read().await;
    fs.read_file(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_write_file(
    state: State<'_, AppState>,
    path: PathBuf,
    content: String,
) -> Result<(), String> {
    let fs = state.fs_engine.read().await;
    fs.write_file(&path, &content).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_modify_file(
    state: State<'_, AppState>,
    path: PathBuf,
    ops: Vec<ModificationOp>,
) -> Result<(), String> {
    let fs = state.fs_engine.read().await;
    fs.modify_file(&path, &ops).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<locus_core::types::SearchResult>, String> {
    let fs = state.fs_engine.read().await;
    fs.search(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_watch_start(state: State<'_, AppState>) -> Result<(), String> {
    let root = state
        .workspace_root
        .read()
        .await
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let fs = state.fs_engine.read().await;
    let watcher = fs.watch(&[root]).map_err(|e| e.to_string())?;
    
    // Store the watcher so it doesn't get dropped
    *state.file_watcher.write().await = Some(watcher);
    
    Ok(())
}

#[tauri::command]
pub async fn fs_get_index(state: State<'_, AppState>) -> Result<WorkspaceIndex, String> {
    let fs = state.fs_engine.read().await;
    Ok(fs.get_index().await)
}

#[derive(Serialize)]
pub struct FileEventDto {
    pub path: String,
    pub kind: String,
    pub timestamp: String,
}

impl From<locus_core::types::FileEvent> for FileEventDto {
    fn from(e: locus_core::types::FileEvent) -> Self {
        let kind = match e.kind {
            FileEventKind::Created => "created".to_string(),
            FileEventKind::Modified => "modified".to_string(),
            FileEventKind::Deleted => "deleted".to_string(),
            FileEventKind::Renamed { from, to } => {
                format!("renamed {} -> {}", from.display(), to.display())
            }
        };
        Self {
            path: e.path.display().to_string(),
            kind,
            timestamp: e.timestamp.to_rfc3339(),
        }
    }
}

#[tauri::command]
pub async fn fs_stage_change(
    state: State<'_, AppState>,
    path: PathBuf,
    proposed_content: String,
) -> Result<locus_core::types::StagedFileChange, String> {
    let fs = state.fs_engine.read().await;
    fs.stage_change(&path, &proposed_content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_accept_change(
    state: State<'_, AppState>,
    change_id: String,
) -> Result<(), String> {
    let fs = state.fs_engine.read().await;
    fs.accept_change(&change_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_reject_change(
    state: State<'_, AppState>,
    change_id: String,
) -> Result<(), String> {
    let fs = state.fs_engine.read().await;
    fs.reject_change(&change_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_list_staged_changes(
    state: State<'_, AppState>,
) -> Result<Vec<locus_core::types::StagedFileChange>, String> {
    let fs = state.fs_engine.read().await;
    Ok(fs.list_staged_changes().await)
}

#[tauri::command]
pub async fn fs_compute_hunks(
    state: State<'_, AppState>,
    original: String,
    proposed: String,
) -> Result<Vec<locus_core::types::DiffHunk>, String> {
    let fs = state.fs_engine.read().await;
    Ok(fs.compute_hunks(&original, &proposed))
}

#[tauri::command]
pub async fn fs_accept_hunk(
    state: State<'_, AppState>,
    change_id: String,
    hunk_id: String,
) -> Result<Option<locus_core::types::StagedFileChange>, String> {
    let fs = state.fs_engine.read().await;
    fs.accept_hunk(&change_id, &hunk_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_reject_hunk(
    state: State<'_, AppState>,
    change_id: String,
    hunk_id: String,
) -> Result<Option<locus_core::types::StagedFileChange>, String> {
    let fs = state.fs_engine.read().await;
    fs.reject_hunk(&change_id, &hunk_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_rollback_last(
    state: State<'_, AppState>,
) -> Result<locus_core::types::RollbackResult, String> {
    let fs = state.fs_engine.read().await;
    fs.rollback_last().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fs_list_snapshots(
    state: State<'_, AppState>,
) -> Result<Vec<locus_core::types::FileSnapshot>, String> {
    let fs = state.fs_engine.read().await;
    Ok(fs.list_snapshots().await)
}

#[derive(serde::Serialize)]
pub struct ApplySearchReplaceResultDto {
    pub success: bool,
    pub applied_blocks_count: usize,
    pub new_content: String,
}

#[tauri::command]
pub async fn fs_apply_search_replace(
    state: State<'_, AppState>,
    path: PathBuf,
    content: String,
) -> Result<ApplySearchReplaceResultDto, String> {
    let fs = state.fs_engine.read().await;
    let (new_content, count) = fs
        .apply_search_replace(&path, &content)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ApplySearchReplaceResultDto {
        success: true,
        applied_blocks_count: count,
        new_content,
    })
}

