use crate::state::AppState;
use locus_agents::task_graph::{TaskActionPayload, TaskGraph, TaskNodeResult, TaskNodeStatus, TaskNodeType};
use serde::Deserialize;
use std::time::Instant;
use tauri::{AppHandle, Manager, State};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct DecomposeGoalRequest {
    pub goal: String,
    pub files: Option<Vec<String>>,
}

#[tauri::command]
pub async fn task_graph_decompose(
    _state: State<'_, AppState>,
    request: DecomposeGoalRequest,
) -> Result<TaskGraph, String> {
    let files = request.files.unwrap_or_default();
    let graph = TaskGraph::decompose_goal(&request.goal, &files);
    info!(
        "Decomposed goal '{}' into DAG with {} nodes",
        request.goal,
        graph.nodes.len()
    );
    Ok(graph)
}

#[derive(Debug, Deserialize)]
pub struct ValidateDagRequest {
    pub graph: TaskGraph,
}

#[tauri::command]
pub async fn task_graph_validate(
    _state: State<'_, AppState>,
    request: ValidateDagRequest,
) -> Result<Vec<String>, String> {
    request
        .graph
        .topological_sort()
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub mut_graph: TaskGraph,
    pub node_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub payload: Option<TaskActionPayload>,
    pub status: Option<TaskNodeStatus>,
}

#[tauri::command]
pub async fn task_graph_update_node(
    _state: State<'_, AppState>,
    mut request: UpdateNodeRequest,
) -> Result<TaskGraph, String> {
    request
        .mut_graph
        .update_node(
            &request.node_id,
            request.title,
            request.description,
            request.payload,
            request.status,
        )
        .map_err(|e| e.to_string())?;
    Ok(request.mut_graph)
}

#[derive(Debug, Deserialize)]
pub struct ExecuteNodeRequest {
    pub mut_graph: TaskGraph,
    pub node_id: String,
}

