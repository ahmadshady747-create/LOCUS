//! Live Terminal HUD & Real-Time Telemetry Subsystem.
//!
//! Provides lock-free atomic telemetry collectors and non-blocking 60 FPS
//! terminal dashboards in 100% pure safe Rust.

#![forbid(unsafe_code)]

pub mod telemetry_collector;
pub mod terminal_dashboard;

pub use telemetry_collector::{TelemetryCollector, TelemetrySnapshot};
pub use terminal_dashboard::TerminalHudRenderer;

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn test_telemetry_collector_atomic_aggregation() {
        let collector = Arc::new(TelemetryCollector::new());

        collector.record_ram_kb(16384);
        collector.record_verification_latency(450);
        collector.set_active_agents(42);
        collector.set_occ_leases(15);

        let mut handles = Vec::new();
        for _ in 0..10 {
            let col = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    col.record_wal_commit();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let snapshot = collector.capture_snapshot();
        assert_eq!(snapshot.resident_ram_kb, 16384);
        assert_eq!(snapshot.verification_latency_ns, 450);
        assert_eq!(snapshot.active_agents, 42);
        assert_eq!(snapshot.occ_active_leases, 15);
        assert_eq!(snapshot.wal_transactions_committed, 10_000);
        assert_eq!(snapshot.total_operations, 10_000);
        assert!(snapshot.throughput_ops_per_sec > 0.0);
    }

    #[test]
    fn test_terminal_hud_rendering_and_sub_0_05ms_latency() {
        let mut renderer = TerminalHudRenderer::new();
        let snapshot = TelemetrySnapshot {
            timestamp_ns: 1_000_000,
            resident_ram_kb: 32768,
            verification_latency_ns: 85,
            active_agents: 128,
            occ_active_leases: 32,
            wal_transactions_committed: 50000,
            total_operations: 50000,
            throughput_ops_per_sec: 125000.0,
            uptime_secs: 0.40,
        };

        let start = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let output = renderer.render_to_buffer(&snapshot);
            assert!(output.contains("LOCUS v2.0"));
            assert!(output.contains("128"));
        }
        let elapsed = start.elapsed();
        let avg_us = (elapsed.as_micros() as f64) / (iterations as f64);

        println!("⚡ Average Terminal HUD Frame Render Latency: {:.3} µs", avg_us);

        let budget_us = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
        assert!(
            avg_us < budget_us,
            "HUD render must execute in sub-0.05ms (<50µs in release), got: {:.3} µs",
            avg_us
        );
    }
}
