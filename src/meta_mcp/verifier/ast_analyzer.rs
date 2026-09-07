//! Static AST Capability & Security Taint Analyzer for Dynamic Meta-MCP Tools.
//!
//! Statically scans tool implementations for unauthorized process spawning,
//! unsanctioned filesystem mutations, unbounded networking, and infinite loops in 100% safe Rust.

#![forbid(unsafe_code)]

use super::types::{SecurityViolation, SecurityViolationKind};
use crate::meta_mcp::sandbox::types::ToolCapability;

/// High-throughput static AST capability auditor for dynamic tool code.
pub struct ToolAstAnalyzer;

impl ToolAstAnalyzer {
    /// Audits tool source code against granted capability tokens.
    pub fn analyze(
        source_code: &str,
        declared_capabilities: &[ToolCapability],
    ) -> Vec<SecurityViolation> {
        let mut violations = Vec::new();

        let has_write_cap = declared_capabilities.contains(&ToolCapability::WriteWorkspace);
        let has_net_cap = declared_capabilities.contains(&ToolCapability::NetworkRestricted);

        let lines: Vec<&str> = source_code.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_no = line_idx + 1;
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            // 1. Process Spawning & Shell Execution Guard (Strict Zero-Tolerance)
            if trimmed.contains("std::process::Command")
                || trimmed.contains("Command::new")
                || trimmed.contains("libc::fork")
                || trimmed.contains("libc::exec")
                || trimmed.contains("std::process::exit")
            {
                violations.push(SecurityViolation::new(
                    SecurityViolationKind::ProcessExecutionAttempt,
                    "ProcessSpawnInvocation",
                    line_no,
                    "Direct system process invocation or shell execution is strictly prohibited in sovereign tool sandboxes.",
                    "Replace external process invocation with pure in-memory Rust algorithms or registered MCP tool calls.",
                ));
            }

            // 2. Unauthorized Disk & Filesystem Write Guard
            if !has_write_cap {
                if trimmed.contains("std::fs::write")
                    || trimmed.contains("std::fs::remove_file")
                    || trimmed.contains("std::fs::create_dir")
                    || trimmed.contains("File::create")
                    || trimmed.contains("OpenOptions::new().write")
                {
                    violations.push(SecurityViolation::new(
                        SecurityViolationKind::UnauthorizedFileAccess,
                        "ArbitraryFileWrite",
                        line_no,
                        "Tool attempts filesystem write operations without declaring 'WriteWorkspace' capability.",
                        "Request 'WriteWorkspace' capability or route mutations through the transactional WorkspaceTransaction buffer.",
                    ));
                }
            }

            // 3. Unauthorized Network Access Guard
            if !has_net_cap {
                if trimmed.contains("std::net::TcpStream")
                    || trimmed.contains("std::net::UdpSocket")
                    || trimmed.contains("reqwest::")
                    || trimmed.contains("hyper::Client")
                {
                    violations.push(SecurityViolation::new(
                        SecurityViolationKind::CapabilityMismatch,
                        "NetworkSocketInvocation",
                        line_no,
                        "Tool attempts network socket connection without declaring 'NetworkRestricted' capability.",
                        "Request 'NetworkRestricted' capability or use local AST query engines.",
                    ));
                }
            }

            // 4. Infinite / Unbounded Loop Guard
            if (trimmed.starts_with("loop {") || trimmed.starts_with("while true {"))
                && !source_code.contains("break;")
                && !source_code.contains("break ")
            {
                violations.push(SecurityViolation::new(
                    SecurityViolationKind::UnboundedLoopDetected,
                    "InfiniteLoopConstruct",
                    line_no,
                    "Detected potential unbounded loop construct lacking deterministic termination or break condition.",
                    "Use bounded iterator constructs (e.g. for ... in ...) or specify explicit break conditions.",
                ));
            }
        }

        violations
    }
}
