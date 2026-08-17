mod lifecycle;
mod monitor;
mod sandbox;
mod task;
pub mod skill_manifest;
pub mod skill_runner;
pub mod skill_registry;
pub mod task_graph;
pub mod reasoning_engine;
pub mod spec_aligner;
pub mod adversarial_qa;
pub mod security_gate;
pub mod verifier;
pub mod ambient_agent;

pub use lifecycle::{AgentManager, agent_lifecycle, kill_agent};
pub use monitor::{AgentMonitor, AgentMonitor as Monitor};
pub use sandbox::{Sandbox, spawn_agent as spawn_sandboxed_agent};
pub use task::{
    AgentHandle, AgentStats, AgentStatus, SandboxConfig, Task, TaskResult, TestResults,
};
pub use skill_manifest::{
    generate_skill_boilerplate, LoadedSkill, SkillLocation, SkillManifest, SkillPermissions,
    SkillRuntime,
};
pub use skill_runner::{kill_process_tree, SkillExecutionResult, SkillRunner};
pub use skill_registry::SkillRegistry;
pub use task_graph::{
    TaskActionPayload, TaskGraph, TaskGraphStatus, TaskNode, TaskNodeResult, TaskNodeStatus,
    TaskNodeType,
};
pub use reasoning_engine::{
    ConstraintKind, DeterministicReasoningEngine, SymbolicConstraint,
};
pub use spec_aligner::{
    SpecAligner, SpecAlignmentReport, SpecAmbiguity, SpecTradeoffOption, TradeoffCategory,
};
pub use adversarial_qa::{
    AdversarialQaAgent, FuzzTestCase, QaReport, QaRiskItem, QaRiskSeverity,
};
pub use security_gate::{
    SecurityGate, SecurityScanResult, SecuritySeverity, SecurityViolation, SecurityViolationCategory,
};
pub use ambient_agent::{AmbientActionResult, AmbientAgentEngine};

use anyhow::Result;
use uuid::Uuid;

pub struct EphemeralAgentManager {
    manager: AgentManager,
}

impl EphemeralAgentManager {
    pub fn new() -> Self {
        Self {
            manager: AgentManager::new(),
        }
    }

    pub async fn spawn_agent(&self, task: Task) -> Result<AgentHandle> {
        self.manager.spawn_agent(task).await
    }

    pub async fn kill_agent(&self, agent_id: Uuid) -> Result<()> {
        self.manager.kill_agent(agent_id).await
    }

    pub fn agent_status(&self, agent_id: Uuid) -> Option<AgentStatus> {
        self.manager.agent_status(agent_id)
    }

    pub fn list_active_agents(&self) -> Vec<AgentHandle> {
        self.manager.list_active_agents()
    }

    pub async fn monitor_agent(&self, agent_id: Uuid) -> Result<AgentStats> {
        self.manager.monitor_agent(agent_id).await
    }

    pub async fn execute_task(&self, task: Task) -> Result<TaskResult> {
        self.manager.execute_task(task).await
    }
}

impl Default for EphemeralAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    #[tokio::test]
    async fn test_spawn_simple_task() {
        let manager = EphemeralAgentManager::new();
        #[cfg(target_os = "windows")]
        let cmd = "echo hello world";
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo 'hello world'";

        let task = Task::new(cmd.to_string(), "bash".to_string())
            .with_timeout(10)
            .with_memory(128);

        let handle = manager.spawn_agent(task).await.unwrap();
        
        assert!(matches!(handle.status, AgentStatus::Created | AgentStatus::Starting | AgentStatus::Running | AgentStatus::Stopped));
        
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        let status = manager.agent_status(handle.id);
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn test_kill_agent() {
        let manager = EphemeralAgentManager::new();
        #[cfg(target_os = "windows")]
        let cmd = "ping 127.0.0.1 -n 10";
        #[cfg(not(target_os = "windows"))]
        let cmd = "sleep 10";

        let task = Task::new(cmd.to_string(), "bash".to_string())
            .with_timeout(10)
            .with_memory(128);

        let handle = manager.spawn_agent(task).await.unwrap();
        let result = manager.kill_agent(handle.id).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_active_agents() {
        let manager = EphemeralAgentManager::new();
        let active = manager.list_active_agents();
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_execute_echo() {
        let manager = EphemeralAgentManager::new();
        let task = Task::new("echo test output".to_string(), "bash".to_string())
            .with_timeout(10)
            .with_memory(128);

        let result = manager.execute_task(task).await.unwrap();
        
        assert!(result.success);
        assert!(result.output.contains("test output"));
    }

    #[tokio::test]
    async fn test_execute_failing_command() {
        let manager = EphemeralAgentManager::new();
        let task = Task::new("exit 1".to_string(), "bash".to_string())
            .with_timeout(10)
            .with_memory(128);

        let result = manager.execute_task(task).await.unwrap();
        
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
    }
}