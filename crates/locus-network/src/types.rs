use locus_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDevice {
    pub id: DeviceId,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub capabilities: DeviceCapabilities,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub status: DeviceStatus,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceStatus {
    Online,
    Busy,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    Main,
    Worker,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub models: Vec<ModelInfo>,
    pub max_context_tokens: usize,
    pub vram_gb: Option<f32>,
    pub quantization: Vec<String>,
    pub cpu_cores: u32,
    pub memory_gb: f32,
    pub supports_gpu: bool,
    pub specializations: Vec<Specialization>,
    pub performance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Specialization {
    CodeGeneration,
    CodeReview,
    Testing,
    Linting,
    Embeddings,
    Documentation,
    SecurityAnalysis,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            models: vec![],
            max_context_tokens: 4096,
            vram_gb: None,
            quantization: vec![],
            cpu_cores: num_cpus::get() as u32,
            memory_gb: 8.0,
            supports_gpu: false,
            specializations: vec![Specialization::CodeGeneration],
            performance_score: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub quantization: String,
    pub context_window: usize,
    pub parameter_count: String,
    pub size_gb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTask {
    pub id: Uuid,
    pub task_type: TaskType,
    pub payload: serde_json::Value,
    pub priority: TaskPriority,
    pub required_capabilities: DeviceCapabilities,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    GenerateCode,
    ReviewCode,
    RunTests,
    LintCode,
    GenerateEmbeddings,
    GenerateDocs,
    SecurityAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 50,
    High = 100,
    Critical = 200,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub device_id: DeviceId,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub tokens_used: Option<usize>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAnnouncement {
    pub device: LocalDevice,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Announce(DeviceAnnouncement),
    TaskRequest(NetworkTask),
    TaskResponse(TaskResult),
    Heartbeat { device_id: DeviceId, status: DeviceStatus },
    DeviceListRequest,
    DeviceListResponse(Vec<LocalDevice>),
    Goodbye { device_id: DeviceId },
}