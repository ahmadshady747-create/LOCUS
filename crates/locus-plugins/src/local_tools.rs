//! Zero-Panic Local Tool Runner, Windows Shebang Resolver, and Circuit Breaker.

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalToolManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub script_path: PathBuf,
    pub shebang: String,
    pub parameters: Vec<ToolParameter>,
    pub timeout_secs: u64,
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open {
        failure_count: u32,
        last_error: String,
        opened_at: String,
    },
    HalfOpen,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("Circuit breaker is OPEN for tool '{0}': {1} consecutive failures")]
    CircuitOpen(String, u32),

    #[error("Tool execution timed out after {0}s")]
    Timeout(u64),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool script I/O error: {0}")]
    IoError(String),
}

/// Windows & Cross-Platform Shebang Resolution Engine.
pub struct ShebangResolver;

impl ShebangResolver {
    /// Resolves the executable program and command-line arguments for a given script.
    pub fn resolve_interpreter_and_args(shebang: &str, script_path: &Path) -> (String, Vec<String>) {
        let script_str = script_path.to_string_lossy().to_string();
        let clean_shebang = shebang.trim().trim_start_matches("#!").trim();

        #[cfg(target_os = "windows")]
        {
            let lower = clean_shebang.to_lowercase();
            if lower.contains("python3") || lower.contains("python") {
                ("python".to_string(), vec![script_str])
            } else if lower.contains("node") {
                ("node".to_string(), vec![script_str])
            } else if lower.contains("pwsh") || lower.contains("powershell") {
                ("powershell.exe".to_string(), vec!["-NoProfile".to_string(), "-File".to_string(), script_str])
            } else if lower.contains("bash") || lower.contains("sh") {
                // Try Git Bash if installed or fallback to bash.exe / powershell
                let git_bash = Path::new("C:\\Program Files\\Git\\bin\\bash.exe");
                if git_bash.exists() {
                    (git_bash.to_string_lossy().to_string(), vec![script_str])
                } else {
                    ("bash.exe".to_string(), vec![script_str])
                }
            } else if lower.contains("ruby") {
                ("ruby".to_string(), vec![script_str])
            } else {
                // Extension fallback
                Self::resolve_by_extension(script_path)
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if clean_shebang.is_empty() {
                Self::resolve_by_extension(script_path)
            } else {
                let parts: Vec<&str> = clean_shebang.split_whitespace().collect();
                if let Some(cmd) = parts.first() {
                    let mut args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
                    args.push(script_str);
                    (cmd.to_string(), args)
                } else {
                    Self::resolve_by_extension(script_path)
                }
            }
        }
    }

    /// Extension-based executable resolution fallback.
    pub fn resolve_by_extension(script_path: &Path) -> (String, Vec<String>) {
        let script_str = script_path.to_string_lossy().to_string();
        let ext = script_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext.to_lowercase().as_str() {
            "py" => ("python".to_string(), vec![script_str]),
            "js" | "mjs" | "cjs" => ("node".to_string(), vec![script_str]),
            "ts" => ("npx".to_string(), vec!["ts-node".to_string(), script_str]),
            "ps1" => ("powershell.exe".to_string(), vec!["-NoProfile".to_string(), "-File".to_string(), script_str]),
            "sh" => ("bash".to_string(), vec![script_str]),
            _ => (script_str, vec![]),
        }
    }
}

/// Parses metadata header comments from a local tool script.
pub fn parse_script_headers(content: &str, path: &Path, is_global: bool) -> LocalToolManifest {
    let mut name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("tool").to_string();
    let id = format!("{}_{}", if is_global { "global" } else { "local" }, name.to_lowercase().replace(' ', "_"));
    let mut description = "Local developer automation tool".to_string();
    let mut shebang = String::new();
    let mut parameters = Vec::new();
    let mut timeout_secs = 5u64;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if idx == 0 && trimmed.starts_with("#!") {
            shebang = trimmed.to_string();
            continue;
        }

        if trimmed.starts_with("# @name:") || trimmed.starts_with("// @name:") {
            name = trimmed.splitn(2, ':').nth(1).unwrap_or(&name).trim().to_string();
        } else if trimmed.starts_with("# @description:") || trimmed.starts_with("// @description:") {
            description = trimmed.splitn(2, ':').nth(1).unwrap_or(&description).trim().to_string();
        } else if trimmed.starts_with("# @timeout:") || trimmed.starts_with("// @timeout:") {
            if let Some(sec_str) = trimmed.splitn(2, ':').nth(1) {
                if let Ok(t) = sec_str.trim().parse::<u64>() {
                    timeout_secs = t;
                }
            }
        } else if trimmed.starts_with("# @param:") || trimmed.starts_with("// @param:") {
            if let Some(param_spec) = trimmed.splitn(2, ':').nth(1) {
                let parts: Vec<&str> = param_spec.trim().split_whitespace().collect();
                if let Some(pname) = parts.first() {
                    let required = parts.get(1).map(|&s| s.contains("required")).unwrap_or(false);
                    let pdesc = if parts.len() > 2 { parts[2..].join(" ") } else { "Parameter".to_string() };
                    parameters.push(ToolParameter {
                        name: pname.to_string(),
                        description: pdesc,
                        required,
                        default_value: None,
                    });
                }
            }
        } else if !trimmed.starts_with('#') && !trimmed.starts_with("//") && !trimmed.is_empty() {
            // End of header comment block
            break;
        }
    }

