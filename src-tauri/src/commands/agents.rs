use crate::state::AppState;
use locus_agents::{AgentHandle, AgentStats, AgentStatus, Task, TaskResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SpawnAgentRequest {
    pub context: String,
    pub language: String,
    pub timeout_seconds: Option<u64>,
    pub max_memory_mb: Option<u64>,
    pub test_command: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
}

#[tauri::command]
pub async fn agent_spawn(
    state: State<'_, AppState>,
    request: SpawnAgentRequest,
) -> Result<AgentHandle, String> {
    let mut task = Task::new(request.context, request.language)
        .with_timeout(request.timeout_seconds.unwrap_or(300))
        .with_memory(request.max_memory_mb.unwrap_or(512));

    if let Some(test_cmd) = request.test_command {
        task = task.with_test_command(test_cmd);
    }

    if let Some(env) = request.env_vars {
        for (k, v) in env {
            task = task.with_env(k, v);
        }
    }

    state
        .agents
        .spawn_agent(task)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_kill(state: State<'_, AppState>, agent_id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&agent_id).map_err(|e| e.to_string())?;
    state.agents.kill_agent(id).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct AgentStatusDto {
    pub status: Option<String>,
}

#[tauri::command]
pub async fn agent_status(state: State<'_, AppState>, agent_id: String) -> Result<AgentStatusDto, String> {
    let id = Uuid::parse_str(&agent_id).map_err(|e| e.to_string())?;
    let status = state.agents.agent_status(id);
    Ok(AgentStatusDto {
        status: status.map(|s| format!("{:?}", s)),
    })
}

#[tauri::command]
pub async fn agent_list_active(state: State<'_, AppState>) -> Result<Vec<AgentHandle>, String> {
    Ok(state.agents.list_active_agents())
}

#[tauri::command]
pub async fn agent_monitor(state: State<'_, AppState>, agent_id: String) -> Result<AgentStats, String> {
    let id = Uuid::parse_str(&agent_id).map_err(|e| e.to_string())?;
    state.agents.monitor_agent(id).await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct ExecuteTaskRequest {
    pub context: String,
    pub language: String,
    pub timeout_seconds: Option<u64>,
    pub max_memory_mb: Option<u64>,
    pub test_command: Option<String>,
}

#[tauri::command]
pub async fn agent_execute_task(
    state: State<'_, AppState>,
    request: ExecuteTaskRequest,
) -> Result<TaskResult, String> {
    let mut task = Task::new(request.context, request.language)
        .with_timeout(request.timeout_seconds.unwrap_or(300))
        .with_memory(request.max_memory_mb.unwrap_or(512));

    if let Some(test_cmd) = request.test_command {
        task = task.with_test_command(test_cmd);
    }

    state
        .agents
        .execute_task(task)
        .await
        .map_err(|e| e.to_string())
}

#[allow(dead_code)]
fn _statuses() -> Vec<AgentStatus> {
    vec![
        AgentStatus::Created,
        AgentStatus::Running,
        AgentStatus::Stopped,
        AgentStatus::Killed,
    ]
}

// === Simplified Tauri API ===

#[derive(Serialize)]
pub struct AgentStatusSimple {
    pub id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
}

#[tauri::command]
pub async fn get_agent_status(
    state: State<'_, AppState>,
) -> Result<Vec<AgentStatusSimple>, String> {
    let agents = state.agents.list_active_agents();
    Ok(agents
        .into_iter()
        .map(|a| AgentStatusSimple {
            id: a.id.to_string(),
            status: format!("{:?}", a.status),
            pid: a.pid,
            started_at: a.started_at.map(|t| t.to_rfc3339()),
        })
        .collect())
}
