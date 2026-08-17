//! Background Compiler Diagnostics Probe for LOCUS.
//!
//! Executes non-blocking compiler checks in the background (Rust, TypeScript, Python),
//! guarantees Single-Flight Execution to prevent `.cargo-lock` collisions and CPU spikes,
//! parses structured JSON and stream compiler diagnostics, and resiliently treats non-zero exit codes
//! as valid diagnostic output.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticItem {
    pub file_path: String,
    pub line: usize,
    pub col: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
    pub code: Option<String>,
}

#[derive(Debug, Default)]
pub struct DiagnosticStore {
    // Map: workspace_root -> list of active diagnostics
    entries: HashMap<String, Vec<DiagnosticItem>>,
}

impl DiagnosticStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_workspace(&mut self, workspace_root: &str, items: Vec<DiagnosticItem>) {
        self.entries.insert(workspace_root.to_string(), items);
    }

    pub fn get_workspace_diagnostics(&self, workspace_root: &str) -> Vec<DiagnosticItem> {
        self.entries.get(workspace_root).cloned().unwrap_or_default()
    }

    pub fn get_all_diagnostics(&self) -> Vec<DiagnosticItem> {
        self.entries.values().flatten().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Single-Flight guarded background compiler diagnostics probe runner.
#[derive(Debug, Clone)]
pub struct CompilerProbeEngine {
    store: Arc<RwLock<DiagnosticStore>>,
    is_running: Arc<AtomicBool>,
}

impl Default for CompilerProbeEngine {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(DiagnosticStore::new())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl CompilerProbeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store(&self) -> Arc<RwLock<DiagnosticStore>> {
        self.store.clone()
    }

    /// Runs compiler checks in the background using Single-Flight execution.
    /// If an existing check is already running for the workspace, it yields early
    /// to avoid `target/.cargo-lock` collisions and CPU spikes.
    pub async fn probe_workspace(&self, workspace_root: &Path) -> Result<Vec<DiagnosticItem>, String> {
        // Single-Flight guard: Ensure only one check runs at a time
        if self.is_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            // Already running: return currently cached diagnostics without colliding
            let root_str = workspace_root.to_string_lossy().to_string();
            return Ok(self.store.read().get_workspace_diagnostics(&root_str));
        }

        let result = self.execute_probe_internal(workspace_root).await;
        self.is_running.store(false, Ordering::SeqCst);

        result
    }

    async fn execute_probe_internal(&self, workspace_root: &Path) -> Result<Vec<DiagnosticItem>, String> {
        let mut diagnostics = Vec::new();
        let root_str = workspace_root.to_string_lossy().to_string();

        // 1. Rust project detection (Cargo.toml)
        if workspace_root.join("Cargo.toml").exists() {
            if let Ok(items) = run_cargo_check(workspace_root).await {
                diagnostics.extend(items);
            }
        }

        // 2. TypeScript / JavaScript detection (tsconfig.json or package.json)
        if workspace_root.join("tsconfig.json").exists() || workspace_root.join("package.json").exists() {
            if let Ok(items) = run_tsc_check(workspace_root).await {
                diagnostics.extend(items);
            }
        }

        // 3. Python detection (pyproject.toml or requirements.txt or *.py files)
        if workspace_root.join("pyproject.toml").exists() || workspace_root.join("requirements.txt").exists() {
            if let Ok(items) = run_python_check(workspace_root).await {
                diagnostics.extend(items);
            }
        }

        // Update in-memory store
        self.store.write().update_workspace(&root_str, diagnostics.clone());

        Ok(diagnostics)
    }
}

// === Compiler Runners & Parsers ===

async fn run_cargo_check(workspace_root: &Path) -> Result<Vec<DiagnosticItem>, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["check", "--message-format=json", "--lib"])
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Non-blocking timeout of 10s
    let child = cmd.spawn().map_err(|e| format!("Failed to spawn cargo check: {}", e))?;
    let output = match tokio::time::timeout(Duration::from_secs(10), child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_) => return Err("cargo check timed out".to_string()),
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_cargo_json(&stdout_str))
}

