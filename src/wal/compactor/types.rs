//! Data types and models for the WAL Compactor & Zero-Copy Checkpoint Engine.
//!
//! Provides deterministic checkpoint identifiers, compaction policies,
//! space reclamation statistics, and cryptographic snapshot models.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers and Policies
// ---------------------------------------------------------------------------

/// Deterministic 64-bit identifier for a WAL checkpoint snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CheckpointId(pub u64);

impl CheckpointId {
    /// Constructs a `CheckpointId` from sequence number and timestamp.
    pub fn new(sequence: u64, timestamp_ms: u64) -> Self {
        let combined = format!("checkpoint:{}:{}", sequence, timestamp_ms);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chk_{:016x}", self.0)
    }
}

/// Trigger policy for automated background log compaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompactorPolicy {
    /// Trigger compaction when total uncompacted log segments exceed this byte size.
    SegmentSizeThreshold(usize),
    /// Trigger compaction when number of committed transactions exceeds this threshold.
    TransactionCountThreshold(usize),
    /// Periodic compaction interval in milliseconds.
    PeriodicIntervalMs(u64),
    /// Manual compaction on explicit trigger only.
    Manual,
}

impl Default for CompactorPolicy {
    fn default() -> Self {
        Self::SegmentSizeThreshold(64 * 1024 * 1024) // 64 MB
    }
}

// ---------------------------------------------------------------------------
// Compaction Statistics
// ---------------------------------------------------------------------------

/// Metric report generated following a compaction cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactorStats {
    /// Total number of old segment files purged or summarized.
    pub compacted_segments_count: usize,
    /// Disk space reclaimed in bytes.
    pub bytes_reclaimed: u64,
    /// Number of surviving (latest-state) mutations retained.
    pub surviving_mutations_count: usize,
    /// Total historical input mutations analyzed before deduplication.
    pub total_input_mutations: usize,
    /// Total duration of the compaction cycle in milliseconds.
    pub compaction_duration_ms: f64,
}

impl CompactorStats {
    /// Percentage of space/mutation reduction achieved by compaction.
    pub fn reduction_percentage(&self) -> f64 {
        if self.total_input_mutations == 0 {
            0.0
        } else {
            let reduced = self.total_input_mutations.saturating_sub(self.surviving_mutations_count);
            (reduced as f64 / self.total_input_mutations as f64) * 100.0
        }
    }
}

// ---------------------------------------------------------------------------
// Checkpoint Snapshot
// ---------------------------------------------------------------------------

/// Self-contained, cryptographically verified binary checkpoint snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointSnapshot {
    /// Deterministic checkpoint identifier.
    pub id: CheckpointId,
    /// Monotonic sequence number corresponding to the latest committed transaction.
    pub sequence_number: u64,
    /// CST root content hash at the time of checkpointing.
    pub cst_root_hash: u64,
    /// Serialized snapshot state payload.
    pub serialized_state: Vec<u8>,
    /// FIPS 180-4 SHA-256 hexadecimal checksum.
    pub checksum_sha256: String,
    /// Castagnoli CRC32-C checksum.
    pub checksum_crc32c: u32,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Compactor Configuration
// ---------------------------------------------------------------------------

/// Configuration for the WAL compactor and checkpoint engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactorConfig {
    pub wal_directory: PathBuf,
    pub checkpoint_directory: PathBuf,
    pub policy: CompactorPolicy,
    pub max_retained_checkpoints: usize,
}

impl Default for CompactorConfig {
    fn default() -> Self {
        Self {
            wal_directory: PathBuf::from(".locus_wal"),
            checkpoint_directory: PathBuf::from(".locus_wal/snapshots"),
            policy: CompactorPolicy::default(),
            max_retained_checkpoints: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactorError {
    #[error("I/O error during log compaction: {0}")]
    IoError(String),

    #[error("Corrupted WAL segment encountered: {0}")]
    CorruptedSegment(String),

    #[error("Checksum mismatch for snapshot: expected {expected}, calculated {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("CRC32-C checksum mismatch for snapshot: expected {expected:#010x}, calculated {actual:#010x}")]
    CrcMismatch { expected: u32, actual: u32 },

    #[error("Log contains no committed mutations to compact")]
    EmptyLog,

    #[error("Checkpoint snapshot with sequence {0} not found")]
    CheckpointNotFound(u64),

    #[error("Compaction already in progress")]
    CompactionInProgress,
}
