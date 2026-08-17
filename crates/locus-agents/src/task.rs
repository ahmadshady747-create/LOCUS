use locus_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub context: String,
    pub timeout_seconds: u64,
    pub max_memory_mb: u64,
    pub language: String,
    pub test_command: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    pub fn new(context: String, language: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            context,
            timeout_seconds: 300,
            max_memory_mb: 512,
            language,
            test_command: None,
            working_dir: None,
            env_vars: HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_memory(mut self, mb: u64) -> Self {
        self.max_memory_mb = mb;
        self
    }

    pub fn with_test_command(mut self, cmd: String) -> Self {
        self.test_command = Some(cmd);
        self
    }

    pub fn with_working_dir(mut self, dir: String) -> Self {
        self.working_dir = Some(dir);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
    pub duration_ms: u64,
    pub peak_memory_mb: u64,
    pub exit_code: Option<i32>,
    pub test_results: Option<TestResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Created,
    Starting,
    Running,
    Testing,
    Stopping,
    Stopped,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandle {
    pub id: Uuid,
    pub task_id: Uuid,
    pub status: AgentStatus,
    pub pid: Option<u32>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub config: SandboxConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub memory_limit_mb: u64,
    pub cpu_limit: Option<f32>,
    pub timeout_seconds: u64,
    pub network_allowed: bool,
    pub read_only_fs: bool,
    pub allowed_paths: Vec<String>,
    pub blocked_syscalls: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 512,
            cpu_limit: Some(1.0),
            timeout_seconds: 300,
            network_allowed: false,
            read_only_fs: true,
            allowed_paths: vec!["/tmp".to_string()],
            blocked_syscalls: vec![
                "ptrace".to_string(),
                "process_vm_readv".to_string(),
                "process_vm_writev".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub memory_peak_mb: u64,
    pub disk_read_mb: u64,
    pub disk_write_mb: u64,
    pub network_rx_mb: u64,
    pub network_tx_mb: u64,
    pub uptime_seconds: u64,
    pub thread_count: u32,
}