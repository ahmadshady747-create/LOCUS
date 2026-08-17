//! End-to-End Integration & Benchmark Suite for Ambient Agent & Quick Verifier Bridge.
//!
//! Validates autonomous code translation, bounds repair, and formal proofs (<=50ms).

use locus_agents::AmbientAgentEngine;
use locus_core::QuickVerifierBridge;
use std::time::Instant;

#[tokio::test]
async fn test_ambient_verifier_pipeline_e2e() {
    // 1. Benchmark Quick Verifier on Safe Function (<=50ms)
    let safe_snippet = r#"
pub fn safe_divide(a: f64, b: f64) -> Option<f64> {
    if b != 0.0 {
        Some(a / b)
    } else {
        None
    }
}
"#;

    let start_verify = Instant::now();
    let report_safe = QuickVerifierBridge::verify_expression_or_function("safe_divide", Some(safe_snippet));
    let verify_elapsed = start_verify.elapsed().as_nanos() as f64 / 1_000_000.0;

    assert!(report_safe.is_safe);
    assert_eq!(report_safe.forward_safety_score, 100.0);
    assert!(verify_elapsed < 50.0, "Verification must be <=50ms (got {:.2}ms)", verify_elapsed);

    // 2. Counterexample Discovery on Unsafe Function
    let unsafe_snippet = r#"
pub fn unsafe_divide(a: i32, b: i32) -> i32 {
    a / b
}
"#;

    let report_unsafe = QuickVerifierBridge::verify_expression_or_function("unsafe_divide", Some(unsafe_snippet));
    assert!(!report_unsafe.is_safe);
    assert!(report_unsafe.counterexample.is_some());
    assert!(report_unsafe.counterexample.unwrap().contains("Division-by-zero"));

    // 3. Autonomous Ambient Action + Self-Verification Pipeline
    let python_code = r#"
def compute_percentage(part, total):
    return (part * 100) / total
"#;

    let start_action = Instant::now();
    let action_result = AmbientAgentEngine::execute_ambient_action(
        "حوّل هذه الدالة إلى Rust",
        Some(python_code),
    )
    .await
    .expect("Ambient action execution failed");
    let action_elapsed = start_action.elapsed().as_nanos() as f64 / 1_000_000.0;

    assert!(action_result.generated_patch.is_some());
    let patch = action_result.generated_patch.unwrap();
    assert!(patch.contains("pub fn compute_percentage"));
    assert!(patch.contains("total != 0"));
    assert!(action_result.verification_passed, "Auto-generated patch must pass formal verification");
    assert!(action_elapsed < 100.0, "E2E action pipeline must finish in <100ms (got {:.2}ms)", action_elapsed);
}
