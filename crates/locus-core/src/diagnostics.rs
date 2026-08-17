//! Diagnostic Log Exporter for LOCUS
//!
//! Provides anonymous, privacy-safe system diagnostics collection
//! for technical support and issue reporting. All sensitive data
//! (API keys, IP addresses, user paths) is automatically redacted.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Compiled regex patterns for secret/PII redaction (computed once).
static RE_API_KEYS: OnceLock<Regex> = OnceLock::new();
static RE_IP_ADDR: OnceLock<Regex> = OnceLock::new();
static RE_WIN_PATH: OnceLock<Regex> = OnceLock::new();
static RE_NIX_PATH: OnceLock<Regex> = OnceLock::new();

fn re_api_keys() -> &'static Regex {
    RE_API_KEYS.get_or_init(|| {
        Regex::new(
            r"(?i)(sk-[A-Za-z0-9_\-]{16,}|ghp_[A-Za-z0-9]{20,}|key-[A-Za-z0-9_\-]{16,}|bearer\s+[A-Za-z0-9_\-\.]{16,}|api[_-]?key\s*[:=]\s*[^\s]+|gsk_[A-Za-z0-9_\-]{20,}|AIza[A-Za-z0-9_\-]{30,})"
        ).unwrap()
    })
}

fn re_ip_addr() -> &'static Regex {
    RE_IP_ADDR.get_or_init(|| {
        Regex::new(
            r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b"
        ).unwrap()
    })
}

fn re_win_path() -> &'static Regex {
    RE_WIN_PATH.get_or_init(|| {
        Regex::new(r"(?i)(Users\\[^\\]+)").unwrap()
    })
}

fn re_nix_path() -> &'static Regex {
    RE_NIX_PATH.get_or_init(|| {
        Regex::new(r"(/home/[^/]+)").unwrap()
    })
}

/// Core diagnostic report structure.
///
/// All fields are designed to be privacy-safe by default.
/// Sensitive fields like API keys and user paths are never collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// Unique identifier for this diagnostic snapshot.
    pub snapshot_id: String,
    /// ISO-8601 timestamp when the snapshot was created.
    pub created_at: String,
    /// LOCUS application version string.
    pub app_version: String,
    /// Anonymous system hardware/OS information.
    pub system: SystemInfo,
    /// Summary of the current workspace state (no file contents).
    pub workspace: WorkspaceInfo,
    /// AI engine and model routing status (no API keys).
    pub ai_engine: AiEngineInfo,
    /// P2P mesh network status summary.
    pub mesh: MeshInfo,
    /// Sandboxed agent pool status.
    pub agents: AgentsInfo,
    /// Privacy-sanitized log entries for debugging.
    pub logs: Vec<SanitizedLogEntry>,
}

/// Anonymous system hardware and OS information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub logical_cpus: usize,
    /// Approximate total RAM in GB (rounded, not exact).
    pub approx_ram_gb: Option<f64>,
}

/// Workspace index summary (never includes file contents or full paths).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub loaded: bool,
    pub total_files: usize,
    pub total_size_bytes: u64,
}

/// AI engine routing information (never includes API keys).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEngineInfo {
    pub selected_model: Option<String>,
    pub local_models_count: usize,
    pub local_model_names: Vec<String>,
    pub fallback_strategy: String,
    pub fallback_enabled: bool,
    pub active_targets: Vec<String>,
    /// Cloud providers with keys configured (names only, never the keys themselves).
    pub configured_providers: Vec<String>,
}

/// P2P mesh network discovery status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshInfo {
    pub running: bool,
    pub peer_count: usize,
}

/// Agent sandbox pool status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsInfo {
    pub active_count: usize,
    pub max_memory_mb: u64,
}

/// A single sanitized diagnostic log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedLogEntry {
    pub timestamp: String,
    pub level: String,
    pub subsystem: String,
    pub message: String,
}

/// Sanitizes arbitrary text by redacting secrets, IPs, and user paths.
///
/// # What gets redacted:
/// - API keys matching common patterns (OpenAI `sk-`, GitHub `ghp_`, Groq `gsk_`, Google `AIza`)
/// - Bearer tokens
/// - Non-localhost IP addresses
/// - Windows user directory paths
/// - Unix home directory paths
pub fn sanitize_text(input: &str) -> String {
    let mut text = input.to_string();

    // 1. Redact API Keys / Tokens
    text = re_api_keys().replace_all(&text, "[REDACTED_API_KEY]").to_string();

    // 2. Redact non-localhost IP addresses (preserve 127.0.0.1 and 0.0.0.0)
    let re_ip = re_ip_addr();
    text = re_ip.replace_all(&text, |caps: &regex::Captures| {
        let matched = caps.get(0).unwrap().as_str();
        if matched == "127.0.0.1" || matched == "0.0.0.0" {
            matched.to_string()
        } else {
            "[REDACTED_IP]".to_string()
        }
    }).to_string();

    // 3. Redact Windows user paths
    text = re_win_path().replace_all(&text, "Users\\[USER]").to_string();

    // 4. Redact Unix user paths
    text = re_nix_path().replace_all(&text, "/home/[USER]").to_string();

    text
}

