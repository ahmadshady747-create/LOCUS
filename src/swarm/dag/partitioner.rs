//! Subtree Conflict Partitioner and Blast Radius Isolation.
//!
//! Audits concurrent execution levels to ensure zero file/symbol overlap between
//! parallel swarm agents, deferring conflicting tasks across levels to guarantee OCC safety.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use crate::swarm::dag::types::{DagError, ExecutionDag, TaskId};

pub struct SubtreeConflictPartitioner;

impl SubtreeConflictPartitioner {
    /// Partitions and resolves any concurrent file/symbol collisions across all DAG execution levels.
    /// Returns the number of resolved conflicts.
    pub fn partition_conflicts(dag: &mut ExecutionDag) -> Result<usize, DagError> {
        let mut total_conflicts_resolved = 0;
        let mut level_idx = 0;

        while level_idx < dag.execution_levels.len() {
            let current_tasks = dag.execution_levels[level_idx].clone();
            let mut files_claimed: HashMap<String, TaskId> = HashMap::new();
            let mut symbols_claimed: HashMap<String, TaskId> = HashMap::new();

            let mut tasks_to_keep = Vec::new();
            let mut tasks_to_defer = Vec::new();

            for task_id in current_tasks {
                let task = match dag.nodes.get(&task_id) {
                    Some(t) => t,
                    None => return Err(DagError::TaskNotFound(task_id)),
                };

                let mut has_conflict = false;

                // Check file overlap
                for file in &task.target_files {
                    if let Some(&_other_id) = files_claimed.get(file) {
                        has_conflict = true;
                        break;
                    }
                }

                // Check symbol overlap
                if !has_conflict {
                    for sym in &task.target_symbols {
                        if let Some(&_other_id) = symbols_claimed.get(sym) {
                            has_conflict = true;
                            break;
                        }
                    }
                }

                if has_conflict {
                    tasks_to_defer.push(task_id);
                    total_conflicts_resolved += 1;
                } else {
                    for file in &task.target_files {
                        files_claimed.insert(file.clone(), task_id);
                    }
                    for sym in &task.target_symbols {
                        symbols_claimed.insert(sym.clone(), task_id);
                    }
                    tasks_to_keep.push(task_id);
                }
            }

            dag.execution_levels[level_idx] = tasks_to_keep;

            // If there are tasks to defer, push them to the next level
            if !tasks_to_defer.is_empty() {
                if level_idx + 1 < dag.execution_levels.len() {
                    dag.execution_levels[level_idx + 1].extend(tasks_to_defer);
                } else {
                    dag.execution_levels.push(tasks_to_defer);
                }
            }

            level_idx += 1;
        }

        // Clean empty levels if any
        dag.execution_levels.retain(|lvl| !lvl.is_empty());

        Ok(total_conflicts_resolved)
    }

    /// Verifies that no two tasks in any execution level share target files.
    pub fn verify_zero_conflicts(dag: &ExecutionDag) -> Result<(), DagError> {
        for (lvl_idx, level) in dag.execution_levels.iter().enumerate() {
            let mut claimed_files = HashSet::new();
            for &task_id in level {
                let task = dag.nodes.get(&task_id).ok_or(DagError::TaskNotFound(task_id))?;
                for file in &task.target_files {
                    if !claimed_files.insert(file) {
                        return Err(DagError::ConflictDetected(format!(
                            "Execution level {} contains multiple tasks competing for file '{}'",
                            lvl_idx, file
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}
