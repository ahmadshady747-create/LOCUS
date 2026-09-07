//! Swarm OS — Autonomous Multi-Agent Coordination, Topological Task DAGs & Blast Radius Partitioning.

#![forbid(unsafe_code)]

pub mod channel;
pub mod dag;
pub mod occ;

pub use channel::{
    ChannelError, ChannelStats, ConsensusStatus, MessageId, SwarmConsensusAggregator,
    SwarmConsensusReceipt, SwarmMessage, SwarmMessageType, SwarmRingChannel,
    TaskCompletionRecord, DEFAULT_RING_CAPACITY,
};
pub use dag::{
    AgentId, AgentRole, DagError, ExecutionDag, SubtreeConflictPartitioner, SwarmDagPlanner,
    SwarmTaskNode, TaskId, TaskPriority, TaskStatus,
};
pub use occ::{
    AgentShadowBuffer, BufferId, OccError, OccLockState, OccValidationEngine,
    OccValidationResult, ShadowBufferManager, StagedMutation, SubtreeSpan,
    SwarmWorkspaceManager, VersionTag, MAX_SWARM_BUFFERS,
};
