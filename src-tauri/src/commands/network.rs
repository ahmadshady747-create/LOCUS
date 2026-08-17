use crate::state::AppState;
use locus_network::{
    DeviceCapabilities, DeviceType, LocalDevice, NetworkTask, Specialization, TaskPriority,
    TaskType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn network_start(state: State<'_, AppState>) -> Result<(), String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        net.start().await.map_err(|e| e.to_string())
    } else {
        Err("Network orchestrator not initialized".to_string())
    }
}

#[tauri::command]
pub async fn network_stop(state: State<'_, AppState>) -> Result<(), String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        net.stop().await.map_err(|e| e.to_string())
    } else {
        Err("Network orchestrator not initialized".to_string())
    }
}

#[tauri::command]
pub async fn network_discover_devices(state: State<'_, AppState>) -> Result<Vec<LocalDevice>, String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        Ok(net.discover_devices().await)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn network_get_local_device(state: State<'_, AppState>) -> Result<LocalDevice, String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        Ok(net.get_local_device().await)
    } else {
        Err("Network orchestrator not initialized".to_string())
    }
}

#[derive(Deserialize)]
pub struct AssignTaskRequest {
    pub prompt: String,
    pub task_type: Option<String>,
    pub model_preference: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub stream: Option<bool>,
}

#[derive(Serialize)]
pub struct AssignTaskResponse {
    pub task_id: Uuid,
    pub response: String,
    pub peer_id: Uuid,
    pub duration_ms: u64,
    pub used_local: bool,
}

#[tauri::command]
pub async fn network_assign_task(
    state: State<'_, AppState>,
    request: AssignTaskRequest,
) -> Result<AssignTaskResponse, String> {
    let task_type = match request.task_type.as_deref() {
        Some("codegen") => TaskType::GenerateCode,
        Some("review") => TaskType::ReviewCode,
        Some("test") => TaskType::RunTests,
        Some("lint") => TaskType::LintCode,
        Some("embed") => TaskType::GenerateEmbeddings,
        Some("docs") => TaskType::GenerateDocs,
        Some("security") => TaskType::SecurityAudit,
        _ => TaskType::GenerateCode,
    };

    let task = NetworkTask {
        id: Uuid::new_v4(),
        task_type,
        payload: serde_json::json!({
            "prompt": request.prompt,
            "model": request.model_preference,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        }),
        priority: TaskPriority::Normal,
        required_capabilities: DeviceCapabilities::default(),
        created_at: chrono::Utc::now(),
        timeout_seconds: 120,
    };

    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        let start = std::time::Instant::now();
        let result = net.assign_task(task).await.map_err(|e| e.to_string())?;
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(AssignTaskResponse {
            task_id: result.task_id,
            response: result
                .output
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            peer_id: result.device_id.0,
            duration_ms,
            used_local: result.device_id == net.get_local_device().await.id,
        })
    } else {
        Err("Network orchestrator not initialized".to_string())
    }
}

#[derive(Serialize)]
pub struct LoadBalancerStateDto {
    pub devices: Vec<DeviceLoadDto>,
}

#[derive(Serialize)]
pub struct DeviceLoadDto {
    pub device_id: String,
    pub load: f32,
}

#[tauri::command]
pub async fn network_load_balancer_state(
    state: State<'_, AppState>,
) -> Result<LoadBalancerStateDto, String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        let lb_state = net.get_load_balancer_state().await;
        Ok(LoadBalancerStateDto {
            devices: lb_state
                .into_iter()
                .map(|(id, load)| DeviceLoadDto {
                    device_id: id.0.to_string(),
                    load,
                })
                .collect(),
        })
    } else {
        Ok(LoadBalancerStateDto { devices: vec![] })
    }
}

// === Simplified Tauri API ===

#[derive(Serialize)]
pub struct LocalDeviceSimple {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub status: String,
    pub device_type: String,
    pub models: Vec<String>,
    pub vram_gb: Option<f32>,
    pub specializations: Vec<String>,
}

#[tauri::command]
pub async fn get_local_devices(
    state: State<'_, AppState>,
) -> Result<Vec<LocalDeviceSimple>, String> {
    let net = state.network.read().await;
    if let Some(net) = net.as_ref() {
        let devices = net.discover_devices().await;
        Ok(devices
            .into_iter()
            .map(|d| LocalDeviceSimple {
                id: d.id.0.to_string(),
                name: d.name,
                hostname: d.hostname,
                ip_address: d.ip_address,
                port: d.port,
                status: format!("{:?}", d.status),
                device_type: format!("{:?}", d.device_type),
                models: d.capabilities.models.iter().map(|m| m.name.clone()).collect(),
                vram_gb: d.capabilities.vram_gb,
                specializations: d.capabilities.specializations.iter().map(|s| format!("{:?}", s)).collect(),
            })
            .collect())
    } else {
        Ok(vec![])
    }
}