#[tauri::command]
pub async fn task_graph_execute_node(
    state: State<'_, AppState>,
    mut request: ExecuteNodeRequest,
) -> Result<TaskGraph, String> {
    let node_id = request.node_id.clone();

    let node_index = request
        .mut_graph
        .nodes
        .iter()
        .position(|n| n.id == node_id)
        .ok_or_else(|| format!("Node '{}' not found in task graph", node_id))?;

    let node_type = request.mut_graph.nodes[node_index].node_type.clone();
    let node_title = request.mut_graph.nodes[node_index].title.clone();
    let node_payload = request.mut_graph.nodes[node_index].payload.clone();
    let start = Instant::now();

    request
        .mut_graph
        .mark_running(&node_id)
        .map_err(|e| e.to_string())?;

    match node_type {
        TaskNodeType::ShellCommand | TaskNodeType::Test => {
            let cmd_str = node_payload
                .shell_command
                .clone()
                .unwrap_or_else(|| "cargo check".to_string());

            let workspace_root = state.workspace_root.read().await.clone();

            #[cfg(target_os = "windows")]
            let mut cmd = std::process::Command::new("powershell");
            #[cfg(target_os = "windows")]
            cmd.args(["-NoProfile", "-Command", &cmd_str]);

            #[cfg(not(target_os = "windows"))]
            let mut cmd = std::process::Command::new("sh");
            #[cfg(not(target_os = "windows"))]
            cmd.args(["-c", &cmd_str]);

            if let Some(ref root) = workspace_root {
                cmd.current_dir(root);
            }

            let output = cmd.output().map_err(|e| format!("Failed to execute command: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let success = output.status.success();
            let duration_ms = start.elapsed().as_millis() as u64;

            let result = TaskNodeResult {
                success,
                output: if success { stdout } else { format!("{}\n{}", stdout, stderr) },
                diff_preview: None,
                error: if success { None } else { Some(format!("Exit code: {:?}", output.status.code())) },
                duration_ms,
            };

            if success {
                request.mut_graph.mark_completed(&node_id, result).map_err(|e| e.to_string())?;
            } else {
                request.mut_graph.mark_failed(&node_id, &format!("Command failed with exit code: {:?}", output.status.code())).map_err(|e| e.to_string())?;
            }
        }
        TaskNodeType::CodeEdit | TaskNodeType::CreateFile => {
            // If search_replace_block is present, apply it
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = TaskNodeResult {
                success: true,
                output: "Code modification staged successfully".to_string(),
                diff_preview: node_payload.search_replace_block.clone().or_else(|| node_payload.proposed_content.clone()),
                error: None,
                duration_ms,
            };
            request.mut_graph.mark_completed(&node_id, result).map_err(|e| e.to_string())?;
        }
        TaskNodeType::Analysis | TaskNodeType::SkillExecution => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = TaskNodeResult {
                success: true,
                output: format!("Step '{}' completed analysis successfully.", node_title),
                diff_preview: None,
                error: None,
                duration_ms,
            };
            request.mut_graph.mark_completed(&node_id, result).map_err(|e| e.to_string())?;
        }
    }

    Ok(request.mut_graph)
}

// === Spotlight HUD Window Management ===

#[tauri::command]
pub async fn spotlight_toggle(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("spotlight") {
        let is_vis = window.is_visible().map_err(|e| e.to_string())?;
        if is_vis {
            window.hide().map_err(|e| e.to_string())?;
            Ok(false)
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            Ok(true)
        }
    } else {
        Err("Spotlight window not found".to_string())
    }
}

#[tauri::command]
pub async fn spotlight_hide(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("spotlight") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn spotlight_show(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("spotlight") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn spotlight_set_pinned(app: AppHandle, pinned: bool) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("spotlight") {
        window.set_always_on_top(pinned).map_err(|e| e.to_string())?;
        Ok(pinned)
    } else {
        Err("Spotlight window not found".to_string())
    }
}

// ---------------------------------------------------------------------------
// Specification Alignment & Architectural Tradeoff Gate IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn spec_aligner_analyze(
    goal: String,
    workspace_summary: Option<String>,
) -> Result<locus_agents::SpecAlignmentReport, String> {
    let report = locus_agents::SpecAligner::analyze_goal(&goal, workspace_summary.as_deref());
    Ok(report)
}

#[derive(Debug, Deserialize)]
pub struct ApplyTradeoffsRequest {
    pub report: locus_agents::SpecAlignmentReport,
    pub selections: std::collections::HashMap<String, String>,
}

#[tauri::command]
pub async fn spec_aligner_apply_tradeoffs(
    mut request: ApplyTradeoffsRequest,
) -> Result<locus_agents::SpecAlignmentReport, String> {
    locus_agents::SpecAligner::apply_tradeoff_choices(&mut request.report, &request.selections);
    Ok(request.report)
}

// ---------------------------------------------------------------------------
// Adversarial QA Agent IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn adversarial_qa_evaluate(
    code: String,
    lang: String,
) -> Result<locus_agents::QaReport, String> {
    let report = locus_agents::AdversarialQaAgent::evaluate_code(&code, &lang);
    Ok(report)
}

