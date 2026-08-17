use crate::monitor::AgentMonitor;
use crate::sandbox::{spawn_agent, execute_spawned_agent};
use crate::task::{AgentHandle, AgentStatus, SandboxConfig, Task, TaskResult, TestResults};
use anyhow::Result;
use tracing::{info, warn, error};
use uuid::Uuid;

pub async fn agent_lifecycle(task: Task) -> Result<TaskResult> {
    info!("Starting agent lifecycle for task {}", task.id);
    
    let config = SandboxConfig {
        memory_limit_mb: task.max_memory_mb,
        timeout_seconds: task.timeout_seconds,
        cpu_limit: Some(1.0),
        network_allowed: false,
        read_only_fs: true,
        allowed_paths: vec!["/tmp".to_string()],
        blocked_syscalls: vec![],
    };

    let mut handle = spawn_agent(task.clone(), config).await?;
    let agent_id = handle.id;

    let monitor = AgentMonitor::new();
    monitor.register(handle.clone());

    if task.test_command.is_some() {
        monitor.auto_kill_on_timeout(agent_id, std::time::Duration::from_secs(task.timeout_seconds)).await;
        monitor.auto_kill_on_memory(agent_id, task.max_memory_mb, std::time::Duration::from_secs(5)).await;
    }

    let start = std::time::Instant::now();
    let (stdout, stderr, exit_code, _, peak_memory_mb) = execute_spawned_agent(&mut handle, &task).await?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let success = exit_code == Some(0);
    let mut errors = Vec::new();
    if !success {
        errors.push(format!("Process exited with code {:?}", exit_code));
        if !stderr.is_empty() {
            errors.push(stderr.clone());
        }
    }

    let mut result = TaskResult {
        success,
        output: stdout,
        errors,
        duration_ms,
        peak_memory_mb,
        exit_code,
        test_results: None,
    };

    if let Some(ref test_cmd) = task.test_command {
        handle.status = AgentStatus::Testing;
        let test_results = run_tests(&task, &test_cmd).await?;
        result.test_results = Some(test_results);
        
        if result.test_results.as_ref().map(|t| t.failed > 0).unwrap_or(false) {
            result.success = false;
            result.errors.push("Tests failed".to_string());
        }
    }

    kill_agent(&mut handle).await?;
    handle.status = if result.success { AgentStatus::Stopped } else { AgentStatus::Failed };
    handle.completed_at = Some(chrono::Utc::now());

    info!("Agent lifecycle completed for task {}: success={}", task.id, result.success);
    Ok(result)
}

async fn run_tests(task: &Task, test_cmd: &str) -> Result<TestResults> {
    use std::process::Command;
    
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(test_cmd);
    
    if let Some(ref dir) = task.working_dir {
        cmd.current_dir(dir);
    }

    let output = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let passed = combined.matches("PASS").count() 
                 + combined.matches("PASSED").count() 
                 + combined.matches("passed").count();
    
    let failed = combined.matches("FAILED").count() +
                 combined.matches("failed").count() +
                 combined.matches("FAIL").count();

    Ok(TestResults {
        passed,
        failed,
        output: combined,
    })
}

pub async fn kill_agent(handle: &mut AgentHandle) -> Result<()> {
    info!("Killing agent {}", handle.id);
    
    handle.status = AgentStatus::Stopping;
    
    let mut sandbox = crate::sandbox::Sandbox::new(handle.config.clone());
    sandbox.kill().await?;
    
    handle.status = AgentStatus::Killed;
    handle.completed_at = Some(chrono::Utc::now());
    
    Ok(())
}

pub struct AgentManager {
    monitor: AgentMonitor,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            monitor: AgentMonitor::new(),
        }
    }

    pub async fn spawn_agent(&self, task: Task) -> Result<AgentHandle> {
        let config = SandboxConfig {
            memory_limit_mb: task.max_memory_mb,
            timeout_seconds: task.timeout_seconds,
            cpu_limit: Some(1.0),
            network_allowed: false,
            read_only_fs: true,
            allowed_paths: vec!["/tmp".to_string()],
            blocked_syscalls: vec![],
        };

        let handle = spawn_agent(task, config).await?;
        self.monitor.register(handle.clone());
        Ok(handle)
    }

    pub async fn kill_agent(&self, agent_id: Uuid) -> Result<()> {
        if let Some(mut handle) = self.monitor.unregister(agent_id) {
            kill_agent(&mut handle).await?;
        }
        Ok(())
    }

    pub fn agent_status(&self, agent_id: Uuid) -> Option<AgentStatus> {
        self.monitor.get(agent_id).map(|h| h.status)
    }

    pub fn list_active_agents(&self) -> Vec<AgentHandle> {
        self.monitor.list_active()
    }

    pub async fn monitor_agent(&self, agent_id: Uuid) -> Result<crate::task::AgentStats> {
        self.monitor.monitor_agent(agent_id).await
    }

    pub async fn execute_task(&self, task: Task) -> Result<TaskResult> {
        agent_lifecycle(task).await
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}