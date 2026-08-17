//! Integration test suite for QuickVerifierBridge.

use locus_core::QuickVerifierBridge;

#[test]
fn test_verify_safe_function() {
    let safe_code = r#"
pub fn calculate_safe_ratio(total: f64, count: f64) -> Option<f64> {
    if count != 0.0 {
        Some(total / count)
    } else {
        None
    }
}
"#;

    let report = QuickVerifierBridge::verify_expression_or_function("calculate_safe_ratio", Some(safe_code));
    assert!(report.is_safe, "Guarded division function must be verified safe");
    assert_eq!(report.forward_safety_score, 100.0);
    assert_eq!(report.backward_intent_score, 100.0);
    assert!(report.counterexample.is_none());
    assert!(report.execution_time_ms < 50.0, "Verification must complete in <=50ms");
}

#[test]
fn test_verify_div_by_zero_counterexample() {
    let unsafe_code = r#"
pub fn compute_average(sum: i32, n: i32) -> i32 {
    sum / n
}
"#;

    let report = QuickVerifierBridge::verify_expression_or_function("compute_average", Some(unsafe_code));
    assert!(!report.is_safe, "Unguarded division by `n` must be flagged unsafe");
    assert!(report.counterexample.is_some());
    let ce = report.counterexample.unwrap();
    assert!(ce.contains("Division-by-zero"), "Counterexample must mention division by zero");
    assert!(ce.contains("n = 0") || ce.contains("n"));
}

#[test]
fn test_verify_unsafe_unwrap_counterexample() {
    let unsafe_unwrap = r#"
pub fn parse_and_get(s: &str) -> i32 {
    s.parse::<i32>().unwrap()
}
"#;

    let report = QuickVerifierBridge::verify_expression_or_function("parse_and_get", Some(unsafe_unwrap));
    assert!(!report.is_safe);
    assert!(report.counterexample.is_some());
    assert!(report.counterexample.unwrap().contains("Unwrap counterexample"));
}

#[test]
fn test_verify_unsafe_array_bounds_counterexample() {
    let unsafe_index = r#"
pub fn get_element(arr: &[i32], idx: usize) -> i32 {
    arr[idx]
}
"#;

    let report = QuickVerifierBridge::verify_expression_or_function("get_element", Some(unsafe_index));
    assert!(!report.is_safe);
    assert!(report.counterexample.is_some());
    assert!(report.counterexample.unwrap().contains("Out-of-bounds counterexample"));
}
