//! Subtree Optimistic Concurrency Control (OCC) Types and Domain Models.
//!
//! Enforces zero global write-locks, spatial subtree span overlap detection,
//! isolated shadow buffers, and monotonic version tagging for up to 300 swarm agents.

#![forbid(unsafe_code)]

use crate::cst::polyglot::types::GreenTreeRoot;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Buffer & Version Identifiers
// ---------------------------------------------------------------------------

/// Deterministic 64-bit identifier for an agent's shadow buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BufferId(pub u64);

impl BufferId {
    /// Constructs a `BufferId` using FNV-1a hashing from agent ID and a monotonic counter.
    pub fn new(agent_id: u32, sequence: u64) -> Self {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for byte in agent_id.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for byte in sequence.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        Self(hash)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "buf_{:016x}", self.0)
    }
}

/// Monotonic content-addressed version tag for AST subtrees and files ($V_n$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VersionTag(pub u64);

impl VersionTag {
    pub const GENESIS: Self = Self(0);

    pub fn new(v: u64) -> Self {
        Self(v)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for VersionTag {
    fn default() -> Self {
        Self::GENESIS
    }
}

impl fmt::Display for VersionTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "V_{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Transaction Lock States
// ---------------------------------------------------------------------------

/// Lifecycle states of an OCC transaction operating within an isolated shadow buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OccLockState {
    /// Unlocked, fresh buffer allocated.
    Unlocked,
    /// Optimistic read phase, recording accessed subtrees into read_set.
    OptimisticRead,
    /// Validation phase, testing for spatial overlap and version drift.
    ValidationPhase,
    /// Atomically committed to the master CST tree ($V_n \to V_{n+1}$).
    Committed,
    /// Aborted due to conflict, shadow buffer discarded.
    Aborted,
}

// ---------------------------------------------------------------------------
// Subtree Span (Spatial Coordinate)
// ---------------------------------------------------------------------------

/// Fine-grained spatial coordinate defining an AST node or subtree in the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubtreeSpan {
    /// Normalized workspace-relative file path (e.g. `src/auth/session.rs`).
    pub file_path: String,
    /// Content-addressed symbol hash or node identifier (0 if entire file).
    pub symbol_id: u64,
    /// Byte range `[start, end)` within the file.
    pub byte_range: (usize, usize),
}

impl SubtreeSpan {
    pub fn new(file_path: impl Into<String>, symbol_id: u64, byte_range: (usize, usize)) -> Self {
        Self {
            file_path: file_path.into(),
            symbol_id,
            byte_range,
        }
    }

    /// Checks if this subtree span spatially overlaps with another span.
    pub fn overlaps(&self, other: &Self) -> bool {
        if self.file_path != other.file_path {
            return false;
        }

        // If symbol IDs match and are non-zero, direct conflict
        if self.symbol_id != 0 && self.symbol_id == other.symbol_id {
            return true;
        }

        // Spatial byte range intersection: [s1, e1) and [s2, e2) overlap if s1 < e2 && s2 < e1
        self.byte_range.0 < other.byte_range.1 && other.byte_range.0 < self.byte_range.1
    }
}

impl fmt::Display for SubtreeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{:#x}[{}..{}]",
            self.file_path, self.symbol_id, self.byte_range.0, self.byte_range.1
        )
    }
}

// ---------------------------------------------------------------------------
// Staged Mutation
// ---------------------------------------------------------------------------

/// An in-memory atomic mutation staged within an agent's shadow buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedMutation {
    pub mutation_id: u64,
    pub target_span: SubtreeSpan,
    pub replacement_cst: Option<GreenTreeRoot>,
    pub replacement_source: Option<String>,
    pub base_version: VersionTag,
}

impl StagedMutation {
    pub fn new(
        mutation_id: u64,
        target_span: SubtreeSpan,
        replacement_source: impl Into<String>,
        base_version: VersionTag,
    ) -> Self {
        Self {
            mutation_id,
            target_span,
            replacement_cst: None,
            replacement_source: Some(replacement_source.into()),
            base_version,
        }
    }

    pub fn with_cst(mut self, cst: GreenTreeRoot) -> Self {
        self.replacement_cst = Some(cst);
        self
    }
}

// ---------------------------------------------------------------------------
// Agent Shadow Buffer
// ---------------------------------------------------------------------------

/// Isolated virtual overlay buffer allocated to a single swarm agent.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentShadowBuffer {
    pub buffer_id: BufferId,
    pub agent_id: u32,
    pub staged_mutations: Vec<StagedMutation>,
    pub read_set: HashSet<SubtreeSpan>,
    pub write_set: HashSet<SubtreeSpan>,
    pub created_at_ms: u64,
    pub state: OccLockState,
}

impl AgentShadowBuffer {
    pub fn new(buffer_id: BufferId, agent_id: u32, created_at_ms: u64) -> Self {
        Self {
            buffer_id,
            agent_id,
            staged_mutations: Vec::new(),
            read_set: HashSet::new(),
            write_set: HashSet::new(),
            created_at_ms,
            state: OccLockState::OptimisticRead,
        }
    }

    pub fn stage(&mut self, mutation: StagedMutation) {
        self.write_set.insert(mutation.target_span.clone());
        self.staged_mutations.push(mutation);
    }

    pub fn record_read(&mut self, span: SubtreeSpan) {
        self.read_set.insert(span);
    }
}

// ---------------------------------------------------------------------------
// OCC Validation Result & Error Domain
// ---------------------------------------------------------------------------

/// Outcome of the 3-Phase OCC validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OccValidationResult {
    pub is_valid: bool,
    pub conflict_span: Option<SubtreeSpan>,
    pub current_version: VersionTag,
    pub latency_ms: f64,
}

impl OccValidationResult {
    pub fn valid(current_version: VersionTag, latency_ms: f64) -> Self {
        Self {
            is_valid: true,
            conflict_span: None,
            current_version,
            latency_ms,
        }
    }

    pub fn conflict(conflict_span: SubtreeSpan, current_version: VersionTag, latency_ms: f64) -> Self {
        Self {
            is_valid: false,
            conflict_span: Some(conflict_span),
            current_version,
            latency_ms,
        }
    }
}

/// Errors raised during optimistic concurrency control.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum OccError {
    #[error("Shadow buffer '{0}' not found in active registry")]
    BufferNotFound(BufferId),

    #[error("OCC conflict detected on span '{span}': base version {base_version} != active version {active_version}")]
    ConflictDetected {
        span: SubtreeSpan,
        base_version: VersionTag,
        active_version: VersionTag,
    },

    #[error("Transaction aborted: {0}")]
    TransactionAborted(String),

    #[error("Invalid staged mutation: {0}")]
    InvalidMutation(String),

    #[error("Swarm shadow buffer capacity exceeded ({0} active buffers)")]
    CapacityExceeded(usize),
}
