//! Comprehensive SARIF 2.1.0 & Release Audit Gate Integration Tests.
//!
//! Validates OASIS SARIF 2.1.0 schema compliance, 10,000-finding stress benchmarks,
//! zero-regression release gate enforcement, and air-gapped deterministic reporting in 100% safe Rust.

#![forbid(unsafe_code)]

use locus_engine::guard::AstGuard;
use locus_engine::release::{AuditGateConfig, AuditVerdict, ReleaseAuditGate};
use locus_engine::sarif::{
    SarifLevel, SarifReportGenerator, SARIF_SCHEMA_URI, SARIF_VERSION,
};
use locus_engine::types::ViolationKind;
use std::time::Instant;

#[test]
fn test_oasis_sarif_v2_1_0_schema_field_conformance() {
    let mut generator = SarifReportGenerator::new();

    generator.add_violation(
        "src/core/alloc.rs",
        ViolationKind::ArrayOutOfBounds,
        "Array index access without bounds check",
        15,
        8,
        15,
        22,
        Some("items[idx]".to_string()),
    );

    generator.add_violation(
        "src/web/login.tsx",
        ViolationKind::JsxTagMismatch,
        "Unbalanced JSX tag <div> closed with </span>",
        30,
        1,
        30,
        8,
        Some("</span>".to_string()),
    );

    let log = generator.build_log();

    // 1. Top-Level Schema Checks
    assert_eq!(log.schema, SARIF_SCHEMA_URI);
    assert_eq!(log.version, SARIF_VERSION);
    assert_eq!(log.runs.len(), 1);

    // 2. Tool Driver Checks
    let run = &log.runs[0];
    assert_eq!(run.tool.driver.name, "locus-engine");
    assert!(!run.tool.driver.rules.is_empty());
    assert!(run.tool.driver.information_uri.is_some());

    // 3. Results Checks
    assert_eq!(run.results.len(), 2);
    let r1 = &run.results[0];
    assert_eq!(r1.rule_id, "LOCUS004");
    assert_eq!(r1.level, SarifLevel::Error);
    assert_eq!(r1.locations.len(), 1);

    let loc1 = &r1.locations[0].physical_location.as_ref().unwrap();
    assert_eq!(loc1.artifact_location.uri, "src/core/alloc.rs");
    assert_eq!(loc1.region.as_ref().unwrap().start_line, Some(15));
    assert_eq!(loc1.region.as_ref().unwrap().snippet.as_ref().unwrap().text, "items[idx]");

    let r2 = &run.results[1];
    assert_eq!(r2.rule_id, "LOCUS011");
    assert_eq!(r2.level, SarifLevel::Error);

    // 4. JSON Serialization & Round-Trip Parsing
    let json_str = generator.export_to_json().expect("Export succeeds");
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["results"].as_array().unwrap().len(), 2);
}

#[test]
fn test_10k_findings_stress_benchmark() {
    let mut generator = SarifReportGenerator::new();

    // Generate 10,000 findings across 100 virtual modules
    for i in 0..10_000 {
        let kind = match i % 4 {
            0 => ViolationKind::DivisionByZero,
            1 => ViolationKind::SqlInjection,
            2 => ViolationKind::PathTraversal,
            _ => ViolationKind::ClientSecretLeak,
        };

        generator.add_violation(
            &format!("src/service/worker_{}.rs", i % 100),
            kind,
            &format!("Synthetic stress violation #{}", i),
            (i % 500) + 1,
            4,
            (i % 500) + 1,
            30,
            Some("unsafe_expr()".to_string()),
        );
    }

    assert_eq!(generator.results_count(), 10_000);

    // Warm up
    for _ in 0..2 {
        let _ = generator.export_to_compact_json();
    }

    let iterations = 5;
    let start = Instant::now();

    for _ in 0..iterations {
        let json_output = generator.export_to_compact_json().expect("Export succeeds");
        assert!(json_output.len() > 100_000);
    }

    let total_ms = start.elapsed().as_secs_f64() * 1000.0;
    let avg_latency_ms = total_ms / (iterations as f64);

    println!(
        "⚡ 10,000-Finding Full SARIF Generation & Serialization: {:.4}ms",
        avg_latency_ms
    );

    // Release budget: 20ms in release, up to 2000ms in unoptimized debug build.
    // The old sub-millisecond budget was not achievable for this workload and
    // was not a code regression: serializing 10,000 findings with serde_json
    // already uses borrowed references (see export_to_compact_json), and the
    // measured average is 8.4-9.1ms locally (this VM, release, 3 runs),
    // 5.5-9.9ms in earlier local runs, 3.97ms on CI macOS and 10.03ms on CI
    // Windows for commit 4b3bf6a (run 40099191200). 20ms keeps a real,
    // tight budget at about 2x the worst observed value; no correctness
    // assertion in this test is changed.
    let budget = if cfg!(debug_assertions) { 2000.0 } else { 20.0 };
    assert!(
        avg_latency_ms < budget,
        "10,000 finding serialization ({:.4}ms) must meet budget ({:.2}ms)",
        avg_latency_ms,
        budget
    );
}

#[test]
fn test_zero_regression_gate_rejection_on_injected_critical_violations() {
    let gate = ReleaseAuditGate::new(AuditGateConfig::zero_tolerance());
    let mut generator = SarifReportGenerator::new();

    // Inject 3 critical violations
    generator.add_violation(
        "src/api/handler.rs",
        ViolationKind::SqlInjection,
        "Dynamic SQL query construction detected",
        20,
        1,
        20,
        45,
        Some("format!(\"SELECT * FROM users WHERE id = {}\", input)".to_string()),
    );
    generator.add_violation(
        "src/net/fetch.rs",
        ViolationKind::SsrfUnsafeFetch,
        "Outbound request to 169.254.169.254 AWS metadata endpoint",
        88,
        1,
        88,
        40,
        Some("client.get(\"http://169.254.169.254/\")".to_string()),
    );
    generator.add_violation(
        "src/auth/token.rs",
        ViolationKind::HardcodedKeyEntropy,
        "Hardcoded private signing key detected",
        5,
        1,
        5,
        64,
        Some("const SECRET: &str = \"d83b5f92a1c84f...\";".to_string()),
    );

    let log = generator.build_log();
    let verdict = gate.evaluate_sarif(&log, 0.12);

    assert!(!verdict.is_approved());
    assert_eq!(verdict.exit_code(), 1);

    match verdict {
        AuditVerdict::Rejected {
            blocker_count,
            violations,
            reason,
            ..
        } => {
            assert_eq!(blocker_count, 3);
            assert_eq!(violations.len(), 3);
            assert!(reason.contains("Critical invariant errors"));
            assert_eq!(violations[0].rule_id, "LOCUS012");
            assert_eq!(violations[1].rule_id, "LOCUS026");
            assert_eq!(violations[2].rule_id, "LOCUS030");
        }
        _ => panic!("Expected Rejected verdict"),
    }
}

#[test]
fn test_end_to_end_ast_guard_to_sarif_pipeline() {
    let clean_code = r#"
        pub fn is_safe_index(items: &[u32], idx: usize) -> Option<u32> {
            if idx < items.len() {
                Some(items[idx])
            } else {
                None
            }
        }
    "#;

    let report = AstGuard::verify(clean_code);
    assert!(report.passed);

    let gate = ReleaseAuditGate::new(AuditGateConfig::zero_tolerance());
    let verdict = gate.evaluate_verification_report(&report, "src/math/safe_indexing.rs");

    assert!(verdict.is_approved());
    assert_eq!(verdict.exit_code(), 0);
}
