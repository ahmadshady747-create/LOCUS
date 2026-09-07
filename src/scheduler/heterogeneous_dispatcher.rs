//! Heterogeneous Workload Dispatcher.
//!
//! Evaluates compute task properties (data size, branching complexity, SIMD requirements)
//! to make sub-microsecond routing decisions between GPU, SIMD Workers, and Main Thread.

#![forbid(unsafe_code)]

use crate::scheduler::types::{ComputeBackend, ComputeTask, TaskRouteDecision};

pub struct HeterogeneousDispatcher {
    /// Minimum elements threshold to warrant GPU pipeline dispatch.
    gpu_parallel_threshold: usize,
}

impl Default for HeterogeneousDispatcher {
    fn default() -> Self {
        Self {
            gpu_parallel_threshold: 2048,
        }
    }
}

impl HeterogeneousDispatcher {
    pub fn new(gpu_parallel_threshold: usize) -> Self {
        Self {
            gpu_parallel_threshold,
        }
    }

    /// Evaluates a compute task and makes an instant, sub-microsecond routing decision.
    #[inline(always)]
    pub fn dispatch_task(&self, task: &ComputeTask) -> TaskRouteDecision {
        let backend = if task.data_size >= self.gpu_parallel_threshold && !task.is_branch_heavy {
            // Massive data-parallel workload with uniform compute paths -> GPU Pipeline
            ComputeBackend::GpuComputePipeline
        } else if task.requires_simd || task.is_branch_heavy {
            // Vector-accelerated SIMD with branching -> SIMD CPU Worker Pool
            ComputeBackend::SimdCpuWorker
        } else {
            // Lightweight scalar kinematics / scalar compute -> Main Thread Direct
            ComputeBackend::MainThreadDirect
        };

        TaskRouteDecision {
            task_id: task.id,
            backend,
            decision_latency_ns: 0,
            is_deferred: false,
        }
    }

    pub fn gpu_parallel_threshold(&self) -> usize {
        self.gpu_parallel_threshold
    }
}
