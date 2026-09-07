//! Autonomous Swarm Task Planner & Deterministic DAG Synthesizer.
//!
//! Decomposes intent and architectural specifications into a Directed Acyclic Graph (DAG)
//! with Kahn's topological sorting, cycle detection, and parallel execution levels.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use crate::intent::socratic::SpecContract;
use crate::swarm::dag::types::{
    AgentRole, DagError, ExecutionDag, SwarmTaskNode, TaskId, TaskPriority,
};
use crate::types::fnv1a_64;

pub struct SwarmDagPlanner;

impl SwarmDagPlanner {
    /// Builds and topologically partitions a list of swarm task nodes into a validated `ExecutionDag`.
    pub fn build_dag(title: &str, tasks: Vec<SwarmTaskNode>) -> Result<ExecutionDag, DagError> {
        if tasks.is_empty() {
            return Err(DagError::EmptyDag);
        }

        let total_tasks_count = tasks.len();
        let mut nodes: HashMap<TaskId, SwarmTaskNode> = HashMap::with_capacity(total_tasks_count);
        for task in tasks {
            nodes.insert(task.id, task);
        }

        // Validate that all dependencies exist in the node set
        for task in nodes.values() {
            for dep_id in &task.dependencies {
                if !nodes.contains_key(dep_id) {
                    return Err(DagError::InvalidDependency(format!(
                        "Task '{}' ({}) references non-existent dependency {}",
                        task.title, task.id, dep_id
                    )));
                }
            }
        }

        // Compute parallel execution levels using Kahn's algorithm
        let execution_levels = Self::compute_topological_levels(&nodes)?;

        let dag_id = fnv1a_64(title.as_bytes());

        Ok(ExecutionDag {
            dag_id,
            title: title.to_string(),
            nodes,
            execution_levels,
            total_tasks_count,
        })
    }

    /// Computes parallel execution levels and detects cyclic dependencies using Kahn's algorithm.
    pub fn compute_topological_levels(
        nodes: &HashMap<TaskId, SwarmTaskNode>,
    ) -> Result<Vec<Vec<TaskId>>, DagError> {
        let mut in_degrees: HashMap<TaskId, usize> = HashMap::with_capacity(nodes.len());
        let mut downstream: HashMap<TaskId, Vec<TaskId>> = HashMap::with_capacity(nodes.len());

        for &id in nodes.keys() {
            in_degrees.insert(id, 0);
            downstream.insert(id, Vec::new());
        }

        for task in nodes.values() {
            let deg = task.dependencies.len();
            in_degrees.insert(task.id, deg);
            for &dep_id in &task.dependencies {
                if let Some(d) = downstream.get_mut(&dep_id) {
                    d.push(task.id);
                }
            }
        }

        // Initialize Level 0 with all tasks having in_degree == 0
        let mut current_level: Vec<TaskId> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        if current_level.is_empty() {
            // All nodes have dependencies => whole graph is a cycle
            let titles: Vec<String> = nodes.values().map(|n| n.title.clone()).collect();
            return Err(DagError::CyclicDependencyDetected(titles));
        }

        // Sort Level 0 deterministically by priority and TaskId
        Self::sort_level_tasks(&mut current_level, nodes);

        let mut levels: Vec<Vec<TaskId>> = Vec::new();
        let mut visited_count = 0;

        while !current_level.is_empty() {
            visited_count += current_level.len();
            let mut next_level_set: HashSet<TaskId> = HashSet::new();

            for &task_id in &current_level {
                if let Some(dependents) = downstream.get(&task_id) {
                    for &dep_id in dependents {
                        if let Some(entry) = in_degrees.get_mut(&dep_id) {
                            *entry = entry.saturating_sub(1);
                            if *entry == 0 {
                                next_level_set.insert(dep_id);
                            }
                        }
                    }
                }
            }

            levels.push(current_level);

            let mut next_level: Vec<TaskId> = next_level_set.into_iter().collect();
            Self::sort_level_tasks(&mut next_level, nodes);
            current_level = next_level;
        }

        // Cycle check: If visited tasks < total nodes, an unresolvable cycle exists
        if visited_count < nodes.len() {
            let cyclic_tasks: Vec<String> = in_degrees
                .iter()
                .filter(|&(_, &deg)| deg > 0)
                .filter_map(|(&id, _)| nodes.get(&id).map(|n| n.title.clone()))
                .collect();
            return Err(DagError::CyclicDependencyDetected(cyclic_tasks));
        }

        Ok(levels)
    }

