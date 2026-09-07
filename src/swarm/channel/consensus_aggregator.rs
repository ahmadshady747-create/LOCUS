//! Deterministic Swarm Consensus Aggregator.
//!
//! Synchronizes parallel execution levels across the 300-agent swarm, tracks task completions,
//! enforces fault-tolerant reassignment, and seals verifiable consensus receipts.

#![forbid(unsafe_code)]

use crate::swarm::channel::types::{ChannelError, ConsensusStatus, SwarmConsensusReceipt};
use crate::swarm::dag::types::ExecutionDag;
use crate::types::fnv1a_64;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Record of an individual completed task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCompletionRecord {
    pub task_id: u64,
    pub agent_id: u32,
    pub output_hash: u64,
}

/// Central aggregator coordinating deterministic consensus and parallel DAG level progression.
#[derive(Debug, Clone)]
pub struct SwarmConsensusAggregator {
    inner: Arc<RwLock<AggregatorInner>>,
}

#[derive(Debug)]
struct AggregatorInner {
    dag: ExecutionDag,
    current_level: usize,
    completed_tasks: HashMap<u64, TaskCompletionRecord>,
    active_assignments: HashMap<u64, u32>,
    task_progress: HashMap<u64, f32>,
    failed_reassignments: HashMap<u64, Vec<u32>>,
    start_time: Instant,
    status: ConsensusStatus,
}

impl SwarmConsensusAggregator {
    /// Constructs a new `SwarmConsensusAggregator` initialized with an `ExecutionDag`.
    pub fn new(dag: ExecutionDag) -> Self {
        let mut initial_assignments = HashMap::new();
        for (task_id, node) in &dag.nodes {
            if let Some(agent) = node.assigned_agent {
                initial_assignments.insert(task_id.0, agent.0);
            }
        }

        Self {
            inner: Arc::new(RwLock::new(AggregatorInner {
                dag,
                current_level: 0,
                completed_tasks: HashMap::new(),
                active_assignments: initial_assignments,
                task_progress: HashMap::new(),
                failed_reassignments: HashMap::new(),
                start_time: Instant::now(),
                status: ConsensusStatus::Collecting,
            })),
        }
    }

    /// Assigns or rebinds a task to a specific agent.
    pub fn assign_task(&self, task_id: u64, agent_id: u32) -> Result<(), ChannelError> {
        let mut inner = self.inner.write();
        if inner.status == ConsensusStatus::ConsensusSealed {
            return Err(ChannelError::ConsensusAlreadySealed(inner.dag.dag_id));
        }

        inner.active_assignments.insert(task_id, agent_id);
        Ok(())
    }

    /// Records progress percentage (0.0 to 1.0) for an active task.
    pub fn register_progress(&self, agent_id: u32, task_id: u64, progress: f32) -> Result<(), ChannelError> {
        let mut inner = self.inner.write();
        if let Some(&assigned) = inner.active_assignments.get(&task_id) {
            if assigned != agent_id {
                return Err(ChannelError::TaskAssignmentConflict(task_id));
            }
        }

        inner.task_progress.insert(task_id, progress.clamp(0.0, 1.0));
        Ok(())
    }

    /// Records task completion by an agent with the task's output hash.
    pub fn complete_task(&self, agent_id: u32, task_id: u64, output_hash: u64) -> Result<(), ChannelError> {
        let mut inner = self.inner.write();
        if inner.status == ConsensusStatus::ConsensusSealed {
            return Err(ChannelError::ConsensusAlreadySealed(inner.dag.dag_id));
        }

        inner.completed_tasks.insert(
            task_id,
            TaskCompletionRecord {
                task_id,
                agent_id,
                output_hash,
            },
        );
        inner.task_progress.insert(task_id, 1.0);

        // Update status if current level reached quorum
        let current_lvl = inner.current_level;
        if current_lvl < inner.dag.execution_levels.len() {
            let level_tasks = &inner.dag.execution_levels[current_lvl];
            let all_done = level_tasks
                .iter()
                .all(|tid| inner.completed_tasks.contains_key(&tid.0));
            if all_done {
                inner.status = ConsensusStatus::QuorumReached;
            }
        }

        Ok(())
    }

