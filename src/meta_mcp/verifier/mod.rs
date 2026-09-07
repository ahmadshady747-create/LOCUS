//! Meta-MCP Tool Security Verifier & AST Capability Analyzer Module.
//!
//! Orchestrates static AST capability analysis, taint checking, synthetic fuzz execution,
//! and cryptographic SHA-256 tool safety certification in pure safe Rust.

#![forbid(unsafe_code)]

pub mod ast_analyzer;
pub mod test_runner;
pub mod types;

pub use ast_analyzer::ToolAstAnalyzer;
pub use test_runner::ToolSyntheticTestRunner;
pub use types::{
    compute_sha256, SecurityPolicyRule, SecurityViolation, SecurityViolationKind,
    ToolSecurityCertificate, ToolVerificationReport,
};

use crate::meta_mcp::sandbox::types::{DynamicToolSpec, ToolCapability};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Central security auditor and certification authority for dynamic Meta-MCP tools.
pub struct ToolSecurityAuditor;

impl ToolSecurityAuditor {
    /// Executes full multi-phase static capability analysis and synthetic sandbox audit.
    pub fn audit_tool(
        spec: &DynamicToolSpec,
        granted_capabilities: &[ToolCapability],
    ) -> ToolVerificationReport {
        let start = Instant::now();

        // Phase 1: Static AST Capability & Taint Analysis
        let mut violations = ToolAstAnalyzer::analyze(&spec.implementation_body, granted_capabilities);

        // Phase 2: Synthetic Fuzz & Sandbox Execution (only if static checks passed)
        if violations.is_empty() {
            let test_violations =
                ToolSyntheticTestRunner::run_synthetic_tests(spec, granted_capabilities);
            violations.extend(test_violations);
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        if violations.is_empty() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let cert = ToolSecurityCertificate::new(
                spec.id.value(),
                &spec.implementation_body,
                granted_capabilities.to_vec(),
                timestamp,
            );

            ToolVerificationReport::passed(cert, elapsed_ms)
        } else {
            ToolVerificationReport::failed(violations, elapsed_ms)
        }
    }
}

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_mcp::sandbox::tool_synthesizer::MetaToolSynthesizer;
    use serde_json::json;

    #[test]
    fn test_valid_text_transformer_tool_passes_audit_and_issues_certificate() {
        let schema = json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": { "type": "string" }
            }
        });

        let safe_body = r#"
            pub fn run(text: &str) -> String {
                text.trim().to_uppercase()
            }
        "#;

        let spec = MetaToolSynthesizer::synthesize_tool(
            "text_normalizer",
            "Trims and uppercases input strings",
            schema,
            vec![ToolCapability::ReadWorkspace],
            safe_body,
        )
        .expect("Synthesis succeeds");

        let granted = vec![ToolCapability::ReadWorkspace];
        let report = ToolSecurityAuditor::audit_tool(&spec, &granted);

        assert!(report.passed, "Clean tool must pass security audit");
        assert!(report.violations.is_empty());
        assert!(report.certificate.is_some());

        let cert = report.certificate.unwrap();
        assert_eq!(cert.tool_id, spec.id.value());
        assert!(!cert.code_sha256.is_empty());
        assert!(cert.issuer_signature.starts_with("LOCUS-VERIFIED:"));
    }

    #[test]
    fn test_process_spawn_attempt_is_strictly_rejected() {
        let schema = json!({ "type": "object" });
        let malicious_body = r#"
            pub fn execute_payload() {
                let _ = std::process::Command::new("rm").arg("-rf").arg("/").spawn();
            }
        "#;

        let spec = MetaToolSynthesizer::synthesize_tool(
            "malicious_spawner",
            "Attempts process spawning",
            schema,
            vec![ToolCapability::ReadWorkspace],
            malicious_body,
        )
        .expect("Synthesis succeeds");

        let granted = vec![ToolCapability::ReadWorkspace];
        let report = ToolSecurityAuditor::audit_tool(&spec, &granted);

        assert!(!report.passed, "Process spawning must be rejected");
        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0].violation_kind,
            SecurityViolationKind::ProcessExecutionAttempt
        );
        assert!(report.certificate.is_none());
    }

    #[test]
    fn test_unauthorized_filesystem_write_is_rejected() {
        let schema = json!({ "type": "object" });
        let file_write_body = r#"
            pub fn write_secret_log(data: &str) {
                let _ = std::fs::write("/etc/config.json", data);
            }
        "#;

        let spec = MetaToolSynthesizer::synthesize_tool(
            "covert_file_writer",
            "Attempts unauthorized file write",
            schema,
            vec![ToolCapability::ReadWorkspace],
            file_write_body,
        )
        .expect("Synthesis succeeds");

        // Grant only ReadWorkspace, NOT WriteWorkspace
        let granted = vec![ToolCapability::ReadWorkspace];
        let report = ToolSecurityAuditor::audit_tool(&spec, &granted);

        assert!(!report.passed, "Unauthorized file write must be rejected");
        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0].violation_kind,
            SecurityViolationKind::UnauthorizedFileAccess
        );
    }

    #[test]
    fn test_sub_0_05ms_tool_verification_budget() {
        let schema = json!({
            "type": "object",
            "properties": {
                "val": { "type": "integer" }
            }
        });

        let safe_body = "pub fn add_one(x: i32) -> i32 { x + 1 }";
        let spec = MetaToolSynthesizer::synthesize_tool(
            "math_incrementer",
            "Increments integers",
            schema,
            vec![ToolCapability::ReadWorkspace],
            safe_body,
        )
        .expect("Synthesis succeeds");

        let granted = vec![ToolCapability::ReadWorkspace];

        let start = Instant::now();
        let iterations = 100;
        for _ in 0..iterations {
            let report = ToolSecurityAuditor::audit_tool(&spec, &granted);
            assert!(report.passed);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / (iterations as f64);

        println!("⚡ Average Meta-MCP Tool Verification Latency: {:.3} µs", avg_us);

        let budget_us = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
        assert!(
            avg_us < budget_us,
            "Tool audit and certification must execute in sub-0.05ms (<50µs in release), got: {:.3} µs",
            avg_us
        );
    }
}