    LocalToolManifest {
        id,
        name,
        description,
        script_path: path.to_path_buf(),
        shebang,
        parameters,
        timeout_secs,
        is_global,
    }
}

/// Discovers local tools in `.locus/tools/` and `~/.locus/tools/`.
pub fn discover_local_tools(workspace_root: Option<&Path>) -> Vec<LocalToolManifest> {
    let mut tools = Vec::new();
    let mut paths_to_check = Vec::new();

    // 1. Workspace tools (.locus/tools/)
    if let Some(root) = workspace_root {
        paths_to_check.push((root.join(".locus").join("tools"), false));
    }

    // 2. Global tools (~/.locus/tools/)
    if let Some(home) = dirs::home_dir() {
        paths_to_check.push((home.join(".locus").join("tools"), true));
    }

    for (dir, is_global) in paths_to_check {
        if !dir.exists() || !dir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let manifest = parse_script_headers(&content, &path, is_global);
                        tools.push(manifest);
                    }
                }
            }
        }
    }

    tools
}

/// Circuit Breaker Manager (Max 3 consecutive failures threshold).
pub struct CircuitBreakerManager {
    failures: RwLock<HashMap<String, (u32, String, String)>>,
}

impl CircuitBreakerManager {
    pub const FAILURE_THRESHOLD: u32 = 3;

    pub fn new() -> Self {
        Self {
            failures: RwLock::new(HashMap::new()),
        }
    }

    /// Checks whether execution is permitted for this tool.
    pub fn check_allowed(&self, tool_id: &str) -> Result<(), PluginError> {
        let map = self.failures.read();
        if let Some((count, _, _)) = map.get(tool_id) {
            if *count >= Self::FAILURE_THRESHOLD {
                return Err(PluginError::CircuitOpen(tool_id.to_string(), *count));
            }
        }
        Ok(())
    }

    /// Records a successful execution (resets failure count).
    pub fn record_success(&self, tool_id: &str) {
        let mut map = self.failures.write();
        map.remove(tool_id);
    }

    /// Records an execution failure and trips the circuit if threshold is reached.
    pub fn record_failure(&self, tool_id: &str, error: &str) {
        let mut map = self.failures.write();
        let entry = map.entry(tool_id.to_string()).or_insert((0, String::new(), String::new()));
        entry.0 += 1;
        entry.1 = error.to_string();
        entry.2 = Utc::now().to_rfc3339();

        if entry.0 >= Self::FAILURE_THRESHOLD {
            warn!("🚨 Circuit breaker TRIPPED (OPEN) for tool '{}' after {} failures!", tool_id, entry.0);
        }
    }

