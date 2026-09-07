//! Domain models and types for Zero-Loss Crash Recovery & Deterministic Log Replay Engine.
//!
//! Provides recovery modes, transaction reconstruction models, payload serialization,
//! and crash recovery receipts in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::wal::log::types::{TxId, WalError};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Recovery Modes & Status
// ---------------------------------------------------------------------------

/// Execution mode for crash recovery and log replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryMode {
    /// Replays all available log segments from the beginning.
    FullReplay,
    /// Fast recovery: Starts from the latest valid checkpoint and ignores older segments.
    CheckpointOnly,
    /// Audits and validates log integrity without applying mutations.
    DryRunAudit,
}

impl Default for RecoveryMode {
    fn default() -> Self {
        Self::FullReplay
    }
}

/// Recovery lifecycle status of an individual scanned transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionRecoveryStatus {
    /// Fully committed transaction (has BeginTx and CommitTx). Replayed into master state.
    Committed,
    /// Uncommitted or explicitly aborted transaction. Rolled back with zero disk/heap mutation.
    RolledBack,
    /// Corrupted due to incomplete / torn write at crash boundary.
    CorruptedTornWrite,
}

impl fmt::Display for TransactionRecoveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed => write!(f, "COMMITTED"),
            Self::RolledBack => write!(f, "ROLLED_BACK"),
            Self::CorruptedTornWrite => write!(f, "CORRUPTED_TORN_WRITE"),
        }
    }
}

// ---------------------------------------------------------------------------
// Mutation & Transaction Payloads
// ---------------------------------------------------------------------------

/// Structured payload for a `SubtreeMutation` WAL frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalMutationPayload {
    pub file_path: String,
    pub symbol_id: u64,
    pub byte_range: (usize, usize),
    pub replacement_content: String,
    pub version_tag: u64,
}

impl WalMutationPayload {
    pub fn new(
        file_path: impl Into<String>,
        symbol_id: u64,
        byte_range: (usize, usize),
        replacement_content: impl Into<String>,
        version_tag: u64,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            symbol_id,
            byte_range,
            replacement_content: replacement_content.into(),
            version_tag,
        }
    }

    /// Serializes payload to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, WalError> {
        serde_json::to_vec(self)
            .map_err(|e| WalError::CorruptedFrame(format!("Payload serialization failed: {}", e)))
    }

    /// Deserializes payload from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        serde_json::from_slice(bytes)
            .map_err(|e| WalError::CorruptedFrame(format!("Payload deserialization failed: {}", e)))
    }
}

/// Structured payload for a `CheckpointMarker` WAL frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalCheckpointPayload {
    pub checkpoint_seq: u64,
    pub timestamp_ms: u64,
    pub active_version: u64,
}

impl WalCheckpointPayload {
    pub fn new(checkpoint_seq: u64, timestamp_ms: u64, active_version: u64) -> Self {
        Self {
            checkpoint_seq,
            timestamp_ms,
            active_version,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, WalError> {
        serde_json::to_vec(self).map_err(|e| {
            WalError::CorruptedFrame(format!("Checkpoint serialization failed: {}", e))
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        serde_json::from_slice(bytes).map_err(|e| {
            WalError::CorruptedFrame(format!("Checkpoint deserialization failed: {}", e))
        })
    }
}

/// Summary of a single reconstructed transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredTransaction {
    pub tx_id: TxId,
    pub status: TransactionRecoveryStatus,
    pub applied_mutations_count: usize,
    pub affected_files: Vec<String>,
}

// ---------------------------------------------------------------------------
// Recovery Receipt
// ---------------------------------------------------------------------------

/// Deterministic audit receipt produced upon completion of crash recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryReceipt {
    pub scanned_segments_count: usize,
    pub recovered_transactions: Vec<RecoveredTransaction>,
    pub total_mutations_replayed: usize,
    pub uncommitted_rolled_back: usize,
    pub last_valid_checkpoint_seq: u64,
    pub recovery_latency_ms: f64,
}

impl RecoveryReceipt {
    pub fn is_clean_recovery(&self) -> bool {
        self.uncommitted_rolled_back == 0
    }
}
