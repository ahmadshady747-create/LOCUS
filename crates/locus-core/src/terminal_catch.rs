//! Terminal Error & Stack Trace Interception Engine for LOCUS.
//!
//! Intercepts and parses non-zero exit code outputs from the Sandbox Terminal,
//! stripping ANSI color codes, bounding stderr buffer inspection (max 64KB / 500 lines),
//! and producing structured, decoupled failure diagnostics.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Structured diagnostic of a detected error from terminal stderr/stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticLocation {
    pub file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub error_type: String,
    pub message: String,
}

/// Decoupled structured failure report produced by the core engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalFailureReport {
    pub command: String,
    pub exit_code: i32,
    pub clean_stderr_snippet: String,
    pub primary_error: Option<DiagnosticLocation>,
    pub all_diagnostics: Vec<DiagnosticLocation>,
    pub stack_trace_lines: Vec<String>,
}

/// Strips ANSI escape sequences (colors, cursor movements) from raw terminal output.
pub fn strip_ansi(raw: &str) -> String {
    let re = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").unwrap();
    re.replace_all(raw, "").to_string()
}

/// Analyzes terminal failure outputs and extracts structured diagnostic information.
pub fn process_terminal_failure(command: &str, exit_code: i32, raw_stderr: &str) -> TerminalFailureReport {
    // 1. Strip ANSI escape codes
    let clean = strip_ansi(raw_stderr);

    // 2. Cap inspection at max 64KB or 500 lines to prevent freeze
    let mut bounded_lines: Vec<&str> = Vec::new();
    let mut current_bytes = 0;
    const MAX_BYTES: usize = 64 * 1024;
    const MAX_LINES: usize = 500;

    for line in clean.lines() {
        if bounded_lines.len() >= MAX_LINES || current_bytes + line.len() > MAX_BYTES {
            break;
        }
        bounded_lines.push(line);
        current_bytes += line.len() + 1;
    }

    let mut diagnostics = Vec::new();
    let mut stack_trace_lines = Vec::new();

    // Regex patterns for various language compilers & runtimes:
    // Rust: `error[E0433]: cannot find... --> src/main.rs:12:16`
    let rust_loc_re = Regex::new(r"-->\s+([^\s:]+):(\d+):?(\d+)?").unwrap();
    let rust_err_re = Regex::new(r"error(?:\[E\d+\])?:\s*(.+)").unwrap();

    // Python: `File "app.py", line 42, in <module>`
    let py_loc_re = Regex::new(r#"File\s+"([^"]+)",\s+line\s+(\d+)"#).unwrap();

    // Node/TypeScript: `at Object.<anonymous> (/path/to/file.ts:15:3)` or `src/index.ts:15:3 - error TS2304:`
    let ts_loc_re1 = Regex::new(r"at\s+.*?\((.+?):(\d+):(\d+)\)").unwrap();
    let ts_loc_re2 = Regex::new(r"([^\s:]+\.[a-zA-Z0-9]+):(\d+):(\d+)\s*-\s*error\s*(.+)").unwrap();

    // Generic: `path/to/file.ext:42:15: error:`
    let generic_loc_re = Regex::new(r"([a-zA-Z0-9_\-\\/.]+\.[a-zA-Z0-9]+):(\d+):?(\d+)?:\s*(?:error|warning)?:\s*(.+)").unwrap();

    let mut last_error_msg = String::new();

    for line in &bounded_lines {
        let trimmed = line.trim();

        if let Some(caps) = rust_err_re.captures(trimmed) {
            last_error_msg = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        }

        // Rust location check
        if let Some(caps) = rust_loc_re.captures(trimmed) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col_num: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());

            diagnostics.push(DiagnosticLocation {
                file_path: normalize_path(&file_path),
                line: line_num,
                column: col_num,
                error_type: "RustCompilerError".to_string(),
                message: if last_error_msg.is_empty() { trimmed.to_string() } else { last_error_msg.clone() },
            });
            stack_trace_lines.push(trimmed.to_string());
            continue;
        }

        // Python location check
        if let Some(caps) = py_loc_re.captures(trimmed) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());

            diagnostics.push(DiagnosticLocation {
                file_path: normalize_path(&file_path),
                line: line_num,
                column: None,
                error_type: "PythonTraceback".to_string(),
                message: trimmed.to_string(),
            });
            stack_trace_lines.push(trimmed.to_string());
            continue;
        }

        // TypeScript location check 1 (stack line)
        if let Some(caps) = ts_loc_re1.captures(trimmed) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col_num: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());

            diagnostics.push(DiagnosticLocation {
                file_path: normalize_path(&file_path),
                line: line_num,
                column: col_num,
                error_type: "TypeScriptStack".to_string(),
                message: trimmed.to_string(),
            });
            stack_trace_lines.push(trimmed.to_string());
            continue;
        }

        // TypeScript location check 2 (tsc error)
        if let Some(caps) = ts_loc_re2.captures(trimmed) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col_num: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());
            let msg = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            diagnostics.push(DiagnosticLocation {
                file_path: normalize_path(&file_path),
                line: line_num,
                column: col_num,
                error_type: "TypeScriptCompilerError".to_string(),
                message: msg,
            });
            stack_trace_lines.push(trimmed.to_string());
            continue;
        }

        // Generic location check
        if let Some(caps) = generic_loc_re.captures(trimmed) {
            let file_path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: Option<usize> = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col_num: Option<usize> = caps.get(3).and_then(|m| m.as_str().parse().ok());
            let msg = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            if !file_path.ends_with(".rs") && !file_path.ends_with(".py") && !file_path.ends_with(".ts") {
                diagnostics.push(DiagnosticLocation {
                    file_path: normalize_path(&file_path),
                    line: line_num,
                    column: col_num,
                    error_type: "CompilerError".to_string(),
                    message: msg,
                });
                stack_trace_lines.push(trimmed.to_string());
            }
        }
    }

    let primary_error = diagnostics.first().cloned();
    let clean_snippet = bounded_lines.iter().take(20).cloned().collect::<Vec<&str>>().join("\n");

    TerminalFailureReport {
        command: command.to_string(),
        exit_code,
        clean_stderr_snippet: clean_snippet,
        primary_error,
        all_diagnostics: diagnostics,
        stack_trace_lines,
    }
}

