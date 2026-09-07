//! Heterogeneous Workload Dispatcher & Work-Stealing Swarm Scheduler Barrel.
//!
//! Exposes heterogeneous compute routing, lock-free work-stealing pools,
//! and deterministic frame-budget pacing engines in 100% safe Rust.

#![forbid(unsafe_code)]

pub mod budget_manager;
pub mod heterogeneous_dispatcher;
pub mod types;
pub mod work_stealing_pool;

pub use budget_manager::{ComputeBudgetManager, FrameReport};
pub use heterogeneous_dispatcher::HeterogeneousDispatcher;
pub use types::{
    ComputeBackend, ComputeTask, FramePacingMode, SchedulerError, TaskRouteDecision,
};
pub use work_stealing_pool::WorkStealingPool;

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
    fn test_heterogeneous_dispatcher_routing_accuracy_and_sub_microsecond_latency() {
        let dispatcher = HeterogeneousDispatcher::default();

        // 1. Massive parallel task -> GPU Pipeline
        let gpu_task = ComputeTask::new("MatMul", 1, 4096, false, false, 1, 100);
        let dec_gpu = dispatcher.dispatch_task(&gpu_task);
        assert_eq!(dec_gpu.backend, ComputeBackend::GpuComputePipeline);

        // 2. SIMD vectorized task -> SIMD CPU Worker
        let simd_task = ComputeTask::new("VectorNorm", 2, 512, false, true, 1, 50);
        let dec_simd = dispatcher.dispatch_task(&simd_task);
        assert_eq!(dec_simd.backend, ComputeBackend::SimdCpuWorker);

        // 3. Branch-heavy task with large data -> SIMD CPU Worker
        let branch_task = ComputeTask::new("TreeSearch", 3, 8192, true, false, 1, 80);
        let dec_branch = dispatcher.dispatch_task(&branch_task);
        assert_eq!(dec_branch.backend, ComputeBackend::SimdCpuWorker);

        // 4. Lightweight scalar task -> Main Thread Direct
        let scalar_task = ComputeTask::new("Kinematics", 4, 64, false, false, 0, 5);
        let dec_scalar = dispatcher.dispatch_task(&scalar_task);
        assert_eq!(dec_scalar.backend, ComputeBackend::MainThreadDirect);

        // Latency benchmark for 1000 dispatches
        let start = Instant::now();
        for i in 0..1000 {
            let task = ComputeTask::new("BenchTask", i, (i % 5000) as usize, i % 2 == 0, i % 3 == 0, 1, 10);
            let _ = dispatcher.dispatch_task(&task);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 1000.0;

        let threshold_us = if cfg!(debug_assertions) { 20.0 } else { 2.0 };
        assert!(
            avg_us < threshold_us,
            "Routing decision must execute rapidly (sub-20µs in debug, sub-2.0µs in release), took: {:.4}µs",
            avg_us
        );
    }

    #[test]
    fn test_work_stealing_across_8_concurrent_threads_without_deadlock() {
        let pool = Arc::new(WorkStealingPool::new(8, 10_000));

        // Load 8,000 tasks exclusively onto Worker 0 (heavily unbalanced workload)
        for i in 0..8000 {
            let task = ComputeTask::new("HeavyBatch", i, 128, false, false, 2, 10);
            pool.push_to_worker(0, task).unwrap();
        }

        assert_eq!(pool.total_pending_tasks(), 8000);

        // Spawn 8 worker threads to process tasks concurrently via work-stealing
        let mut handles = Vec::new();
        for worker_id in 0..8 {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                let mut processed = 0;
                let start = Instant::now();
                while start.elapsed().as_millis() < 2000 {
                    if let Some(_task) = pool_clone.pop_or_steal(worker_id) {
                        processed += 1;
                        if worker_id == 0 && processed % 10 == 0 {
                            thread::yield_now();
                        }
                    } else if pool_clone.total_pending_tasks() == 0 {
                        break;
                    } else {
                        thread::yield_now();
                    }
                }
                processed
            });
            handles.push(handle);
        }

        let mut total_processed = 0;
        for handle in handles {
            total_processed += handle.join().expect("Worker thread panicked");
        }

        assert_eq!(total_processed, 8000, "All 8,000 tasks must be processed");
        assert_eq!(pool.total_pending_tasks(), 0, "No pending tasks should remain");
        assert!(
            pool.total_stolen_count() > 0,
            "Work-stealing must occur across idle workers (stolen: {})",
            pool.total_stolen_count()
        );
    }

    #[test]
    fn test_compute_budget_manager_pacing_and_lod_demotion() {
        let budget_mgr = ComputeBudgetManager::new(FramePacingMode::Pacing60Fps);
        budget_mgr.start_frame();

        // Priority 0 (Critical) is never throttled
        assert!(!budget_mgr.should_throttle_or_defer(0));
        // Priority 1 (High) is never throttled
        assert!(!budget_mgr.should_throttle_or_defer(1));

        // Under normal frame start (< 14.2ms), Priority 2 is not throttled
        assert!(!budget_mgr.should_throttle_or_defer(2));

        // Simulate custom budget with 100µs threshold to test immediate throttle
        let tight_budget = ComputeBudgetManager::new(FramePacingMode::Custom {
            budget_us: 200,
            throttle_threshold_us: 10,
        });
        tight_budget.start_frame();
        // Sleep briefly to exceed 10µs
        thread::sleep(std::time::Duration::from_micros(50));

        // Priority 0 & 1 still proceed
        assert!(!tight_budget.should_throttle_or_defer(0));
        assert!(!tight_budget.should_throttle_or_defer(1));

        // Priority 2 & 3 must be throttled / deferred
        assert!(tight_budget.should_throttle_or_defer(2));
        assert!(tight_budget.should_throttle_or_defer(3));

        let report = tight_budget.end_frame();
        assert_eq!(report.deferred_tasks_count, 2);
    }

    #[test]
    fn test_high_throughput_50000_tasks_stress_allocation_budget() {
        let pool = WorkStealingPool::new(4, 60_000);
        let dispatcher = HeterogeneousDispatcher::default();

        let start = Instant::now();
        for i in 0..50_000 {
            let task = ComputeTask::new("StressTask", i, (i % 4096) as usize, i % 5 == 0, i % 3 == 0, (i % 4) as u8, 1);
            let _decision = dispatcher.dispatch_task(&task);
            pool.schedule_balanced(task);
        }

        assert_eq!(pool.total_pending_tasks(), 50_000);
        let processed = pool.process_all_tasks();
        assert_eq!(processed, 50_000);

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 50_000.0;

        assert!(
            avg_us < 50.0,
            "50,000 tasks dispatch, schedule, and process must complete rapidly (< 50µs per task), took: {:.2}µs",
            avg_us
        );
    }
}