/// Collects anonymous system information.
pub fn collect_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        approx_ram_gb: None,
    }
}

/// Generates a diagnostic summary string from a [`DiagnosticSnapshot`].
pub fn format_summary(snap: &DiagnosticSnapshot) -> String {
    format!(
        "LOCUS {} | {} {} | {} CPUs | {} files ({} bytes) | {} local models | Strategy: {} | {} cloud providers | {} agents active",
        snap.app_version,
        snap.system.os,
        snap.system.arch,
        snap.system.logical_cpus,
        snap.workspace.total_files,
        snap.workspace.total_size_bytes,
        snap.ai_engine.local_models_count,
        snap.ai_engine.fallback_strategy,
        snap.ai_engine.configured_providers.len(),
        snap.agents.active_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_api_keys() {
        let input = "Using key sk-abc1234567890abcdef1234 for generation";
        let output = sanitize_text(input);
        assert!(output.contains("[REDACTED_API_KEY]"), "Output was: {}", output);
        assert!(!output.contains("sk-abc"), "Output was: {}", output);
    }

    #[test]
    fn test_sanitize_groq_key() {
        let input = "Groq API: gsk_abcdefghijklmnopqrstuvwxyz";
        let output = sanitize_text(input);
        assert!(output.contains("[REDACTED_API_KEY]"), "Output was: {}", output);
        assert!(!output.contains("gsk_"), "Output was: {}", output);
    }

    #[test]
    fn test_sanitize_google_key() {
        let input = "Google API: AIzaSyB1234567890abcdefghijklmnopqrstuv";
        let output = sanitize_text(input);
        assert!(output.contains("[REDACTED_API_KEY]"), "Output was: {}", output);
        assert!(!output.contains("AIza"), "Output was: {}", output);
    }

    #[test]
    fn test_sanitize_ip_addresses() {
        let input = "Connected to 10.0.1.42 and localhost 127.0.0.1";
        let output = sanitize_text(input);
        assert!(output.contains("[REDACTED_IP]"), "Output was: {}", output);
        assert!(output.contains("127.0.0.1"), "Localhost should be preserved. Output: {}", output);
        assert!(!output.contains("10.0.1.42"), "Output was: {}", output);
    }

    #[test]
    fn test_sanitize_windows_paths() {
        let input = r"Config at C:\Users\alice\AppData\LOCUS";
        let output = sanitize_text(input);
        assert!(output.contains(r"Users\[USER]"), "Output was: {}", output);
        assert!(!output.contains("alice"), "Output was: {}", output);
    }

    #[test]
    fn test_sanitize_unix_paths() {
        let input = "Home dir: /home/developer/.config/locus";
        let output = sanitize_text(input);
        assert!(output.contains("/home/[USER]"), "Output was: {}", output);
        assert!(!output.contains("developer"), "Output was: {}", output);
    }

    #[test]
    fn test_collect_system_info() {
        let info = collect_system_info();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.logical_cpus >= 1);
    }

    #[test]
    fn test_format_summary() {
        let snap = DiagnosticSnapshot {
            snapshot_id: "test-id".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            app_version: "v0.1.0-alpha".to_string(),
            system: SystemInfo {
                os: "windows".to_string(),
                arch: "x86_64".to_string(),
                family: "windows".to_string(),
                logical_cpus: 8,
                approx_ram_gb: Some(16.0),
            },
            workspace: WorkspaceInfo {
                loaded: true,
                total_files: 150,
                total_size_bytes: 5_000_000,
            },
            ai_engine: AiEngineInfo {
                selected_model: Some("llama3".to_string()),
                local_models_count: 2,
                local_model_names: vec!["llama3".to_string(), "codellama".to_string()],
                fallback_strategy: "LocalFirst".to_string(),
                fallback_enabled: true,
                active_targets: vec!["ollama".to_string(), "groq".to_string()],
                configured_providers: vec!["groq".to_string()],
            },
            mesh: MeshInfo {
                running: false,
                peer_count: 0,
            },
            agents: AgentsInfo {
                active_count: 0,
                max_memory_mb: 256,
            },
            logs: vec![],
        };

        let summary = format_summary(&snap);
        assert!(summary.contains("v0.1.0-alpha"));
        assert!(summary.contains("windows"));
        assert!(summary.contains("150 files"));
        assert!(summary.contains("2 local models"));
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.abcdef1234";
        let output = sanitize_text(input);
        assert!(output.contains("[REDACTED_API_KEY]"), "Output was: {}", output);
        assert!(!output.contains("eyJhbGciOiJ"), "Output was: {}", output);
    }
}