fn normalize_path(raw: &str) -> String {
    let p = PathBuf::from(raw);
    p.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes() {
        let colored = "\x1B[31mError:\x1B[0m cannot find module \x1B[1m'foo'\x1B[0m";
        assert_eq!(strip_ansi(colored), "Error: cannot find module 'foo'");
    }

    #[test]
    fn test_rust_error_interception() {
        let stderr = "\x1B[31merror[E0433]: cannot find crate `dirs` in this scope\x1B[0m\n --> src-tauri/src/commands/airgap.rs:12:16\n  |\n12 | let h = dirs::home();\n";
        let report = process_terminal_failure("cargo build", 101, stderr);

        assert_eq!(report.exit_code, 101);
        assert!(report.primary_error.is_some());
        let diag = report.primary_error.unwrap();
        assert_eq!(diag.file_path, "src-tauri/src/commands/airgap.rs");
        assert_eq!(diag.line, Some(12));
        assert_eq!(diag.column, Some(16));
    }

    #[test]
    fn test_python_traceback_interception() {
        let stderr = "Traceback (most recent call last):\n  File \"src/server.py\", line 45, in handle_request\n    return auth.verify(token)\nKeyError: 'user_id'\n";
        let report = process_terminal_failure("python src/server.py", 1, stderr);

        assert_eq!(report.exit_code, 1);
        assert!(report.primary_error.is_some());
        let diag = report.primary_error.unwrap();
        assert_eq!(diag.file_path, "src/server.py");
        assert_eq!(diag.line, Some(45));
    }

    #[test]
    fn test_typescript_error_interception() {
        let stderr = "src/components/App.tsx:28:10 - error TS2304: Cannot find name 'InvalidSymbol'.";
        let report = process_terminal_failure("tsc", 2, stderr);

        assert!(report.primary_error.is_some());
        let diag = report.primary_error.unwrap();
        assert_eq!(diag.file_path, "src/components/App.tsx");
        assert_eq!(diag.line, Some(28));
        assert_eq!(diag.column, Some(10));
    }
}