pub fn parse_cargo_json(stdout: &str) -> Vec<DiagnosticItem> {
    let mut items = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if val.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                if let Some(msg) = val.get("message") {
                    let level_str = msg.get("level").and_then(|l| l.as_str()).unwrap_or("error");
                    let severity = match level_str {
                        "error" => DiagnosticSeverity::Error,
                        "warning" => DiagnosticSeverity::Warning,
                        _ => DiagnosticSeverity::Information,
                    };

                    let rendered = msg.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    let code = msg.get("code").and_then(|c| c.get("code")).and_then(|c| c.as_str()).map(|s| s.to_string());

                    // Extract primary span
                    if let Some(spans) = msg.get("spans").and_then(|s| s.as_array()) {
                        for span in spans {
                            let is_primary = span.get("is_primary").and_then(|p| p.as_bool()).unwrap_or(false);
                            if is_primary {
                                let file_name = span.get("file_name").and_then(|f| f.as_str()).unwrap_or("").to_string();
                                let line_start = span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(1) as usize;
                                let col_start = span.get("column_start").and_then(|c| c.as_u64()).unwrap_or(1) as usize;

                                items.push(DiagnosticItem {
                                    file_path: file_name.replace('\\', "/"),
                                    line: line_start,
                                    col: col_start,
                                    severity,
                                    message: rendered.clone(),
                                    source: "rustc".to_string(),
                                    code: code.clone(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    items
}

async fn run_tsc_check(workspace_root: &Path) -> Result<Vec<DiagnosticItem>, String> {
    let mut cmd = Command::new("npx");
    cmd.args(["tsc", "--noEmit", "--pretty", "false"])
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Err(format!("npx tsc not found: {}", e)),
    };

    let output = match tokio::time::timeout(Duration::from_secs(10), child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_) => return Err("tsc timed out".to_string()),
    };

    // Non-zero exit code is normal when compiler discovers errors!
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_tsc_output(&stdout_str))
}

pub fn parse_tsc_output(stdout: &str) -> Vec<DiagnosticItem> {
    let mut items = Vec::new();
    let re = regex::Regex::new(r"([^\s:]+\.[a-zA-Z0-9]+)\((\d+),(\d+)\):\s*(error|warning)\s*(TS\d+):\s*(.+)").unwrap();
    let re_alt = regex::Regex::new(r"([^\s:]+\.[a-zA-Z0-9]+):(\d+):(\d+)\s*-\s*(error|warning)\s*(TS\d+):\s*(.+)").unwrap();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(caps) = re.captures(trimmed).or_else(|| re_alt.captures(trimmed)) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line: usize = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
            let col: usize = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
            let level = caps.get(4).map(|m| m.as_str()).unwrap_or("error");
            let code = caps.get(5).map(|m| m.as_str().to_string());
            let msg = caps.get(6).map(|m| m.as_str().to_string()).unwrap_or_default();

            items.push(DiagnosticItem {
                file_path: file_path.replace('\\', "/"),
                line,
                col,
                severity: if level == "error" { DiagnosticSeverity::Error } else { DiagnosticSeverity::Warning },
                message: msg,
                source: "tsc".to_string(),
                code,
            });
        }
    }

    items
}

async fn run_python_check(workspace_root: &Path) -> Result<Vec<DiagnosticItem>, String> {
    let mut cmd = Command::new("ruff");
    cmd.args(["check", "--output-format=json", "."])
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()), // Ruff not installed, silently skip
    };

    let output = match tokio::time::timeout(Duration::from_secs(8), child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_) => return Err("ruff timed out".to_string()),
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ruff_json(&stdout_str))
}

pub fn parse_ruff_json(stdout: &str) -> Vec<DiagnosticItem> {
    let mut items = Vec::new();
    if let Ok(vals) = serde_json::from_str::<Vec<serde_json::Value>>(stdout.trim()) {
        for v in vals {
            let filename = v.get("filename").and_then(|f| f.as_str()).unwrap_or("").to_string();
            let code = v.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
            let message = v.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
            let loc = v.get("location");
            let line = loc.and_then(|l| l.get("row")).and_then(|r| r.as_u64()).unwrap_or(1) as usize;
            let col = loc.and_then(|l| l.get("column")).and_then(|c| c.as_u64()).unwrap_or(1) as usize;

            items.push(DiagnosticItem {
                file_path: filename.replace('\\', "/"),
                line,
                col,
                severity: DiagnosticSeverity::Error,
                message,
                source: "ruff".to_string(),
                code,
            });
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_json_diagnostics() {
        let sample = r#"{"reason":"compiler-message","package_id":"locus 0.1.0","target":{"kind":["lib"],"name":"locus"},"message":{"rendered":"error[E0433]: cannot find crate\n","children":[],"code":{"code":"E0433","explanation":null},"level":"error","message":"cannot find crate `dirs`","spans":[{"column_start":16,"column_end":20,"file_name":"src/main.rs","is_primary":true,"line_start":12,"line_end":12}]}}"#;
        let items = parse_cargo_json(sample);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].file_path, "src/main.rs");
        assert_eq!(items[0].line, 12);
        assert_eq!(items[0].col, 16);
        assert_eq!(items[0].severity, DiagnosticSeverity::Error);
        assert_eq!(items[0].code, Some("E0433".to_string()));
    }

    #[test]
    fn test_parse_tsc_output() {
        let sample = "src/App.tsx(25,10): error TS2304: Cannot find name 'UnknownVar'.\nsrc/index.ts:10:5 - warning TS2322: Type 'string' is not assignable to type 'number'.";
        let items = parse_tsc_output(sample);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].file_path, "src/App.tsx");
        assert_eq!(items[0].line, 25);
        assert_eq!(items[0].severity, DiagnosticSeverity::Error);
        assert_eq!(items[1].file_path, "src/index.ts");
        assert_eq!(items[1].severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_diagnostic_store_crud() {
        let mut store = DiagnosticStore::new();
        let item = DiagnosticItem {
            file_path: "src/lib.rs".to_string(),
            line: 1,
            col: 1,
            severity: DiagnosticSeverity::Error,
            message: "Syntax Error".to_string(),
            source: "rustc".to_string(),
            code: None,
        };

        store.update_workspace("D:/LOCUS", vec![item.clone()]);
        let fetched = store.get_workspace_diagnostics("D:/LOCUS");
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].message, "Syntax Error");
    }
}
