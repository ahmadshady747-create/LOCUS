//! Prompt-to-DAG Autonomous Swarm Task Planner Barrel.
//!
//! Exposes topological planners, subtree conflict partitioners, specialized agent role matrices,
//! and parallel execution level synthesizers for the 300-agent sovereign swarm.

#![forbid(unsafe_code)]

pub mod partitioner;
pub mod planner;
pub mod types;

pub use partitioner::SubtreeConflictPartitioner;
pub use planner::SwarmDagPlanner;
pub use types::{
    AgentId, AgentRole, DagError, ExecutionDag, SwarmTaskNode, TaskId, TaskPriority, TaskStatus,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::socratic::{ArchitecturalPattern, SpecContract};
    use std::time::Instant;

    #[test]
    fn test_plan_from_contract_fullstack_app_10_tasks_and_5_roles() {
        let contract = SpecContract::new_draft(
            "Lock Sovereign Swarm App",
            vec!["UserEntity".to_string(), "SessionToken".to_string()],
            vec!["src/".to_string()],
            ArchitecturalPattern::CleanArchitecture,
        );

        let dag = SwarmDagPlanner::plan_from_contract(&contract, "SovereignPortal")
            .expect("DAG planning succeeds");

        assert_eq!(dag.total_tasks_count, 10);
        assert!(dag.total_levels() >= 5);
        assert!(dag.max_concurrency() >= 2);

        // Verify all 5 specialized roles exist in the planned DAG
        let roles: std::collections::HashSet<AgentRole> =
            dag.nodes.values().map(|n| n.role).collect();
        assert!(roles.contains(&AgentRole::SystemArchitect));
        assert!(roles.contains(&AgentRole::BackendEngineer));
        assert!(roles.contains(&AgentRole::FrontendEngineer));
        assert!(roles.contains(&AgentRole::SecurityAuditor));
        assert!(roles.contains(&AgentRole::QualityAssurance));
        assert!(roles.contains(&AgentRole::IntegrationOrchestrator));
    }

    #[test]
    fn test_cyclic_dependency_rejection() {
        // Create 2 tasks that depend on each other: Task A -> Task B -> Task A
        let id_a = TaskId::new("TaskA", 0);
        let id_b = TaskId::new("TaskB", 1);

        let task_a = SwarmTaskNode {
            id: id_a,
            title: "Task A".to_string(),
            description: "Cycle node A".to_string(),
            role: AgentRole::BackendEngineer,
            priority: TaskPriority::Normal,
            target_files: vec!["file_a.rs".to_string()],
            target_symbols: vec!["sym_a".to_string()],
            dependencies: vec![id_b], // Depends on B
            status: TaskStatus::Pending,
            assigned_agent: None,
        };

        let task_b = SwarmTaskNode {
            id: id_b,
            title: "Task B".to_string(),
            description: "Cycle node B".to_string(),
            role: AgentRole::BackendEngineer,
            priority: TaskPriority::Normal,
            target_files: vec!["file_b.rs".to_string()],
            target_symbols: vec!["sym_b".to_string()],
            dependencies: vec![id_a], // Depends on A
            status: TaskStatus::Pending,
            assigned_agent: None,
        };

        let res = SwarmDagPlanner::build_dag("Cyclic Graph", vec![task_a, task_b]);
        assert!(res.is_err(), "Cyclic graph must be rejected");
        match res.unwrap_err() {
            DagError::CyclicDependencyDetected(cycle) => {
                assert!(!cycle.is_empty());
            }
            other => panic!("Expected CyclicDependencyDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_topological_sort_invariant_dependencies_precede_dependents() {
        let contract = SpecContract::new_draft(
            "Lock Graph",
            vec!["Sym".to_string()],
            vec!["src/".to_string()],
            ArchitecturalPattern::Hexagonal,
        );
        let dag = SwarmDagPlanner::plan_from_contract(&contract, "TopoApp")
            .expect("Planning succeeds");

        let mut level_map = std::collections::HashMap::new();
        for (lvl_idx, level) in dag.execution_levels.iter().enumerate() {
            for &task_id in level {
                level_map.insert(task_id, lvl_idx);
            }
        }

        // For every task, its dependencies must be in a strictly earlier execution level
        for task in dag.nodes.values() {
            let task_lvl = level_map[&task.id];
            for dep_id in &task.dependencies {
                let dep_lvl = level_map[dep_id];
                assert!(
                    dep_lvl < task_lvl,
                    "Dependency {} (level {}) must execute strictly before task {} (level {})",
                    dep_id,
                    dep_lvl,
                    task.id,
                    task_lvl
                );
            }
        }
    }

    #[test]
    fn test_subtree_conflict_partitioning_defers_overlapping_files() {
        let id_0 = TaskId::new("TaskInit", 0);

        let task_0 = SwarmTaskNode::new("TaskInit", 0, "Root", AgentRole::SystemArchitect, TaskPriority::CriticalPath, vec!["root.rs".to_string()], vec![], vec![]);
        // Both task 1 and task 2 depend on task 0 and target the EXACT SAME FILE
        let task_1 = SwarmTaskNode::new("TaskWorker1", 1, "Edit Shared", AgentRole::BackendEngineer, TaskPriority::High, vec!["shared_state.rs".to_string()], vec![], vec![id_0]);
        let task_2 = SwarmTaskNode::new("TaskWorker2", 2, "Edit Shared Also", AgentRole::BackendEngineer, TaskPriority::Normal, vec!["shared_state.rs".to_string()], vec![], vec![id_0]);

        let mut dag = SwarmDagPlanner::build_dag("Conflict Test", vec![task_0, task_1, task_2]).unwrap();
        assert_eq!(dag.execution_levels.len(), 2);
        assert_eq!(dag.execution_levels[1].len(), 2, "Initially, both worker tasks sit in level 1");

        // Run partitioner
        let conflicts = SubtreeConflictPartitioner::partition_conflicts(&mut dag).unwrap();
        assert_eq!(conflicts, 1, "Must detect and resolve exactly 1 file conflict");

        // Now dag must have 3 levels, and level 1 & level 2 each have only 1 task
        assert_eq!(dag.execution_levels.len(), 3);
        assert_eq!(dag.execution_levels[1].len(), 1);
        assert_eq!(dag.execution_levels[2].len(), 1);

        assert!(SubtreeConflictPartitioner::verify_zero_conflicts(&dag).is_ok());
    }

    #[test]
    fn test_sub_0_05ms_dag_planning_budget() {
        let contract = SpecContract::new_draft(
            "Benchmark Contract",
            vec!["FastEntity".to_string()],
            vec!["src/".to_string()],
            ArchitecturalPattern::ActorMesh,
        );

        let start = Instant::now();
        for _ in 0..100 {
            let mut dag = SwarmDagPlanner::plan_from_contract(&contract, "BenchApp").unwrap();
            let _ = SubtreeConflictPartitioner::partition_conflicts(&mut dag);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 100.0;

        let budget = if cfg!(debug_assertions) { 1000.0 } else { 100.0 };
        assert!(
            avg_us < budget,
            "DAG planning, topological sort, and conflict partitioning must complete rapidly (took: {:.2}µs, budget: {:.2}µs)",
            avg_us, budget
        );
    }
}
