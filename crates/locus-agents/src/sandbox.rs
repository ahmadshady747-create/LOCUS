use crate::task::{AgentHandle, SandboxConfig, Task, AgentStatus};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

#[cfg(target_os = "linux")]
mod linux_sandbox {
    use super::*;
    use nix::unistd::{setrlimit, Resource};
    use nix::sys::resource::{rlimit, rlim_t};
    use std::os::unix::process::CommandExt;

    pub fn apply_resource_limits(config: &SandboxConfig) -> Result<()> {
        setrlimit(
            Resource::RLIMIT_AS,
            rlimit {
                rlim_cur: (config.memory_limit_mb * 1024 * 1024) as rlim_t,
                rlim_max: (config.memory_limit_mb * 1024 * 1024) as rlim_t,
            },
        ).context("Failed to set memory limit")?;

        if let Some(cpu_limit) = config.cpu_limit {
            let cpu_time = (cpu_limit * 60.0) as rlim_t;
            setrlimit(
                Resource::RLIMIT_CPU,
                rlimit {
                    rlim_cur: cpu_time,
                    rlim_max: cpu_time + 1,
                },
            ).context("Failed to set CPU limit")?;
        }

        setrlimit(
            Resource::RLIMIT_NOFILE,
            rlimit {
                rlim_cur: 64,
                rlim_max: 64,
            },
        ).context("Failed to set file descriptor limit")?;

        setrlimit(
            Resource::RLIMIT_NPROC,
            rlimit {
                rlim_cur: 32,
                rlim_max: 32,
            },
        ).context("Failed to set process limit")?;

        Ok(())
    }

    pub fn create_sandboxed_command(task: &Task, config: &SandboxConfig) -> Result<Command> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&task.context)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(ref dir) = task.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &task.env_vars {
            cmd.env(key, value);
        }

        if config.read_only_fs {
            cmd.env("HOME", "/tmp");
            cmd.env("TMPDIR", "/tmp");
        }

        let config_clone = config.clone();
        unsafe {
            cmd.pre_exec(move || {
                apply_resource_limits(&config_clone)?;
                Ok(())
            });
        }

        Ok(cmd)
    }
}

#[cfg(not(target_os = "linux"))]
mod stub_sandbox {
    use super::*;

    #[allow(dead_code)]
    pub fn apply_resource_limits(_config: &SandboxConfig) -> Result<()> {
        warn!("Resource limits not implemented on this platform");
        Ok(())
    }

    pub fn create_sandboxed_command(task: &Task, _config: &SandboxConfig) -> Result<Command> {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&task.context);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&task.context);
            c
        };

        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(ref dir) = task.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &task.env_vars {
            cmd.env(key, value);
        }

        Ok(cmd)
    }
}

#[cfg(target_os = "linux")]
use linux_sandbox::{apply_resource_limits, create_sandboxed_command};

#[cfg(not(target_os = "linux"))]
use stub_sandbox::{apply_resource_limits, create_sandboxed_command};

pub struct Sandbox {
    pub config: SandboxConfig,
    child: Option<Child>,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config, child: None }
    }

    pub async fn execute(&mut self, task: &Task) -> Result<(String, String, Option<i32>, u64, u64)> {
        let start = std::time::Instant::now();
        
        let mut cmd = create_sandboxed_command(task, &self.config)?;
        cmd.kill_on_drop(true);
        let child = cmd.spawn().context("Failed to spawn process")?;
        
        let pid = child.id().unwrap_or(0);
        info!("Spawned agent process PID: {}", pid);
        
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let result = timeout(timeout_duration, async {
            let output = child.wait_with_output().await?;
            Ok::<_, anyhow::Error>(output)
        }).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let peak_memory_mb = self.estimate_peak_memory(pid).await.unwrap_or(0);

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                
                Ok((stdout, stderr, exit_code, duration_ms, peak_memory_mb))
            }
            Ok(Err(e)) => {
                Err(e).context("Process execution failed")
            }
            Err(_) => {
                warn!("Process timed out after {}s, killing", self.config.timeout_seconds);
                Err(anyhow::anyhow!("Process timed out after {} seconds", self.config.timeout_seconds))
            }
        }
    }

    async fn estimate_peak_memory(&self, pid: u32) -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status_path = format!("/proc/{}/status", pid);
            if let Ok(content) = fs::read_to_string(&status_path) {
                for line in content.lines() {
                    if line.starts_with("VmPeak:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return Ok(kb / 1024);
                            }
                        }
                    }
                }
            }
        }
        Ok(0)
    }

    pub async fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().await?;
            child.wait().await?;
        }
        Ok(())
    }
}

pub async fn spawn_agent(task: Task, config: SandboxConfig) -> Result<AgentHandle> {
    let agent_id = Uuid::new_v4();
    info!("Spawning agent {} for task {}", agent_id, task.id);

    let handle = AgentHandle {
        id: agent_id,
        task_id: task.id,
        status: AgentStatus::Created,
        pid: None,
        started_at: None,
        completed_at: None,
        config: config.clone(),
    };

    Ok(handle)
}

pub async fn execute_spawned_agent(handle: &mut AgentHandle, task: &Task) -> Result<(String, String, Option<i32>, u64, u64)> {
    let config = handle.config.clone();
    handle.status = AgentStatus::Running;
    handle.started_at = Some(chrono::Utc::now());

    let mut sandbox = Sandbox::new(config);
    let (stdout, stderr, exit_code, duration_ms, peak_memory_mb) = sandbox.execute(task).await?;

    handle.status = if exit_code == Some(0) {
        AgentStatus::Stopped
    } else {
        AgentStatus::Failed
    };
    handle.completed_at = Some(chrono::Utc::now());

    Ok((stdout, stderr, exit_code, duration_ms, peak_memory_mb))
}

pub async fn kill_agent(agent: &mut AgentHandle) -> Result<()> {
    info!("Killing agent {}", agent.id);
    agent.status = AgentStatus::Killed;
    agent.completed_at = Some(chrono::Utc::now());
    Ok(())
}