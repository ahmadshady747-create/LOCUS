//! Comprehensive Semantic Prover & Tool #30 Integration Tests.
//!
//! Validates formal proof card synthesis, WP calculus derivations, counterexample generation,
//! auto-fix correctness, and MCP Tool #30 JSON-RPC execution under 1.2ms in 100% safe pure Rust.

#![forbid(unsafe_code)]

use locus_engine::cst::polyglot::types::LanguageId;
use locus_engine::mcp::handle_json_rpc_message;
use locus_engine::sarif::SarifLevel;
use locus_engine::semantic::{LocusSemanticAnalyzer, SemanticSarifBridge};
use serde_json::Value;
use std::time::Instant;

#[test]
fn test_semantic_prover_multi_hazard_proof_card_generation() {
    let analyzer = LocusSemanticAnalyzer::new();

    let complex_source = r#"
        import { process } from './worker';

        export async function executeTaskBatch(items: number[]) {
            // S1 Violation: Async in forEach
            items.forEach(async (x) => {
                await process(x);
            });

            // S2 Violation: Object literal in Map key
            const sessionMap = new Map();
            sessionMap.set({ sessionId: 1001 }, { user: "Alice" });

            // S3 Violation: Numeric sort without comparator
            const scores = [100, 25, 5, 10];
            scores.sort();

            // S4 Violation: Date prototype loss via JSON serialization
            const auditEvent = { timestamp: new Date(), action: "LOGIN" };
            const copiedEvent = JSON.parse(JSON.stringify(auditEvent));

            // S5 Violation: Calendar month 12 out-of-bounds comparison
            const now = new Date();
            if (now.getMonth() === 12) {
                console.log("End of year reconciliation");
            }
        }
    "#;

    let report = analyzer.analyze_source(complex_source, LanguageId::TypeScript);

    assert_eq!(report.violations_count, 5);
    assert_eq!(report.proofs.len(), 5);
    assert!(!report.passed);

    // Verify S1 Proof Card
    let s1_proof = report.proofs.iter().find(|p| p.rule_id == "S1").expect("S1 proof");
    assert_eq!(s1_proof.rule_name, "AsyncIteratorRace");
    assert!(s1_proof.mathematical_proof.contains("wp(forEach"));
    assert!(s1_proof.counterexample.as_ref().unwrap().contains("items.forEach"));

    // Verify S2 Proof Card
    let s2_proof = report.proofs.iter().find(|p| p.rule_id == "S2").expect("S2 proof");
    assert_eq!(s2_proof.rule_name, "MapObjectKeyIdentity");
    assert!(s2_proof.mathematical_proof.contains("SameValueZero"));
    assert!(s2_proof.counterexample.as_ref().unwrap().contains("map.set"));

    // Verify S3 Proof Card
    let s3_proof = report.proofs.iter().find(|p| p.rule_id == "S3").expect("S3 proof");
    assert_eq!(s3_proof.rule_name, "NumericSortLexicographic");
    assert!(s3_proof.mathematical_proof.contains("Sort_lex"));
    assert!(s3_proof.remediation_patch.contains("(a, b) => a - b"));

    // Verify S4 Proof Card
    let s4_proof = report.proofs.iter().find(|p| p.rule_id == "S4").expect("S4 proof");
    assert_eq!(s4_proof.rule_name, "DatePrototypeLossViaJson");
    assert!(s4_proof.mathematical_proof.contains("JSON.stringify"));

    // Verify S5 Proof Card
    let s5_proof = report.proofs.iter().find(|p| p.rule_id == "S5").expect("S5 proof");
    assert_eq!(s5_proof.rule_name, "CalendarArithmeticMismatch");
    assert!(s5_proof.mathematical_proof.contains("getMonth"));
    assert!(s5_proof.counterexample.as_ref().unwrap().contains("d.getMonth() is 11"));
}

#[test]
fn test_counterexample_and_patch_correctness() {
    let analyzer = LocusSemanticAnalyzer::new();

    // Source with single numeric sort hazard
    let hazardous_code = r#"
        function sortMetrics(data: number[]): number[] {
            return data.sort();
        }
    "#;

    let report = analyzer.analyze_source(hazardous_code, LanguageId::TypeScript);
    assert_eq!(report.violations_count, 1);

    let proof = &report.proofs[0];
    assert_eq!(proof.rule_id, "S3");
    assert!(proof.remediation_patch.contains("(a, b) => a - b"));

    // Replace with remediated safe code
    let safe_code = r#"
        function sortMetrics(data: number[]): number[] {
            return data.sort((a, b) => a - b);
        }
    "#;

    let clean_report = analyzer.analyze_source(safe_code, LanguageId::TypeScript);
    assert!(clean_report.passed);
    assert_eq!(clean_report.violations_count, 0);
}

