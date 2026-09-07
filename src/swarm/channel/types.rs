//! Data types and domain models for Swarm IPC Channels and Consensus Aggregator.
//!
//! Provides deterministic message identifiers, structured swarm message kinds,
//! bounded ring buffer statistics, and consensus receipt structures for 300-agent swarms.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers and Enums
// ---------------------------------------------------------------------------

/// Deterministic 64-bit identifier for an inter-agent swarm message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MessageId(pub u64);

impl MessageId {
    /// Generates a deterministic `MessageId` from sender, recipient, and sequence counter.
    pub fn new(from_agent: u32, to_agent: u32, seq: u64) -> Self {
        let combined = format!("{}:{}:{}", from_agent, to_agent, seq);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "msg_{:016x}", self.0)
    }
}

/// Enumeration of specialized inter-agent swarm message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwarmMessageType {
    /// Coordinator assigning a task node from the DAG to an agent.
    TaskAssignment,
    /// Agent reporting progress percentage on its active task.
    TaskProgress,
    /// Agent reporting in-memory staged mutation buffer in `stage_tx`.
    MutationStaged,
    /// Agent requesting peer validation or contract compliance verification.
    ValidationRequest,
    /// Response to a validation request with pass/fail verdict.
    ValidationResponse,
    /// Agent successfully completed its task and produced an output hash.
    TaskCompleted,
    /// Agent encountered a fatal error or violation; triggers fault-tolerant reassignment.
    TaskFailed,
    /// Periodic heartbeat confirming agent liveness.
    Heartbeat,
}

impl fmt::Display for SwarmMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskAssignment => write!(f, "TASK_ASSIGNMENT"),
            Self::TaskProgress => write!(f, "TASK_PROGRESS"),
            Self::MutationStaged => write!(f, "MUTATION_STAGED"),
            Self::ValidationRequest => write!(f, "VALIDATION_REQUEST"),
            Self::ValidationResponse => write!(f, "VALIDATION_RESPONSE"),
            Self::TaskCompleted => write!(f, "TASK_COMPLETED"),
            Self::TaskFailed => write!(f, "TASK_FAILED"),
            Self::Heartbeat => write!(f, "HEARTBEAT"),
        }
    }
}

// ---------------------------------------------------------------------------
// Swarm Message
// ---------------------------------------------------------------------------

/// High-performance IPC message passed across the bounded ring channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmMessage {
    pub id: MessageId,
    pub from_agent: u32,
    pub to_agent: u32,
    pub msg_type: SwarmMessageType,
    pub payload_bytes: Vec<u8>,
    pub timestamp_ns: u64,
}

impl SwarmMessage {
    /// Constructs a new `SwarmMessage` with an auto-generated nanosecond timestamp.
    pub fn new(
        from_agent: u32,
        to_agent: u32,
        seq: u64,
        msg_type: SwarmMessageType,
        payload_bytes: Vec<u8>,
    ) -> Self {
        let id = MessageId::new(from_agent, to_agent, seq);
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        Self {
            id,
            from_agent,
            to_agent,
            msg_type,
            payload_bytes,
            timestamp_ns,
        }
    }
}

// ---------------------------------------------------------------------------
// Consensus Status & Receipt
// ---------------------------------------------------------------------------

/// Lifecycle status of the Swarm Consensus Aggregator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsensusStatus {
    /// Actively collecting task completion tokens from parallel agents.
    Collecting,
    /// Quorum of required tasks reached for the current DAG level.
    QuorumReached,
    /// Spatial subtree or contract conflict detected; execution deferred.
    ConflictDetected,
    /// Entire DAG completed; consensus sealed into atomic workspace commit.
    ConsensusSealed,
}

impl fmt::Display for ConsensusStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Collecting => write!(f, "COLLECTING"),
            Self::QuorumReached => write!(f, "QUORUM_REACHED"),
            Self::ConflictDetected => write!(f, "CONFLICT_DETECTED"),
            Self::ConsensusSealed => write!(f, "CONSENSUS_SEALED"),
        }
    }
}

/// Cryptographically verifiable receipt confirming multi-agent swarm consensus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmConsensusReceipt {
    /// Deterministic execution DAG identifier.
    pub dag_id: u64,
    /// List of successfully committed task IDs.
    pub committed_tasks: Vec<u64>,
    /// Cumulative FNV-1a / SHA-256 hash of all committed CST tree states.
    pub final_cst_root_hash: u64,
    /// Total wall-clock duration of swarm execution in milliseconds.
    pub total_execution_time_ms: f64,
    /// Final consensus status.
    pub status: ConsensusStatus,
}

// ---------------------------------------------------------------------------
// Channel Statistics
// ---------------------------------------------------------------------------

/// Real-time throughput and health metrics for the bounded ring channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChannelStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub dropped_messages: u64,
    pub current_depth: usize,
    pub capacity: usize,
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelError {
    #[error("Ring buffer channel is at maximum capacity ({0})")]
    ChannelFull(usize),

    #[error("Ring buffer channel is empty")]
    ChannelEmpty,

    #[error("Agent with ID {0} not found or unregistered")]
    AgentNotFound(u32),

    #[error("Task with ID {0} not found in active DAG")]
    TaskNotFound(u64),

    #[error("Task assignment conflict for task {0}")]
    TaskAssignmentConflict(u64),

    #[error("Quorum not reached for execution level: expected {expected}, actual {actual}")]
    QuorumNotReached { expected: usize, actual: usize },

    #[error("Consensus is already sealed for DAG {0}")]
    ConsensusAlreadySealed(u64),

    #[error("Validation failed during consensus check: {0}")]
    ValidationFailed(String),

    #[error("Internal channel error: {0}")]
    InternalError(String),
}
