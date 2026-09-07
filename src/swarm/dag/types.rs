//! Data types and models for the Autonomous Swarm Task Planner and DAG Engine.
//!
//! Provides deterministic task identification, specialized agent role matrices,
//! parallel execution levels, and conflict-aware task nodes for up to 300 swarm agents.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers and Enums
// ---------------------------------------------------------------------------

/// Deterministic 64-bit FNV-1a identifier for a swarm task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Generates a deterministic `TaskId` from title and index.
    pub fn new(title: &str, index: usize) -> Self {
        let combined = format!("{}:{}", title, index);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task_{:016x}", self.0)
    }
}

/// Agent ID in the 300-agent autonomous swarm (0..299).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgentId(pub u32);

impl AgentId {
    pub const MAX_SWARM_SIZE: u32 = 300;

    pub fn new(id: u32) -> Option<Self> {
        if id < Self::MAX_SWARM_SIZE {
            Some(Self(id))
        } else {
            None
        }
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "agent_{:03}", self.0)
    }
}

/// Specialized agent roles across the sovereign swarm matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    SystemArchitect,
    BackendEngineer,
    FrontendEngineer,
    SecurityAuditor,
    QualityAssurance,
    IntegrationOrchestrator,
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SystemArchitect => "System Architect (Contracts & Types)",
            Self::BackendEngineer => "Backend Engineer (Engines & IPC)",
            Self::FrontendEngineer => "Frontend Engineer (UI & Layouts)",
            Self::SecurityAuditor => "Security Auditor (32 Invariants)",
            Self::QualityAssurance => "Quality Assurance (Tests & Benchmarks)",
            Self::IntegrationOrchestrator => "Integration Orchestrator (DAG & Swarm)",
        };
        write!(f, "{}", s)
    }
}

/// Task execution priority in the topological schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    CriticalPath = 0,
    High = 1,
    Normal = 2,
    Background = 3,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CriticalPath => write!(f, "CriticalPath"),
            Self::High => write!(f, "High"),
            Self::Normal => write!(f, "Normal"),
            Self::Background => write!(f, "Background"),
        }
    }
}

/// Task execution lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,
    Assigned,
    Staged,
    Committed,
    Failed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Ready => write!(f, "Ready"),
            Self::Assigned => write!(f, "Assigned"),
            Self::Staged => write!(f, "Staged"),
            Self::Committed => write!(f, "Committed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Swarm Task Node & Execution DAG
// ---------------------------------------------------------------------------

/// A discrete task node in the directed acyclic graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmTaskNode {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub role: AgentRole,
    pub priority: TaskPriority,
    pub target_files: Vec<String>,
    pub target_symbols: Vec<String>,
    pub dependencies: Vec<TaskId>,
    pub status: TaskStatus,
    pub assigned_agent: Option<AgentId>,
}

impl SwarmTaskNode {
    /// Constructs a new task node with dependencies.
    pub fn new(
        title: impl Into<String>,
        index: usize,
        description: impl Into<String>,
        role: AgentRole,
        priority: TaskPriority,
        target_files: Vec<String>,
        target_symbols: Vec<String>,
        dependencies: Vec<TaskId>,
    ) -> Self {
        let title_str = title.into();
        let id = TaskId::new(&title_str, index);
        Self {
            id,
            title: title_str,
            description: description.into(),
            role,
            priority,
            target_files,
            target_symbols,
            dependencies,
            status: TaskStatus::Pending,
            assigned_agent: None,
        }
    }
}

/// Topologically sorted, parallelized execution DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDag {
    pub dag_id: u64,
    pub title: String,
    pub nodes: HashMap<TaskId, SwarmTaskNode>,
    /// Tasks partitioned into sequential levels where tasks in the same level execute in parallel.
    pub execution_levels: Vec<Vec<TaskId>>,
    pub total_tasks_count: usize,
}

impl ExecutionDag {
    /// Returns the maximum parallel concurrency (width) of the DAG.
    pub fn max_concurrency(&self) -> usize {
        self.execution_levels.iter().map(|lvl| lvl.len()).max().unwrap_or(0)
    }

    /// Returns the number of execution tiers (depth) of the DAG.
    pub fn total_levels(&self) -> usize {
        self.execution_levels.len()
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DagError {
    #[error("Cyclic dependency detected in task graph involving tasks: {0:?}")]
    CyclicDependencyDetected(Vec<String>),

    #[error("Task not found with ID: {0}")]
    TaskNotFound(TaskId),

    #[error("Conflict detected: {0}")]
    ConflictDetected(String),

    #[error("Cannot plan an empty task DAG")]
    EmptyDag,

    #[error("Invalid dependency reference: {0}")]
    InvalidDependency(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Internal DAG planner error: {0}")]
    InternalError(String),
}
