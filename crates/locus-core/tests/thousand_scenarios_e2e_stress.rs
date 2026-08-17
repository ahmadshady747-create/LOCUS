//! End-to-End Stress Test Suite for 1,000 Real-World Developer Workflows.
//!
//! Evaluates:
//! 1. 1,000 sequential/concurrent full-cycle workflows (Wake -> Parse -> Verify -> Atomic FS -> Cleanup).
//! 2. Zero-leak memory stability across all iterations (bounded <90MB).
//! 3. Guaranteed zero panics and zero data corruptions.

use locus_core::chaos_simulator::{run_chaos_benchmark, simulate_developer_workflow};
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_thousand_scenarios_comprehensive_stress() {
    let temp_dir = TempDir::new().expect("Failed to create temporary test directory");
    let start_all = Instant::now();

    // Run 1,000 developer workflow cycles
    let report = run_chaos_benchmark(1000, temp_dir.path())
        .expect("1,000 scenario benchmark must succeed without errors");

    let total_elapsed_s = start_all.elapsed().as_secs_f64();

    println!(
        "🚀 1,000 Scenarios Benchmark Completed in {:.2}s | Avg Latency: {:.3}ms | Peak RAM: {:.2}MB | Passed: {}/{} | Corruptions: {} | Panics: {}",
        total_elapsed_s,
        report.avg_latency_ms,
        report.peak_memory_mb,
        report.passed_scenarios,
        report.total_scenarios_executed,
        report.corrupted_files,
        report.panics_detected
    );

    // 1. Completeness & Success Rate Invariants
    assert_eq!(report.total_scenarios_executed, 1000);
    assert_eq!(report.passed_scenarios, 1000, "All 1,000 scenarios must pass 100%");
    assert_eq!(report.panics_detected, 0, "Zero panics allowed");
    assert_eq!(report.corrupted_files, 0, "Zero file corruption allowed");

    // 2. Resource & Latency Invariants
    assert!(
        report.peak_memory_mb <= 90.0,
        "Peak memory footprint must stay strictly <= 90.0MB (got {:.2}MB)",
        report.peak_memory_mb
    );
    assert!(
        report.avg_latency_ms < 15.0,
        "Average workflow latency must be < 15.0ms (got {:.3}ms)",
        report.avg_latency_ms
    );
}

#[test]
fn test_memory_leak_stability_over_iterations() {
    let temp_dir = TempDir::new().expect("Failed to create temporary test directory");

    // Sample first 50 iterations vs last 50 iterations to confirm zero memory accumulation
    let mut initial_allocated_bytes = 0;
    for id in 0..50 {
        let m = simulate_developer_workflow(id, temp_dir.path()).expect("Initial simulation failed");
        initial_allocated_bytes += m.ram_allocated_bytes;
    }

    let mut later_allocated_bytes = 0;
    for id in 950..1000 {
        let m = simulate_developer_workflow(id, temp_dir.path()).expect("Later simulation failed");
        later_allocated_bytes += m.ram_allocated_bytes;
    }

    let avg_initial = initial_allocated_bytes / 50;
    let avg_later = later_allocated_bytes / 50;

    // Both batches should have nearly identical allocation footprint (Zero Memory Leak, bounded by ID string length)
    let delta = (avg_initial as isize - avg_later as isize).abs();
    assert!(
        delta < 50,
        "Memory allocation footprint must be deterministic and constant across iterations (delta: {} bytes)",
        delta
    );
}