#[test]
fn test_mcp_tool_30_semantic_analyze_json_rpc() {
    let payload = r#"{
        "jsonrpc": "2.0",
        "id": "sem-30-test",
        "method": "tools/call",
        "params": {
            "name": "semantic_analyze",
            "arguments": {
                "code": "function run(list: string[]) { const nums = [5, 10]; nums.sort(); const d = new Date(); if (d.getMonth() === 12) {} }",
                "language": "typescript",
                "sarif": true
            }
        }
    }"#;

    let resp_str = handle_json_rpc_message(payload).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON-RPC");

    assert_eq!(resp["id"], "sem-30-test");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text response");

    let parsed_result: Value = serde_json::from_str(text).expect("Valid tool payload");
    assert_eq!(parsed_result["status"], "violations_detected");
    assert_eq!(parsed_result["passed"], false);
    assert_eq!(parsed_result["violations_count"], 2);

    let proofs = parsed_result["proofs"].as_array().expect("Proofs array");
    assert_eq!(proofs.len(), 2);

    // Verify SARIF log attachment
    assert!(parsed_result["sarifLog"].is_object());
    assert_eq!(parsed_result["sarifLog"]["version"], "2.1.0");
    assert_eq!(parsed_result["sarifLog"]["runs"][0]["results"].as_array().unwrap().len(), 2);
}

#[test]
fn test_sarif_bridge_schema_conformance() {
    let analyzer = LocusSemanticAnalyzer::new();
    let source = "const m = new Map(); m.set({ id: 42 }, 'token');";
    let report = analyzer.analyze_source(source, LanguageId::TypeScript);

    let sarif_log = SemanticSarifBridge::to_sarif_log(&report, "src/auth/session.ts");
    assert_eq!(sarif_log.version, "2.1.0");
    assert_eq!(sarif_log.runs.len(), 1);

    let run = &sarif_log.runs[0];
    assert_eq!(run.tool.driver.name, "locus-engine");
    assert_eq!(run.results.len(), 1);

    let result = &run.results[0];
    assert_eq!(result.rule_id, "LOCUS_S2");
    assert_eq!(result.level, SarifLevel::Error);

    let loc = &result.locations[0].physical_location.as_ref().unwrap();
    assert_eq!(loc.artifact_location.uri, "src/auth/session.ts");
    assert!(loc.region.is_some());

    let json_output = SemanticSarifBridge::export_to_json(&report, "src/auth/session.ts")
        .expect("Serialization succeeds");
    assert!(json_output.contains("LOCUS_S2"));
    assert!(json_output.contains("Mathematical Proof"));
}

#[test]
fn test_sub_1_2ms_semantic_prover_latency_budget() {
    let analyzer = LocusSemanticAnalyzer::new();

    // 1,000-line realistic module
    let mut module_code = String::with_capacity(50_000);
    for i in 0..200 {
        module_code.push_str(&format!(
            "export function handler_{}(input: number): number {{\n    const factor = {};\n    return input * factor;\n}}\n",
            i, i
        ));
    }
    module_code.push_str("export function finalize() { const items = [1, 2]; items.forEach(async (x) => { await compute(x); }); }\n");

    // Warm up
    for _ in 0..3 {
        let _ = analyzer.analyze_source(&module_code, LanguageId::TypeScript);
    }

    let iterations = 20;
    let start = Instant::now();

    for _ in 0..iterations {
        let report = analyzer.analyze_source(&module_code, LanguageId::TypeScript);
        assert_eq!(report.violations_count, 1);
    }

    let total_ms = start.elapsed().as_secs_f64() * 1000.0;
    let avg_latency_ms = total_ms / (iterations as f64);

    println!("⚡ Average Semantic Prover Execution Latency: {:.4}ms", avg_latency_ms);

    let budget = if cfg!(debug_assertions) { 50.0 } else { 1.2 };
    assert!(
        avg_latency_ms < budget,
        "Semantic prover latency ({:.4}ms) must meet budget ({:.2}ms)",
        avg_latency_ms,
        budget
    );
}