    fn sort_level_tasks(level: &mut [TaskId], nodes: &HashMap<TaskId, SwarmTaskNode>) {
        level.sort_by(|&a, &b| {
            match (nodes.get(&a), nodes.get(&b)) {
                (Some(node_a), Some(node_b)) => node_a
                    .priority
                    .cmp(&node_b.priority)
                    .then_with(|| node_a.title.cmp(&node_b.title))
                    .then_with(|| a.cmp(&b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(&b),
            }
        });
    }

    /// Plans a multi-tier, fullstack sovereign system from a locked specification contract.
    pub fn plan_from_contract(
        contract: &SpecContract,
        app_name: &str,
    ) -> Result<ExecutionDag, DagError> {
        let mut tasks = Vec::with_capacity(10);

        // Tier 1: Architecture & Model Contracts
        let task_arch = SwarmTaskNode::new(
            format!("{}: Architecture Contracts & Types", app_name),
            0,
            format!("Synthesize sovereign contract models for {}", contract.id),
            AgentRole::SystemArchitect,
            TaskPriority::CriticalPath,
            vec!["src/core/contracts.rs".to_string()],
            contract.target_symbols.clone(),
            vec![],
        );
        let id_arch = task_arch.id;
        tasks.push(task_arch);

        let task_models = SwarmTaskNode::new(
            format!("{}: Core Data Structures & State", app_name),
            1,
            "Define state models, entities, and database schemas".to_string(),
            AgentRole::SystemArchitect,
            TaskPriority::High,
            vec!["src/models/mod.rs".to_string()],
            vec!["UserEntity".to_string(), "SessionToken".to_string()],
            vec![id_arch],
        );
        let id_models = task_models.id;
        tasks.push(task_models);

        // Tier 2: Backend Engines & Storage Pools
        let task_backend = SwarmTaskNode::new(
            format!("{}: Backend IPC & Processing Engine", app_name),
            2,
            "Implement high-throughput backend services and IPC channels".to_string(),
            AgentRole::BackendEngineer,
            TaskPriority::High,
            vec!["src/backend/engine.rs".to_string()],
            vec!["run_service".to_string(), "handle_ipc".to_string()],
            vec![id_models],
        );
        let id_backend = task_backend.id;
        tasks.push(task_backend);

        let task_db = SwarmTaskNode::new(
            format!("{}: Persistence & Database Pool", app_name),
            3,
            "Implement transactional storage engine with ACID guarantees".to_string(),
            AgentRole::BackendEngineer,
            TaskPriority::Normal,
            vec!["src/db/pool.rs".to_string()],
            vec!["StoragePool".to_string(), "commit_record".to_string()],
            vec![id_models],
        );
        let id_db = task_db.id;
        tasks.push(task_db);

        // Tier 3: Frontend UI Layouts & Preview Cards
        let task_ui_login = SwarmTaskNode::new(
            format!("{}: Authentication Screen & Form Widgets", app_name),
            4,
            "Build responsive login and signup UI with accessibility tokens".to_string(),
            AgentRole::FrontendEngineer,
            TaskPriority::High,
            vec!["ui/login_screen.dart".to_string(), "ui/widgets/button.dart".to_string()],
            vec!["LoginView".to_string(), "SubmitButton".to_string()],
            vec![id_models],
        );
        let id_ui_login = task_ui_login.id;
        tasks.push(task_ui_login);

        let task_ui_dash = SwarmTaskNode::new(
            format!("{}: Analytics Dashboard & Monitoring View", app_name),
            5,
            "Build real-time metric charts and transaction tables".to_string(),
            AgentRole::FrontendEngineer,
            TaskPriority::Normal,
            vec!["ui/dashboard.dart".to_string()],
            vec!["DashboardView".to_string(), "MetricChart".to_string()],
            vec![id_models],
        );
        let id_ui_dash = task_ui_dash.id;
        tasks.push(task_ui_dash);

        // Tier 4: Security Invariant Audit
        let task_security = SwarmTaskNode::new(
            format!("{}: 32 AST Invariants & Taint Flow Audit", app_name),
            6,
            "Verify complete safe Rust conformance and zero secret leaks".to_string(),
            AgentRole::SecurityAuditor,
            TaskPriority::CriticalPath,
            vec!["src/backend/engine.rs".to_string(), "src/core/contracts.rs".to_string()],
            vec!["verify_safety".to_string()],
            vec![id_backend, id_db],
        );
        let id_security = task_security.id;
        tasks.push(task_security);

        // Tier 5: Quality Assurance Tests
        let task_qa_backend = SwarmTaskNode::new(
            format!("{}: Backend Integration & Stress Suite", app_name),
            7,
            "Run concurrent stress benchmarks and error handling checks".to_string(),
            AgentRole::QualityAssurance,
            TaskPriority::Normal,
            vec!["tests/backend_stress.rs".to_string()],
            vec!["test_stress_throughput".to_string()],
            vec![id_backend, id_db],
        );
        let id_qa_backend = task_qa_backend.id;
        tasks.push(task_qa_backend);

        let task_qa_ui = SwarmTaskNode::new(
            format!("{}: Frontend UI Widget & Layout Tests", app_name),
            8,
            "Test widget render bounds, taps, and form input validation".to_string(),
            AgentRole::QualityAssurance,
            TaskPriority::Normal,
            vec!["test/widget_test.dart".to_string()],
            vec!["test_login_flow".to_string()],
            vec![id_ui_login, id_ui_dash],
        );
        let id_qa_ui = task_qa_ui.id;
        tasks.push(task_qa_ui);

        // Tier 6: Final Integration & ACID Workspace Commit
        let task_orchestrator = SwarmTaskNode::new(
            format!("{}: Swarm Integration & Atomic Commit", app_name),
            9,
            "Finalize swarm merge, verify 0 AST violations, and commit transaction".to_string(),
            AgentRole::IntegrationOrchestrator,
            TaskPriority::CriticalPath,
            vec!["src/lib.rs".to_string()],
            vec!["initialize".to_string()],
            vec![id_security, id_qa_backend, id_qa_ui],
        );
        tasks.push(task_orchestrator);

        Self::build_dag(&format!("DAG for {}", app_name), tasks)
    }
}