    /// Manually resets the circuit breaker for a tool.
    pub fn reset(&self, tool_id: &str) -> bool {
        let mut map = self.failures.write();
        map.remove(tool_id).is_some()
    }

    /// Returns the circuit states for all tracked tools.
    pub fn get_status(&self) -> HashMap<String, CircuitState> {
        let map = self.failures.read();
        let mut status = HashMap::new();

        for (id, (count, err, time)) in map.iter() {
            if *count >= Self::FAILURE_THRESHOLD {
                status.insert(
                    id.clone(),
                    CircuitState::Open {
                        failure_count: *count,
                        last_error: err.clone(),
                        opened_at: time.clone(),
                    },
                );
            } else if *count > 0 {
                status.insert(id.clone(), CircuitState::HalfOpen);
            } else {
                status.insert(id.clone(), CircuitState::Closed);
            }
        }

        status
    }
}

impl Default for CircuitBreakerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-Panic Local Tool Runner.
pub struct LocalToolRunner {
    pub circuit_breaker: CircuitBreakerManager,
}

impl LocalToolRunner {
    pub fn new() -> Self {
        Self {
            circuit_breaker: CircuitBreakerManager::new(),
        }
    }

    /// Executes a tool safely with timeout enforcement and zero-panic crash insulation.
    pub async fn execute_tool_safe(
        &self,
        tool: &LocalToolManifest,
        args: &[String],
        custom_timeout: Option<u64>,
    ) -> Result<ToolExecutionOutput, PluginError> {
        // 1. Check Circuit Breaker
        self.circuit_breaker.check_allowed(&tool.id)?;

        let timeout_secs = custom_timeout.unwrap_or(tool.timeout_secs);
        let start = Instant::now();

        let (program, mut cmd_args) = ShebangResolver::resolve_interpreter_and_args(&tool.shebang, &tool.script_path);
        cmd_args.extend_from_slice(args);

        debug!(
            "Spawning LocalTool '{}': {} {:?}",
            tool.name, program, cmd_args
        );

        let mut cmd = Command::new(&program);
        cmd.args(&cmd_args);

        if let Some(parent) = tool.script_path.parent() {
            cmd.current_dir(parent);
        }

        // 2. Execute with Timeout Watchdog
        let run_fut = cmd.output();
        let timeout_fut = tokio::time::timeout(Duration::from_secs(timeout_secs), run_fut);

        match timeout_fut.await {
            Ok(Ok(output)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let exit_code = output.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if exit_code == 0 {
                    self.circuit_breaker.record_success(&tool.id);
                } else {
                    self.circuit_breaker.record_failure(&tool.id, &format!("Exit code {}: {}", exit_code, stderr));
                }

                Ok(ToolExecutionOutput {
                    stdout,
                    stderr,
                    exit_code,
                    duration_ms,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => {
                let err_msg = format!("Failed to spawn command '{}': {}", program, e);
                self.circuit_breaker.record_failure(&tool.id, &err_msg);
                Err(PluginError::ExecutionFailed(err_msg))
            }
            Err(_) => {
                let err_msg = format!("Watchdog timed out after {}s", timeout_secs);
                self.circuit_breaker.record_failure(&tool.id, &err_msg);
                Ok(ToolExecutionOutput {
                    stdout: String::new(),
                    stderr: format!("Process timed out and was aborted after {} seconds", timeout_secs),
                    exit_code: -1,
                    duration_ms: start.elapsed().as_millis() as u64,
                    timed_out: true,
                })
            }
        }
    }
}

impl Default for LocalToolRunner {
    fn default() -> Self {
        Self::new()
    }
}
