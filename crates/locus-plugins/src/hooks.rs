//! Asynchronous Non-Blocking Lifecycle Hook Dispatcher.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PromptReceived,
    BeforeDiffApply,
    AfterDagExecution,
    OnToolFailed,
}

impl HookEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::PromptReceived => "prompt_received",
            Self::BeforeDiffApply => "before_diff_apply",
            Self::AfterDagExecution => "after_dag_execution",
            Self::OnToolFailed => "on_tool_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredHook {
    pub name: String,
    pub command: String,
    pub event: HookEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub event: HookEvent,
    pub hook_name: String,
    pub success: bool,
    pub error: Option<String>,
}

pub struct HookDispatcher {
    hooks: RwLock<HashMap<HookEvent, Vec<RegisteredHook>>>,
}

impl HookDispatcher {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a custom lifecycle hook script or command.
    pub fn register(&self, event: HookEvent, name: &str, command: &str) {
        let mut map = self.hooks.write();
        let list = map.entry(event).or_default();
        list.push(RegisteredHook {
            name: name.to_string(),
            command: command.to_string(),
            event,
        });
        info!("Registered lifecycle hook '{}' for event {:?}", name, event);
    }

    /// Dispatches an event to all registered hooks asynchronously without blocking the caller.
    pub fn dispatch(&self, event: HookEvent, payload: &str) {
        let matching_hooks = {
            let map = self.hooks.read();
            map.get(&event).cloned().unwrap_or_default()
        };

        if matching_hooks.is_empty() {
            return;
        }

        let payload_owned = payload.to_string();
        debug!(
            "Dispatching event {:?} to {} registered hook(s)",
            event,
            matching_hooks.len()
        );

        tokio::spawn(async move {
            for hook in matching_hooks {
                debug!("Executing hook '{}' for event {:?}", hook.name, event);

                #[cfg(target_os = "windows")]
                let mut cmd = Command::new("powershell.exe");
                #[cfg(target_os = "windows")]
                cmd.args(["-NoProfile", "-NonInteractive", "-Command", &hook.command]);

                #[cfg(not(target_os = "windows"))]
                let mut cmd = Command::new("sh");
                #[cfg(not(target_os = "windows"))]
                cmd.args(["-c", &hook.command]);

                cmd.env("LOCUS_HOOK_EVENT", event.name());
                cmd.env("LOCUS_HOOK_PAYLOAD", &payload_owned);

                match cmd.output().await {
                    Ok(output) if output.status.success() => {
                        debug!("Hook '{}' finished successfully", hook.name);
                    }
                    Ok(output) => {
                        warn!(
                            "Hook '{}' exited with code {}: {}",
                            hook.name,
                            output.status.code().unwrap_or(-1),
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    Err(e) => {
                        error!("Failed to spawn hook '{}': {}", hook.name, e);
                    }
                }
            }
        });
    }
}

impl Default for HookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
