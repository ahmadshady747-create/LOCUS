use crate::types::{
    DeviceCapabilities, DeviceId, DeviceStatus, DeviceType, LocalDevice, NetworkTask,
    Specialization, TaskPriority, TaskType,
};
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub struct LoadBalancer {
    devices: HashMap<DeviceId, LocalDevice>,
    task_queues: HashMap<DeviceId, Vec<NetworkTask>>,
    device_load: HashMap<DeviceId, f32>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            task_queues: HashMap::new(),
            device_load: HashMap::new(),
        }
    }

    pub fn update_devices(&mut self, devices: Vec<LocalDevice>) {
        for device in devices {
            if device.status == DeviceStatus::Online {
                self.devices.insert(device.id.clone(), device.clone());
                self.device_load.entry(device.id.clone()).or_insert(0.0);
                self.task_queues.entry(device.id.clone()).or_default();
            }
        }
        
        self.devices.retain(|id, _| self.device_load.contains_key(id));
    }

    pub fn remove_device(&mut self, device_id: &DeviceId) {
        self.devices.remove(device_id);
        self.device_load.remove(device_id);
        self.task_queues.remove(device_id);
    }

    pub fn get_device_ids(&self) -> Vec<DeviceId> {
        self.devices.keys().cloned().collect()
    }

    pub fn select_device(&self, task: &NetworkTask) -> Option<&DeviceId> {
        let candidates: Vec<_> = self.devices
            .values()
            .filter(|d| self.can_handle_task(d, task))
            .filter(|d| d.status == DeviceStatus::Online)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let best = candidates
            .into_iter()
            .max_by(|a, b| {
                let score_a = self.score_device(a, task);
                let score_b = self.score_device(b, task);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        best.map(|d| &d.id)
    }

    fn can_handle_task(&self, device: &LocalDevice, task: &NetworkTask) -> bool {
        let req = &task.required_capabilities;
        
        if req.max_context_tokens > device.capabilities.max_context_tokens {
            return false;
        }
        
        if !req.models.is_empty() {
            let has_model = req.models.iter().any(|required| {
                device.capabilities.models.iter().any(|available| 
                    available.name == required.name && 
                    available.quantization == required.quantization
                )
            });
            if !has_model {
                return false;
            }
        }
        
        if let Some(req_vram) = req.vram_gb {
            if device.capabilities.vram_gb.map(|v| v < req_vram).unwrap_or(true) {
                return false;
            }
        }
        
        if !req.specializations.is_empty() {
            let has_spec = req.specializations.iter().all(|s| 
                device.capabilities.specializations.contains(s)
            );
            if !has_spec {
                return false;
            }
        }
        
        true
    }

    fn score_device(&self, device: &LocalDevice, task: &NetworkTask) -> f32 {
        let mut score = device.capabilities.performance_score;
        
        let load = self.device_load.get(&device.id).copied().unwrap_or(0.0);
        score *= 1.0 - (load * 0.5);
        
        match task.task_type {
            TaskType::GenerateCode => {
                if device.capabilities.specializations.contains(&Specialization::CodeGeneration) {
                    score *= 1.5;
                }
                if device.device_type == DeviceType::Main {
                    score *= 1.3;
                }
            }
            TaskType::RunTests => {
                if device.capabilities.specializations.contains(&Specialization::Testing) {
                    score *= 1.5;
                }
            }
            TaskType::LintCode => {
                if device.capabilities.specializations.contains(&Specialization::Linting) {
                    score *= 1.5;
                }
            }
            TaskType::GenerateEmbeddings => {
                if device.capabilities.specializations.contains(&Specialization::Embeddings) {
                    score *= 1.5;
                }
            }
            TaskType::SecurityAudit => {
                if device.capabilities.specializations.contains(&Specialization::SecurityAnalysis) {
                    score *= 1.5;
                }
            }
            TaskType::GenerateDocs => {
                if device.capabilities.specializations.contains(&Specialization::Documentation) {
                    score *= 1.5;
                }
            }
            TaskType::ReviewCode => {
                if device.capabilities.specializations.contains(&Specialization::CodeReview) {
                    score *= 1.5;
                }
            }
        }
        
        match task.priority {
            TaskPriority::Critical => score *= 1.5,
            TaskPriority::High => score *= 1.2,
            TaskPriority::Normal => {},
            TaskPriority::Low => score *= 0.8,
        }
        
        score
    }

    pub fn assign_task(&mut self, task: NetworkTask, device_id: &DeviceId) -> Result<()> {
        self.task_queues
            .entry(device_id.clone())
            .or_default()
            .push(task);
        
        let current_load = self.device_load.get(device_id).copied().unwrap_or(0.0);
        self.device_load.insert(device_id.clone(), current_load + 0.1);
        
        Ok(())
    }

    pub fn complete_task(&mut self, device_id: &DeviceId) {
        let current_load = self.device_load.get(device_id).copied().unwrap_or(0.0);
        self.device_load.insert(device_id.clone(), (current_load - 0.1).max(0.0));
    }

    pub fn get_queue(&self, device_id: &DeviceId) -> Option<&Vec<NetworkTask>> {
        self.task_queues.get(device_id)
    }

    pub fn get_device_load(&self, device_id: &DeviceId) -> f32 {
        self.device_load.get(device_id).copied().unwrap_or(0.0)
    }

    pub fn get_all_devices(&self) -> Vec<&LocalDevice> {
        self.devices.values().collect()
    }

    pub fn get_online_devices(&self) -> Vec<&LocalDevice> {
        self.devices
            .values()
            .filter(|d| d.status == DeviceStatus::Online)
            .collect()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LocalFallback {
    pub device: LocalDevice,
}

impl LocalFallback {
    pub fn new(device: LocalDevice) -> Self {
        Self { device }
    }

    pub async fn execute_locally(&self, task: NetworkTask) -> Result<crate::types::TaskResult> {
        info!("Executing task {} locally on {}", task.id, self.device.name);
        
        let start = std::time::Instant::now();
        
        let output = match task.task_type {
            TaskType::GenerateCode => {
                serde_json::json!({
                    "code": "// Local fallback: code generation not implemented",
                    "language": "rust"
                })
            }
            TaskType::RunTests => {
                serde_json::json!({
                    "passed": 0,
                    "failed": 0,
                    "output": "Local fallback: testing not implemented"
                })
            }
            TaskType::LintCode => {
                serde_json::json!({
                    "errors": [],
                    "warnings": [],
                    "output": "Local fallback: linting not implemented"
                })
            }
            TaskType::GenerateEmbeddings => {
                serde_json::json!({
                    "embeddings": [],
                    "model": "local-fallback"
                })
            }
            TaskType::GenerateDocs => {
                serde_json::json!({
                    "documentation": "Local fallback: documentation generation not implemented"
                })
            }
            TaskType::SecurityAudit => {
                serde_json::json!({
                    "issues": [],
                    "summary": "Local fallback: security audit not implemented"
                })
            }
            TaskType::ReviewCode => {
                serde_json::json!({
                    "comments": [],
                    "summary": "Local fallback: code review not implemented"
                })
            }
        };
        
        Ok(crate::types::TaskResult {
            task_id: task.id,
            device_id: self.device.id.clone(),
            success: true,
            output,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
            tokens_used: None,
            completed_at: chrono::Utc::now(),
        })
    }
}
