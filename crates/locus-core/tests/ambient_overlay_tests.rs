//! Integration and Concurrency Test Suite for Ambient Overlay & OS Daemon Controller.

use locus_core::{AmbientController, AmbientTelemetry};
use std::sync::Arc;
use std::thread;

#[test]
fn test_ambient_controller_wake_cycle() {
    let controller = AmbientController::new();
    assert!(!controller.is_visible());

    // 1. Trigger Wake
    let latency_ms = controller.trigger_wake();
    assert!(controller.is_visible());
    assert!(latency_ms < 5.0, "Wake latency must be sub-5ms in tests (got {}ms)", latency_ms);

    // 2. Query Telemetry
    let telemetry = controller.get_telemetry();
    assert!(telemetry.ram_usage_mb > 0.0);
    assert_eq!(telemetry.tokens_saved_pct, 96);
    assert!(telemetry.estimated_cost_saved_usd > 0.0);

    // 3. Dismiss Overlay
    controller.dismiss();
    assert!(!controller.is_visible());
}

#[test]
fn test_ambient_telemetry_serialization() {
    let telemetry = AmbientTelemetry {
        ram_usage_mb: 58.4,
        latency_ms: 0.92,
        tokens_saved_pct: 96,
        estimated_cost_saved_usd: 1.45,
    };

    let json = serde_json::to_string(&telemetry).expect("Failed to serialize AmbientTelemetry");
    assert!(json.contains("\"ram_usage_mb\":58.4"));
    assert!(json.contains("\"latency_ms\":0.92"));
    assert!(json.contains("\"tokens_saved_pct\":96"));

    let deserialized: AmbientTelemetry = serde_json::from_str(&json).expect("Failed to deserialize AmbientTelemetry");
    assert_eq!(deserialized, telemetry);
}

#[test]
fn test_zero_panic_concurrency() {
    let controller = Arc::new(AmbientController::new());
    let mut handles = Vec::new();

    // Spawn 10 concurrent threads rapidly exercising wake, telemetry queries, and dismissal
    for _ in 0..10 {
        let c = Arc::clone(&controller);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let latency = c.trigger_wake();
                assert!(latency >= 0.0);
                let telem = c.get_telemetry();
                assert!(telem.latency_ms >= 0.0);
                assert!(telem.ram_usage_mb > 0.0);
                c.dismiss();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Concurrent thread panicked in zero-panic test");
    }

    controller.dismiss();
    assert!(!controller.is_visible());
}
