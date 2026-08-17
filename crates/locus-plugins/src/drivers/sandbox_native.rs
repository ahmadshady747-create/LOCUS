//! Native Process Execution Sandbox Driver.

use crate::traits::SandboxSlot;
use crate::types::{ExecutionResult, SlotError};
use async_trait::async_trait;
use std::time::Instant;
use tokio::process::Command;
use tracing::debug;

pub struct NativeProcessDriver;

impl NativeProcessDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeProcessDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SandboxSlot for NativeProcessDriver {
    fn driver_id(&self) -> &'static str {
        "native"
    }

    fn driver_name(&self) -> &'static str {
        "Native Process Execution"
    }

    async fn execute(&self, command: &str, working_dir: &str) -> Result<ExecutionResult, SlotError> {
        debug!("NativeProcessDriver executing: '{}' in '{}'", command, working_dir);
        let start = Instant::now();

        #[cfg(target_os = "windows")]
        let mut cmd = Command::new("powershell.exe");
        #[cfg(target_os = "windows")]
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", command]);

        #[cfg(not(target_os = "windows"))]
        let mut cmd = Command::new("sh");
        #[cfg(not(target_os = "windows"))]
        cmd.args(["-c", command]);

        if !working_dir.is_empty() {
            cmd.current_dir(working_dir);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| SlotError::ExecutionFailed(format!("Failed to spawn native command: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }
}
