//! Mock Sandbox Isolation Driver for dry-run testing.

use crate::traits::SandboxSlot;
use crate::types::{ExecutionResult, SlotError};
use async_trait::async_trait;
use tracing::debug;

pub struct MockIsolationDriver;

impl MockIsolationDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockIsolationDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SandboxSlot for MockIsolationDriver {
    fn driver_id(&self) -> &'static str {
        "mock"
    }

    fn driver_name(&self) -> &'static str {
        "Mock Sandbox Isolation"
    }

    async fn execute(&self, command: &str, working_dir: &str) -> Result<ExecutionResult, SlotError> {
        debug!("MockIsolationDriver dry-run execution: '{}' in '{}'", command, working_dir);

        Ok(ExecutionResult {
            stdout: format!("[MOCK ISOLATION DRY-RUN]: Command '{}' acknowledged safely.", command),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 1,
        })
    }
}
