//! Universal Silent Editor Bridge.
//!
//! Provides live atomic file synchronization, installed IDE detection (VS Code, Cursor, Zed, Neovim, JetBrains),
//! and cursor deep-linking for seamless pairing with external developer editors.

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedEditor {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub is_installed: bool,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorBridgeStatus {
    pub connected_editor: Option<DetectedEditor>,
    pub active_file: Option<String>,
    pub active_line: Option<usize>,
    pub active_column: Option<usize>,
    pub last_sync_timestamp: String,
    pub sync_mode: String,
    pub detected_editors: Vec<DetectedEditor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSyncReport {
    pub file_path: String,
    pub bytes_synced: usize,
    pub atomic_swap: bool,
    pub timestamp: String,
    pub duration_ms: u64,
}

pub struct EditorBridgeEngine {
    status: parking_lot::RwLock<EditorBridgeStatus>,
}

impl Default for EditorBridgeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBridgeEngine {
    pub fn new() -> Self {
        let editors = Self::detect_installed_editors();
        let connected = editors.iter().find(|e| e.is_installed).cloned();

        Self {
            status: parking_lot::RwLock::new(EditorBridgeStatus {
                connected_editor: connected,
                active_file: None,
                active_line: None,
                active_column: None,
                last_sync_timestamp: Utc::now().to_rfc3339(),
                sync_mode: "Atomic File Swap (Undo-Preserving)".to_string(),
                detected_editors: editors,
            }),
        }
    }

    /// Detects supported editors installed on the host system.
    pub fn detect_installed_editors() -> Vec<DetectedEditor> {
        let editor_specs = [
            ("vscode", "Visual Studio Code", vec!["code.cmd", "code.exe", "code"]),
            ("cursor", "Cursor AI Editor", vec!["cursor.cmd", "cursor.exe", "cursor"]),
            ("zed", "Zed Editor", vec!["zed.exe", "zed"]),
            ("neovim", "Neovim", vec!["nvim.exe", "nvim"]),
            ("jetbrains", "JetBrains IDE", vec!["idea64.exe", "idea.cmd", "idea", "pycharm64.exe", "webstorm64.exe"]),
            ("sublime", "Sublime Text", vec!["subl.exe", "sublime_text.exe", "subl"]),
        ];

        let mut results = Vec::new();

        for (id, name, execs) in editor_specs {
            let mut installed = false;
            let mut chosen_exec = execs[0].to_string();

            for exec in execs {
                if Self::is_executable_in_path(exec) {
                    installed = true;
                    chosen_exec = exec.to_string();
                    break;
                }
            }

            results.push(DetectedEditor {
                id: id.to_string(),
                name: name.to_string(),
                executable: chosen_exec,
                is_installed: installed,
                is_running: installed, // Heuristic default
            });
        }

        results
    }

    /// Atomically writes content to the target file by creating a tempfile in the same directory and renaming.
    /// This avoids lockups in IDEs and keeps undo history intact.
    pub fn atomic_write_file(path: &Path, content: &str) -> Result<EditorSyncReport> {
        let start = Instant::now();

        // 1. Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

        // 2. Create temporary file in the exact same directory (guarantees atomic rename across same filesystem partition)
        let mut temp_file = tempfile::Builder::new()
            .prefix(".locus_tmp_")
            .tempfile_in(parent_dir)?;

        // 3. Write content
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;

        // 4. Atomic persist / rename to target path with resilient retry on Windows NTFS contention
        let mut tf = temp_file;
        let mut last_err = None;

        for attempt in 0..12 {
            match tf.persist(path) {
                Ok(_) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    tf = e.file;
                    last_err = Some(e.error);
                    std::thread::sleep(std::time::Duration::from_millis(1 + attempt));
                }
            }
        }

        if let Some(err) = last_err {
            return Err(anyhow!("Failed to atomically swap file: {}", err));
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let report = EditorSyncReport {
            file_path: path.to_string_lossy().replace('\\', "/"),
            bytes_synced: content.len(),
            atomic_swap: true,
            timestamp: Utc::now().to_rfc3339(),
            duration_ms,
        };

        debug!("Atomic sync completed for {}: {} bytes in {}ms", report.file_path, report.bytes_synced, duration_ms);
        Ok(report)
    }

    /// Opens a file in the active external editor and positions cursor at line/column.
    pub fn open_in_editor(
        path: &Path,
        line: Option<usize>,
        column: Option<usize>,
        preferred_editor: Option<&str>,
    ) -> Result<bool> {
        let editors = Self::detect_installed_editors();
        let target_editor = if let Some(pref) = preferred_editor {
            editors.iter().find(|e| e.id == pref)
        } else {
            editors.iter().find(|e| e.is_installed)
        };

        let editor = target_editor.ok_or_else(|| anyhow!("No supported editor found on host system"))?;
        let line_num = line.unwrap_or(1);
        let col_num = column.unwrap_or(1);
        let path_str = path.to_string_lossy();

        info!("Opening '{}' in {} at line {}:{}", path_str, editor.name, line_num, col_num);

        let mut cmd = Command::new(&editor.executable);

        match editor.id.as_str() {
            "vscode" | "cursor" => {
                // code -g file:line:col
                cmd.arg("-g").arg(format!("{}:{}:{}", path_str, line_num, col_num));
            }
            "zed" => {
                // zed file:line:col
                cmd.arg(format!("{}:{}:{}", path_str, line_num, col_num));
            }
            "neovim" => {
                // nvim +line file
                cmd.arg(format!("+{}", line_num)).arg(path_str.as_ref());
            }
            "jetbrains" => {
                // idea --line line --column col file
                cmd.arg("--line")
                    .arg(line_num.to_string())
                    .arg("--column")
                    .arg(col_num.to_string())
                    .arg(path_str.as_ref());
            }
            _ => {
                // Generic file open
                cmd.arg(path_str.as_ref());
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn()
            .map(|_| true)
            .map_err(|e| anyhow!("Failed to spawn editor '{}': {}", editor.executable, e))
    }

    /// Returns the current bridge status snapshot.
    pub fn get_status(&self) -> EditorBridgeStatus {
        let mut stat = self.status.read().clone();
        stat.detected_editors = Self::detect_installed_editors();
        if stat.connected_editor.is_none() {
            stat.connected_editor = stat.detected_editors.iter().find(|e| e.is_installed).cloned();
        }
        stat
    }

    /// Helper to check if a binary executable exists on system PATH.
    fn is_executable_in_path(exec: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("where").arg(exec).output();
            if let Ok(out) = output {
                return out.status.success();
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let output = Command::new("which").arg(exec).output();
            if let Ok(out) = output {
                return out.status.success();
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_file_creates_and_updates() {
        let dir = tempdir().unwrap();
        let target_file = dir.path().join("subfolder").join("test_file.rs");

        let content_v1 = "fn main() { println!(\"v1\"); }";
        let report1 = EditorBridgeEngine::atomic_write_file(&target_file, content_v1).unwrap();
        assert!(report1.atomic_swap);
        assert_eq!(report1.bytes_synced, content_v1.len());
        assert_eq!(fs::read_to_string(&target_file).unwrap(), content_v1);

        let content_v2 = "fn main() { println!(\"v2 modified\"); }";
        let report2 = EditorBridgeEngine::atomic_write_file(&target_file, content_v2).unwrap();
        assert_eq!(report2.bytes_synced, content_v2.len());
        assert_eq!(fs::read_to_string(&target_file).unwrap(), content_v2);
    }

    #[test]
    fn test_editor_detection_returns_supported_list() {
        let editors = EditorBridgeEngine::detect_installed_editors();
        assert!(!editors.is_empty());
        assert!(editors.iter().any(|e| e.id == "vscode"));
        assert!(editors.iter().any(|e| e.id == "cursor"));
        assert!(editors.iter().any(|e| e.id == "zed"));
        assert!(editors.iter().any(|e| e.id == "neovim"));
    }
}