    /// Handles agent failure: strips assignment and dynamically reassigns task to a fallback agent.
    pub fn fail_and_reassign_task(
        &self,
        failed_agent_id: u32,
        task_id: u64,
        fallback_agent_id: u32,
    ) -> Result<(), ChannelError> {
        let mut inner = self.inner.write();
        if inner.status == ConsensusStatus::ConsensusSealed {
            return Err(ChannelError::ConsensusAlreadySealed(inner.dag.dag_id));
        }

        inner
            .failed_reassignments
            .entry(task_id)
            .or_default()
            .push(failed_agent_id);

        inner.active_assignments.insert(task_id, fallback_agent_id);
        inner.task_progress.insert(task_id, 0.0);
        inner.status = ConsensusStatus::Collecting;

        Ok(())
    }

    /// Checks if all tasks in the given execution level have completed successfully.
    pub fn check_level_quorum(&self, level_idx: usize) -> Result<bool, ChannelError> {
        let inner = self.inner.read();
        if level_idx >= inner.dag.execution_levels.len() {
            return Ok(true);
        }

        let level_tasks = &inner.dag.execution_levels[level_idx];
        let completed_count = level_tasks
            .iter()
            .filter(|tid| inner.completed_tasks.contains_key(&tid.0))
            .count();

        Ok(completed_count == level_tasks.len())
    }

    /// Advances to the next execution level if current level quorum is reached.
    pub fn advance_level(&self) -> Result<usize, ChannelError> {
        let mut inner = self.inner.write();
        let current_lvl = inner.current_level;

        if current_lvl >= inner.dag.execution_levels.len() {
            return Ok(current_lvl);
        }

        let level_tasks = &inner.dag.execution_levels[current_lvl];
        let completed_count = level_tasks
            .iter()
            .filter(|tid| inner.completed_tasks.contains_key(&tid.0))
            .count();

        if completed_count < level_tasks.len() {
            return Err(ChannelError::QuorumNotReached {
                expected: level_tasks.len(),
                actual: completed_count,
            });
        }

        inner.current_level += 1;
        if inner.current_level < inner.dag.execution_levels.len() {
            inner.status = ConsensusStatus::Collecting;
        } else {
            inner.status = ConsensusStatus::QuorumReached;
        }

        Ok(inner.current_level)
    }

    /// Returns true if all tasks across all execution levels in the DAG are complete.
    pub fn is_dag_completed(&self) -> bool {
        let inner = self.inner.read();
        inner.dag.nodes.keys().all(|task_id| inner.completed_tasks.contains_key(&task_id.0))
    }

    /// Finalizes consensus, seals the DAG, and produces a verifiable `SwarmConsensusReceipt`.
    pub fn finalize_consensus(&self) -> Result<SwarmConsensusReceipt, ChannelError> {
        let mut inner = self.inner.write();
        if inner.status == ConsensusStatus::ConsensusSealed {
            return Err(ChannelError::ConsensusAlreadySealed(inner.dag.dag_id));
        }

        let total_nodes = inner.dag.nodes.len();
        let completed_nodes = inner.completed_tasks.len();

        if completed_nodes < total_nodes {
            return Err(ChannelError::QuorumNotReached {
                expected: total_nodes,
                actual: completed_nodes,
            });
        }

        // Deterministically compute cumulative CST root hash
        let mut committed_tasks = Vec::with_capacity(total_nodes);
        let mut cumulative_bytes = Vec::with_capacity(total_nodes * 16);

        // Sort nodes by task ID for deterministic hashing
        let mut sorted_tasks: Vec<_> = inner.completed_tasks.values().cloned().collect();
        sorted_tasks.sort_by_key(|t| t.task_id);

        for task in &sorted_tasks {
            committed_tasks.push(task.task_id);
            cumulative_bytes.extend_from_slice(&task.task_id.to_le_bytes());
            cumulative_bytes.extend_from_slice(&task.output_hash.to_le_bytes());
        }

        let final_cst_root_hash = fnv1a_64(&cumulative_bytes);
        let total_execution_time_ms = inner.start_time.elapsed().as_secs_f64() * 1000.0;

        inner.status = ConsensusStatus::ConsensusSealed;

        Ok(SwarmConsensusReceipt {
            dag_id: inner.dag.dag_id,
            committed_tasks,
            final_cst_root_hash,
            total_execution_time_ms,
            status: ConsensusStatus::ConsensusSealed,
        })
    }
}
