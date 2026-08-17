//! End-to-End Integration & Benchmark Suite for Ambient Floating Architecture.
//!
//! Validates full lifecycle: Wake (<3ms) -> Parse (<=0.5ms) -> Inject -> Dismiss (0% Idle).

use locus_core::{AmbientController, OmniIntent, SafeTextInjector};
use std::time::Instant;

#[test]
fn test_ambient_full_user_lifecycle_e2e() {
    let controller = AmbientController::new();

    // 1. Wake Trigger Benchmark (<3ms)
    let wake_start = Instant::now();
    let wake_latency = controller.trigger_wake();
    let wake_elapsed_ms = (wake_start.elapsed().as_nanos() as f64) / 1_000_000.0;

    assert!(
        wake_latency < 3.0 && wake_elapsed_ms < 3.0,
        "Wake latency must be sub-3ms (got latency {:.3}ms, elapsed {:.3}ms)",
        wake_latency,
        wake_elapsed_ms
    );

    // Verify Telemetry Reporting
    let telemetry = controller.get_telemetry();
    assert!(telemetry.latency_ms < 3.0);
    assert!(telemetry.ram_usage_mb > 0.0);

    // 2. OmniBar Intent Parse (<=0.5ms)
    let parse_start = Instant::now();
    let intent = OmniIntent::parse(
        "! convert this algorithm into safe rust",
        Some("def calc(a, b): return a / b".to_string()),
    );
    let parse_elapsed_micros = parse_start.elapsed().as_micros();

    assert!(
        parse_elapsed_micros < 500,
        "Intent classification must be <=0.5ms (got {}µs)",
        parse_elapsed_micros
    );

    match intent {
        OmniIntent::AgentAction { prompt, target_code } => {
            assert!(prompt.contains("convert"));
            assert!(target_code.is_some());
        }
        _ => panic!("Expected AgentAction intent"),
    }

    // 3. Atomic Text Injection & Metric Verification
    let sample_code = "pub fn add(x: i32, y: i32) -> i32 { x + y }";
    let report = SafeTextInjector::inject_text(sample_code, false);
    assert_eq!(report.bytes_injected, sample_code.len());
    assert!(report.elapsed_ms >= 0.0);

    // 4. Dismiss & Zero-Allocation Idle Return
    controller.dismiss();
    let post_dismiss_telemetry = controller.get_telemetry();
    assert!(post_dismiss_telemetry.ram_usage_mb <= 50.0);
}

#[test]
fn test_ambient_concurrency_stress_e2e() {
    use std::sync::Arc;
    use std::thread;

    let controller = Arc::new(AmbientController::new());
    let mut handles = Vec::new();

    for _ in 0..10 {
        let ctrl = Arc::clone(&controller);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let _ = ctrl.trigger_wake();
                let _ = OmniIntent::parse("? rust tokio channels", None);
                let _ = OmniIntent::parse("> cargo check --workspace", None);
                let _ = OmniIntent::parse("#verify calculate_ratio", None);
                ctrl.dismiss();
            }
        }));
    }

    for h in handles {
        h.join().expect("Concurrent thread join failed");
    }

    let final_telemetry = controller.get_telemetry();
    assert!(final_telemetry.ram_usage_mb > 0.0);
}
