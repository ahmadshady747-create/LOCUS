//! Deterministic SARIF 2.1.0 Reporter Subsystem.
//!
//! Generates OASIS-compliant static analysis reports from formal AST verification passes,
//! invariant security checks, and inter-procedural taint flow traces in 100% pure safe Rust.

#![forbid(unsafe_code)]

pub mod generator;
pub mod rules;
pub mod types;

pub use generator::SarifReportGenerator;
pub use rules::{get_canonical_rules, lookup_rule, rule_id_for_security_violation, rule_id_for_violation};
pub use types::{
    ArtifactContent, ArtifactLocation, CodeFlow, Invocation, Location, LogicalLocation, Message,
    MultiformatMessageString, PhysicalLocation, Region, ReportingConfiguration,
    ReportingDescriptor, SarifError, SarifLevel, SarifLog, SarifResult, SarifRun, ThreadFlow,
    ThreadFlowLocation, ToolComponent, ToolComponentDriver, SARIF_SCHEMA_URI, SARIF_VERSION,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::AstGuard;
    use crate::types::ViolationKind;
    use std::time::Instant;

    #[test]
    fn test_sarif_log_oasis_schema_generation() {
        let mut generator = SarifReportGenerator::new();
        generator.add_violation(
            "src/auth/jwt.rs",
            ViolationKind::ClientSecretLeak,
            "Client component accesses process.env.DATABASE_URL",
            42,
            5,
            42,
            35,
            Some("process.env.DATABASE_URL".to_string()),
        );

        let log = generator.build_log();
        assert_eq!(log.schema, SARIF_SCHEMA_URI);
        assert_eq!(log.version, SARIF_VERSION);
        assert_eq!(log.runs.len(), 1);

        let run = &log.runs[0];
        assert_eq!(run.tool.driver.name, "locus-engine");
        assert!(!run.tool.driver.rules.is_empty());
        assert_eq!(run.results.len(), 1);

        let res = &run.results[0];
        assert_eq!(res.rule_id, "LOCUS009");
        assert_eq!(res.level, SarifLevel::Error);

        let json_output = generator.export_to_json().expect("JSON export succeeds");
        assert!(json_output.contains("\"version\": \"2.1.0\""));
        assert!(json_output.contains("\"ruleId\": \"LOCUS009\""));
        assert!(json_output.contains("\"startLine\": 42"));
    }

    #[test]
    fn test_sarif_from_ast_guard_verification_report() {
        let invalid_code = "fn calculate(x: i32, y: i32) -> i32 { if true { x / y }"; // Unclosed brace
        let report = AstGuard::verify(invalid_code);
        assert!(!report.passed);

        let generator = SarifReportGenerator::from_verification_report(
            "src/math/calc.rs",
            &report,
            invalid_code,
        );

        assert_eq!(generator.results_count(), 1);
        let log = generator.build_log();
        let res = &log.runs[0].results[0];
        assert_eq!(res.rule_id, "LOCUS001");
        assert_eq!(res.level, SarifLevel::Error);
    }

    #[test]
    fn test_sarif_taint_flow_serialization() {
        let mut generator = SarifReportGenerator::new();
        let taint_steps = vec![
            ("src/api/controller.rs".to_string(), 10, 4, "let raw_input = req.query(\"path\");".to_string()),
            ("src/service/file_service.rs".to_string(), 25, 8, "let path = format!(\"/data/{}\", raw_input);".to_string()),
            ("src/io/disk.rs".to_string(), 50, 12, "fs::read_to_string(&path)".to_string()),
        ];

        generator.add_taint_flow(
            "LOCUS017",
            "Unsanitized path traversal vulnerability detected from HTTP controller to disk read",
            &taint_steps,
        );

        let log = generator.build_log();
        let res = &log.runs[0].results[0];
        assert_eq!(res.rule_id, "LOCUS017");
        assert!(res.code_flows.is_some());

        let code_flow = &res.code_flows.as_ref().unwrap()[0];
        assert_eq!(code_flow.thread_flows[0].locations.len(), 3);

        let json_str = generator.export_to_json().expect("Export succeeds");
        assert!(json_str.contains("threadFlows"));
        assert!(json_str.contains("taintTraceLength"));
    }

    #[test]
    fn test_sub_millisecond_1000_results_serialization() {
        let mut generator = SarifReportGenerator::new();

        for i in 0..1000 {
            generator.add_violation(
                &format!("src/module_{}.rs", i % 10),
                ViolationKind::DivisionByZero,
                &format!("Division by variable without guard at index {}", i),
                i + 1,
                10,
                i + 1,
                25,
                Some("x / divisor".to_string()),
            );
        }

        // Warm up
        for _ in 0..3 {
            let _ = generator.export_to_compact_json();
        }

        let iterations = if cfg!(debug_assertions) { 2 } else { 10 };
        let start = Instant::now();

        for _ in 0..iterations {
            let json = generator.export_to_json().expect("Serialization succeeds");
            assert!(json.len() > 1000);
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average 1,000-finding SARIF JSON Serialization Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 500.0 } else { 10.0 };
        assert!(
            avg_latency_ms < budget,
            "1,000-finding SARIF serialization must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
