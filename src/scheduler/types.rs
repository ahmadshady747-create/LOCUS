//! Data types and models for the Heterogeneous Workload Dispatcher & Swarm Scheduler.
//!
//! Enforces sub-microsecond routing decisions, lock-free work-stealing schemas,
//! and deterministic frame-budget pacing contracts in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Compute Backend & Task Routing
// ---------------------------------------------------------------------------

/// Execution backend target determined by the heterogeneous dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeBackend {
    /// Massive data-parallel compute pipelines (N >= 2048 elements).
    GpuComputePipeline,
    /// Vector-accelerated SIMD worker threads (128/256-bit AVX2/NEON/WASM).
    SimdCpuWorker,
    /// Low-latency direct execution on the main thread or local scalar CPU worker.
    MainThreadDirect,
}

impl fmt::Display for ComputeBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GpuComputePipeline => write!(f, "GPU_COMPUTE_PIPELINE"),
            Self::SimdCpuWorker => write!(f, "SIMD_CPU_WORKER"),
            Self::MainThreadDirect => write!(f, "MAIN_THREAD_DIRECT"),
        }
    }
}

/// Discrete unit of compute payload evaluated by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeTask {
    /// Deterministic 64-bit task identifier.
    pub id: u64,
    /// Number of elements / input array size.
    pub data_size: usize,
    /// Indicates whether the task exhibits deep branching vs uniform data paths.
    pub is_branch_heavy: bool,
    /// Explicit request for SIMD vector lane acceleration.
    pub requires_simd: bool,
    /// Priority level (0 = Critical/Realtime, 1 = High, 2 = Normal, 3 = Deferred/LOD).
    pub priority: u8,
    /// Estimated computation execution cost in microseconds.
    pub estimated_cost_us: u32,
}

impl ComputeTask {
    pub fn new(
        name: &str,
        seq: u64,
        data_size: usize,
        is_branch_heavy: bool,
        requires_simd: bool,
        priority: u8,
        estimated_cost_us: u32,
    ) -> Self {
        let id = fnv1a_64(format!("{}:{}", name, seq).as_bytes());
        Self {
            id,
            data_size,
            is_branch_heavy,
            requires_simd,
            priority,
            estimated_cost_us,
        }
    }
}

/// Outcome of a heterogeneous routing decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRouteDecision {
    pub task_id: u64,
    pub backend: ComputeBackend,
    pub decision_latency_ns: u64,
    pub is_deferred: bool,
}

// ---------------------------------------------------------------------------
// Frame Pacing & Budget Modes
// ---------------------------------------------------------------------------

/// Standard display refresh rate pacing targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FramePacingMode {
    /// 60 FPS target (16.6ms budget, 14.2ms throttle threshold).
    Pacing60Fps,
    /// 185 FPS high-refresh target (5.4ms budget, 4.8ms throttle threshold).
    Pacing185Fps,
    /// Custom microsecond budget and throttle threshold.
    Custom {
        budget_us: u64,
        throttle_threshold_us: u64,
    },
}

impl FramePacingMode {
    /// Returns the total frame budget in microseconds.
    pub fn budget_us(&self) -> u64 {
        match self {
            Self::Pacing60Fps => 16_666,
            Self::Pacing185Fps => 5_405,
            Self::Custom { budget_us, .. } => *budget_us,
        }
    }

    /// Returns the threshold in microseconds after which non-critical tasks are throttled/deferred.
    pub fn throttle_threshold_us(&self) -> u64 {
        match self {
            Self::Pacing60Fps => 14_200,
            Self::Pacing185Fps => 4_800,
            Self::Custom {
                throttle_threshold_us,
                ..
            } => *throttle_threshold_us,
        }
    }
}

impl Default for FramePacingMode {
    fn default() -> Self {
        Self::Pacing60Fps
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerError {
    #[error("Work queue overflow: maximum worker queue capacity ({0}) exceeded")]
    QueueOverflow(usize),

    #[error("Worker not found with ID: {0}")]
    WorkerNotFound(usize),

    #[error("Frame budget exceeded: frame elapsed {elapsed_us}µs > budget {budget_us}µs")]
    FrameBudgetExceeded { elapsed_us: u64, budget_us: u64 },

    #[error("Internal scheduler error: {0}")]
    InternalError(String),
}
