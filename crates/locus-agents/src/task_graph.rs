//! DAG-based Task Graph Engine for Goal Decomposition and Autonomous Step Execution
//!
//! Provides Kahn's algorithm topological sorting, cycle detection, dynamic node
//! editing, ready-state computation, and goal-to-DAG decomposition.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeType {
    CodeEdit,
    ShellCommand,
    CreateFile,
    SkillExecution,
    Analysis,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskActionPayload {
    pub target_file: Option<String>,
    pub proposed_content: Option<String>,
    pub search_replace_block: Option<String>,
    pub shell_command: Option<String>,
    pub skill_name: Option<String>,
    pub skill_params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskNodeResult {
    pub success: bool,
    pub output: String,
    pub diff_preview: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub node_type: TaskNodeType,
    pub dependencies: Vec<String>,
    pub status: TaskNodeStatus,
    pub payload: TaskActionPayload,
    pub result: Option<TaskNodeResult>,
    pub auto_execute: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TaskNode {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        node_type: TaskNodeType,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            node_type,
            dependencies: Vec::new(),
            status: TaskNodeStatus::Pending,
            payload: TaskActionPayload::default(),
            result: None,
            auto_execute: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_payload(mut self, payload: TaskActionPayload) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_auto_execute(mut self, auto: bool) -> Self {
        self.auto_execute = auto;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskGraphStatus {
    Draft,
    Planning,
    InProgress,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    pub id: String,
    pub goal: String,
    pub nodes: Vec<TaskNode>,
    pub status: TaskGraphStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TaskGraph {
    pub fn new(goal: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            goal: goal.into(),
            nodes: Vec::new(),
            status: TaskGraphStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Adds a node to the graph and refreshes readiness
    pub fn add_node(&mut self, node: TaskNode) {
        self.nodes.push(node);
        self.updated_at = chrono::Utc::now();
        self.refresh_readiness();
    }

    /// Edits an existing node's title, description, or payload dynamically
    pub fn update_node(
        &mut self,
        node_id: &str,
        title: Option<String>,
        description: Option<String>,
        payload: Option<TaskActionPayload>,
        status: Option<TaskNodeStatus>,
    ) -> Result<()> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow!("Task node '{}' not found in graph", node_id))?;

        if let Some(t) = title {
            node.title = t;
        }
        if let Some(d) = description {
            node.description = d;
        }
        if let Some(p) = payload {
            node.payload = p;
        }
        if let Some(s) = status {
            node.status = s;
        }
        node.updated_at = chrono::Utc::now();
        self.updated_at = chrono::Utc::now();
        self.refresh_readiness();
        Ok(())
    }

    /// Deletes a node and cleans up dependency references from downstream nodes
    pub fn delete_node(&mut self, node_id: &str) -> bool {
        let initial_len = self.nodes.len();
        self.nodes.retain(|n| n.id != node_id);
        if self.nodes.len() != initial_len {
            // Remove node_id from other nodes' dependencies
            for node in &mut self.nodes {
                node.dependencies.retain(|d| d != node_id);
            }
            self.updated_at = chrono::Utc::now();
            self.refresh_readiness();
            true
        } else {
            false
        }
    }

    /// Kahn's Algorithm for Topological Sorting & Cycle Detection
    /// Returns the ordered node IDs or an Err if a cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for node in &self.nodes {
            in_degree.insert(node.id.clone(), 0);
            adjacency.insert(node.id.clone(), Vec::new());
        }

        // Build adjacency: dependency -> dependent (u -> v)
        for node in &self.nodes {
            for dep in &node.dependencies {
                if in_degree.contains_key(dep) {
                    adjacency.get_mut(dep).unwrap().push(node.id.clone());
                    *in_degree.get_mut(&node.id).unwrap() += 1;
                }
            }
        }

        // Queue all nodes with in_degree == 0
        let mut queue: VecDeque<String> = VecDeque::new();
        for (id, deg) in &in_degree {
            if *deg == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut sorted: Vec<String> = Vec::new();

        while let Some(u) = queue.pop_front() {
            sorted.push(u.clone());
            if let Some(neighbors) = adjacency.get(&u) {
                for v in neighbors {
                    if let Some(deg) = in_degree.get_mut(v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            let visited: HashSet<String> = sorted.into_iter().collect();
            let cyclic_nodes: Vec<String> = self
                .nodes
                .iter()
                .filter(|n| !visited.contains(&n.id))
                .map(|n| n.id.clone())
                .collect();
            return Err(anyhow!(
                "Circular dependency cycle detected among task nodes: {:?}",
                cyclic_nodes
            ));
        }

        Ok(sorted)
    }

    /// Recalculates `Ready` vs `Pending` status based on dependency completion
    pub fn refresh_readiness(&mut self) {
        let completed_ids: HashSet<String> = self
            .nodes
            .iter()
            .filter(|n| n.status == TaskNodeStatus::Completed || n.status == TaskNodeStatus::Skipped)
            .map(|n| n.id.clone())
            .collect();

        for node in &mut self.nodes {
            if node.status == TaskNodeStatus::Pending {
                let all_deps_met = node
                    .dependencies
                    .iter()
                    .all(|dep| completed_ids.contains(dep));
                if all_deps_met {
                    node.status = TaskNodeStatus::Ready;
                }
            } else if node.status == TaskNodeStatus::Ready {
                let all_deps_met = node
                    .dependencies
                    .iter()
                    .all(|dep| completed_ids.contains(dep));
                if !all_deps_met {
                    node.status = TaskNodeStatus::Pending;
                }
            }
        }

        // Update overall graph status
        let total = self.nodes.len();
        if total == 0 {
            self.status = TaskGraphStatus::Draft;
            return;
        }

        let completed_count = self
            .nodes
            .iter()
            .filter(|n| n.status == TaskNodeStatus::Completed || n.status == TaskNodeStatus::Skipped)
            .count();
        let running_count = self
            .nodes
            .iter()
            .filter(|n| n.status == TaskNodeStatus::Running)
            .count();
        let failed_count = self
            .nodes
            .iter()
            .filter(|n| n.status == TaskNodeStatus::Failed)
            .count();

        if completed_count == total {
            self.status = TaskGraphStatus::Completed;
        } else if failed_count > 0 {
            self.status = TaskGraphStatus::Failed;
        } else if running_count > 0 || completed_count > 0 {
            self.status = TaskGraphStatus::InProgress;
        } else {
            self.status = TaskGraphStatus::Planning;
        }
    }

    /// Returns list of nodes that are currently `Ready` to run
    pub fn get_ready_nodes(&self) -> Vec<&TaskNode> {
        self.nodes
            .iter()
            .filter(|n| n.status == TaskNodeStatus::Ready)
            .collect()
    }

    /// Marks a node as Running
    pub fn mark_running(&mut self, node_id: &str) -> Result<()> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow!("Node '{}' not found", node_id))?;
        node.status = TaskNodeStatus::Running;
        node.updated_at = chrono::Utc::now();
        self.status = TaskGraphStatus::InProgress;
        Ok(())
    }

    /// Marks a node as Completed with results and refreshes downstream readiness
    pub fn mark_completed(&mut self, node_id: &str, result: TaskNodeResult) -> Result<()> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow!("Node '{}' not found", node_id))?;
        node.status = TaskNodeStatus::Completed;
        node.result = Some(result);
        node.updated_at = chrono::Utc::now();
        self.refresh_readiness();
        Ok(())
    }

    /// Marks a node as Failed
    pub fn mark_failed(&mut self, node_id: &str, error: &str) -> Result<()> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow!("Node '{}' not found", node_id))?;
        node.status = TaskNodeStatus::Failed;
        node.result = Some(TaskNodeResult {
            success: false,
            output: String::new(),
            diff_preview: None,
            error: Some(error.to_string()),
            duration_ms: 0,
        });
        node.updated_at = chrono::Utc::now();
        self.refresh_readiness();
        Ok(())
    }

    /// Intelligent rule-based and phased Goal Decomposition into a structured DAG
    pub fn decompose_goal(goal: &str, files_context: &[String]) -> Self {
        let mut graph = Self::new(goal);
        let goal_lower = goal.to_lowercase();

        // 1. Initial Analysis Step
        let step1_id = "step-1-analysis".to_string();
        let step1 = TaskNode::new(
            &step1_id,
            "Context & Requirements Analysis",
            format!("Inspect workspace context and determine dependencies for goal: '{}'", goal),
            TaskNodeType::Analysis,
        );
        graph.add_node(step1);

        // 2. Implementation Steps
        let step2_id = "step-2-implementation".to_string();
        let target_file = files_context.first().cloned();
        let mut step2 = TaskNode::new(
            &step2_id,
            "Core Implementation",
            format!("Apply code modifications and architectural changes to satisfy '{}'", goal),
            TaskNodeType::CodeEdit,
        ).with_dependencies(vec![step1_id.clone()]);

        if let Some(file) = target_file {
            step2.payload.target_file = Some(file);
        }
        graph.add_node(step2);

        // 3. Shell / Dependency Step (if indicated in goal)
        let mut last_dep = step2_id.clone();
        if goal_lower.contains("install") || goal_lower.contains("dependency") || goal_lower.contains("package") || goal_lower.contains("cargo") || goal_lower.contains("npm") {
            let step3_id = "step-3-dependencies".to_string();
            let step3 = TaskNode::new(
                &step3_id,
                "Package & Dependency Setup",
                "Execute build or dependency manifest update",
                TaskNodeType::ShellCommand,
            ).with_dependencies(vec![step1_id]);
            last_dep = step3_id.clone();
            graph.add_node(step3);
        }

        // 4. Verification / Test Step
        let step_test_id = "step-test-verification".to_string();
        let step_test = TaskNode::new(
            &step_test_id,
            "Verification & Automated Tests",
            "Run unit and integration tests to validate code correctness and prevent regressions",
            TaskNodeType::Test,
        ).with_dependencies(vec![step2_id, last_dep]);
        graph.add_node(step_test);

        graph.refresh_readiness();
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_topological_sort_valid() {
        let mut graph = TaskGraph::new("Test DAG Valid");

        let n1 = TaskNode::new("n1", "Step 1", "Initial setup", TaskNodeType::Analysis);
        let n2 = TaskNode::new("n2", "Step 2", "Core logic", TaskNodeType::CodeEdit)
            .with_dependencies(vec!["n1".to_string()]);
        let n3 = TaskNode::new("n3", "Step 3", "Tests", TaskNodeType::Test)
            .with_dependencies(vec!["n2".to_string()]);

        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);

        let sorted = graph.topological_sort().expect("Should sort successfully");
        assert_eq!(sorted, vec!["n1", "n2", "n3"]);

        // n1 should be Ready, n2 and n3 should be Pending
        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Ready);
        assert_eq!(graph.nodes[1].status, TaskNodeStatus::Pending);
        assert_eq!(graph.nodes[2].status, TaskNodeStatus::Pending);
    }

    #[test]
    fn test_dag_cycle_detection() {
        let mut graph = TaskGraph::new("Test DAG Cycle");

        // n1 -> n2 -> n3 -> n1 (cycle!)
        let n1 = TaskNode::new("n1", "Step 1", "Initial", TaskNodeType::Analysis)
            .with_dependencies(vec!["n3".to_string()]);
        let n2 = TaskNode::new("n2", "Step 2", "Edit", TaskNodeType::CodeEdit)
            .with_dependencies(vec!["n1".to_string()]);
        let n3 = TaskNode::new("n3", "Step 3", "Test", TaskNodeType::Test)
            .with_dependencies(vec!["n2".to_string()]);

        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);

        let err = graph.topological_sort().unwrap_err();
        assert!(err.to_string().contains("Circular dependency cycle"));
    }

    #[test]
    fn test_decompose_goal_phases() {
        let graph = TaskGraph::decompose_goal(
            "Add Redis cache and install redis package",
            &["src/cache.rs".to_string()],
        );

        assert!(graph.nodes.len() >= 3);
        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Ready);
        let sort_res = graph.topological_sort();
        assert!(sort_res.is_ok());
    }

    #[test]
    fn test_node_execution_transitions() {
        let mut graph = TaskGraph::new("Transition Test");
        let n1 = TaskNode::new("n1", "Step 1", "Desc", TaskNodeType::Analysis);
        let n2 = TaskNode::new("n2", "Step 2", "Desc", TaskNodeType::CodeEdit)
            .with_dependencies(vec!["n1".to_string()]);

        graph.add_node(n1);
        graph.add_node(n2);

        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Ready);
        assert_eq!(graph.nodes[1].status, TaskNodeStatus::Pending);

        graph.mark_running("n1").unwrap();
        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Running);

        graph
            .mark_completed(
                "n1",
                TaskNodeResult {
                    success: true,
                    output: "done".to_string(),
                    diff_preview: None,
                    error: None,
                    duration_ms: 50,
                },
            )
            .unwrap();

        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Completed);
        // Downstream n2 should now automatically become Ready!
        assert_eq!(graph.nodes[1].status, TaskNodeStatus::Ready);
    }

    #[test]
    fn test_dynamic_node_update_and_delete() {
        let mut graph = TaskGraph::new("Dynamic Edit Test");
        let n1 = TaskNode::new("n1", "Original Title", "Original Desc", TaskNodeType::Analysis);
        let n2 = TaskNode::new("n2", "Step 2", "Desc", TaskNodeType::Test)
            .with_dependencies(vec!["n1".to_string()]);

        graph.add_node(n1);
        graph.add_node(n2);

        graph
            .update_node(
                "n1",
                Some("Updated Title".to_string()),
                Some("Updated Desc".to_string()),
                None,
                None,
            )
            .unwrap();

        assert_eq!(graph.nodes[0].title, "Updated Title");
        assert_eq!(graph.nodes[0].description, "Updated Desc");

        // Delete n1 -> n2's dependencies should be cleaned up
        let deleted = graph.delete_node("n1");
        assert!(deleted);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes[0].dependencies.is_empty());
        // Since dependencies are empty, n2 is now Ready
        assert_eq!(graph.nodes[0].status, TaskNodeStatus::Ready);
    }
}