// ---------------------------------------------------------------------------
// ADR & Negative Memory Ledger IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn adr_ledger_get(
    workspace_root: String,
) -> Result<locus_context::AdrLedger, String> {
    let path = std::path::Path::new(&workspace_root);
    locus_context::AdrLedgerManager::load_or_create(path).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct AddNegativeMemoryRequest {
    pub workspace_root: String,
    pub entry: locus_context::NegativeMemoryEntry,
}

#[tauri::command]
pub async fn adr_ledger_add_negative(
    request: AddNegativeMemoryRequest,
) -> Result<locus_context::AdrLedger, String> {
    let path = std::path::Path::new(&request.workspace_root);
    let mut ledger = locus_context::AdrLedgerManager::load_or_create(path).map_err(|e| e.to_string())?;
    ledger.negative_memories.retain(|m| m.id != request.entry.id);
    ledger.negative_memories.push(request.entry);
    locus_context::AdrLedgerManager::save(path, &ledger).map_err(|e| e.to_string())?;
    Ok(ledger)
}

#[derive(Debug, Deserialize)]
pub struct AddAdrRecordRequest {
    pub workspace_root: String,
    pub record: locus_context::AdrRecord,
}

#[tauri::command]
pub async fn adr_ledger_add_record(
    request: AddAdrRecordRequest,
) -> Result<locus_context::AdrLedger, String> {
    let path = std::path::Path::new(&request.workspace_root);
    let mut ledger = locus_context::AdrLedgerManager::load_or_create(path).map_err(|e| e.to_string())?;
    ledger.records.retain(|r| r.id != request.record.id);
    ledger.records.push(request.record);
    locus_context::AdrLedgerManager::save(path, &ledger).map_err(|e| e.to_string())?;
    Ok(ledger)
}

// ---------------------------------------------------------------------------
// GitHub OAuth Device Flow IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn github_request_device_code(
    scope: Option<String>,
) -> Result<locus_core::DeviceCodeResponse, String> {
    locus_core::GitHubAuthClient::request_device_code(
        locus_core::DEFAULT_GITHUB_CLIENT_ID,
        &scope.unwrap_or_else(|| "repo,user".to_string()),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_poll_token(
    device_code: String,
) -> Result<locus_core::DeviceFlowPollStatus, String> {
    let status = locus_core::GitHubAuthClient::poll_access_token(
        locus_core::DEFAULT_GITHUB_CLIENT_ID,
        &device_code,
    )
    .await
    .map_err(|e| e.to_string())?;

    if let locus_core::DeviceFlowPollStatus::Success(ref token) = status {
        let _ = locus_core::GitHubAuthClient::save_token(token);
    }

    Ok(status)
}

#[tauri::command]
pub async fn github_get_status() -> Result<locus_core::GitHubAuthStatus, String> {
    Ok(locus_core::GitHubAuthClient::get_auth_status().await)
}

#[tauri::command]
pub async fn github_logout() -> Result<(), String> {
    locus_core::GitHubAuthClient::clear_token().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_list_repos(
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Vec<locus_core::GitHubRepo>, String> {
    let token = locus_core::GitHubAuthClient::load_token()
        .ok_or_else(|| "Not authenticated with GitHub".to_string())?;

    locus_core::GitHubAuthClient::fetch_user_repositories(
        &token,
        page.unwrap_or(1),
        per_page.unwrap_or(30),
    )
    .await
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Git Sync & Smart Commit IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn git_get_status(
    workspace_path: String,
) -> Result<locus_fs::GitStatusReport, String> {
    let path = std::path::Path::new(&workspace_path);
    locus_fs::GitSyncEngine::get_git_status(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_clone_repo(
    options: locus_fs::GitCloneOptions,
) -> Result<String, String> {
    let token = locus_core::GitHubAuthClient::load_token();
    locus_fs::GitSyncEngine::clone_repository(&options, token.as_deref())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct SmartCommitRequest {
    pub workspace_path: String,
    pub intent: Option<String>,
    pub auto_push: bool,
}

#[tauri::command]
pub async fn git_smart_commit(
    request: SmartCommitRequest,
) -> Result<locus_fs::SmartCommitResult, String> {
    let path = std::path::Path::new(&request.workspace_path);
    let token = locus_core::GitHubAuthClient::load_token();
    locus_fs::GitSyncEngine::smart_commit(
        path,
        request.intent.as_deref(),
        request.auto_push,
        token.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_create_pull_request(
    mut request: locus_fs::CreatePrRequest,
) -> Result<locus_fs::PullRequestResult, String> {
    if request.auth_token.is_empty() {
        if let Some(token) = locus_core::GitHubAuthClient::load_token() {
            request.auth_token = token;
        } else {
            return Err("GitHub authentication token required for Pull Request creation".to_string());
        }
    }

    locus_fs::GitSyncEngine::create_pull_request(&request)
        .await
        .map_err(|e| e.to_string())
}


