//! Compute Budget Manager & Deterministic Frame-Pacing Engine.
//!
//! Tracks microsecond frame budgets, throttles non-critical compute workloads,
//! and enforces 60 FPS (16.6ms) / 185 FPS (5.4ms) real-time deadlines in 100% safe Rust.

#![forbid(unsafe_code)]

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use crate::scheduler::types::FramePacingMode;

/// Frame execution summary report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameReport {
    pub frame_index: usize,
    pub elapsed_us: u64,
    pub budget_us: u64,
    pub budget_exceeded: bool,
    pub deferred_tasks_count: usize,
}

pub struct ComputeBudgetManager {
    pacing_mode: FramePacingMode,
    frame_start: RwLock<Instant>,
    accumulated_cost_us: AtomicU64,
    deferred_tasks_count: AtomicUsize,
    total_frames_completed: AtomicUsize,
}

impl Default for ComputeBudgetManager {
    fn default() -> Self {
        Self::new(FramePacingMode::Pacing60Fps)
    }
}

impl ComputeBudgetManager {
    pub fn new(pacing_mode: FramePacingMode) -> Self {
        Self {
            pacing_mode,
            frame_start: RwLock::new(Instant::now()),
            accumulated_cost_us: AtomicU64::new(0),
            deferred_tasks_count: AtomicUsize::new(0),
            total_frames_completed: AtomicUsize::new(0),
        }
    }

    /// Resets the microsecond frame timer for a new frame cycle.
    pub fn start_frame(&self) {
        let mut start_lock = self.frame_start.write();
        *start_lock = Instant::now();
        self.accumulated_cost_us.store(0, Ordering::Relaxed);
        self.deferred_tasks_count.store(0, Ordering::Relaxed);
    }

    /// Evaluates whether a compute task should be deferred or LOD-demoted based on remaining budget.
    #[inline(always)]
    pub fn should_throttle_or_defer(&self, task_priority: u8) -> bool {
        // Priority 0 (Critical/Real-Time) and Priority 1 (High) are never throttled
        if task_priority <= 1 {
            return false;
        }

        let elapsed = self.frame_elapsed_us();
        let threshold = self.pacing_mode.throttle_threshold_us();

        if elapsed >= threshold {
            self.deferred_tasks_count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Records the execution cost of a completed task.
    pub fn record_task_cost(&self, cost_us: u64) {
        self.accumulated_cost_us
            .fetch_add(cost_us, Ordering::Relaxed);
    }

    /// Returns elapsed time in the current frame in microseconds.
    pub fn frame_elapsed_us(&self) -> u64 {
        let start = *self.frame_start.read();
        start.elapsed().as_micros() as u64
    }

    /// Returns remaining frame budget in microseconds.
    pub fn frame_budget_remaining_us(&self) -> u64 {
        let budget = self.pacing_mode.budget_us();
        let elapsed = self.frame_elapsed_us();
        budget.saturating_sub(elapsed)
    }

    /// Finalizes the current frame and returns a performance report.
    pub fn end_frame(&self) -> FrameReport {
        let elapsed_us = self.frame_elapsed_us();
        let budget_us = self.pacing_mode.budget_us();
        let frame_index = self.total_frames_completed.fetch_add(1, Ordering::Relaxed);
        let deferred = self.deferred_tasks_count.load(Ordering::Relaxed);

        FrameReport {
            frame_index,
            elapsed_us,
            budget_us,
            budget_exceeded: elapsed_us > budget_us,
            deferred_tasks_count: deferred,
        }
    }

    pub fn pacing_mode(&self) -> FramePacingMode {
        self.pacing_mode
    }

    pub fn total_deferred_tasks(&self) -> usize {
        self.deferred_tasks_count.load(Ordering::Relaxed)
    }
}
