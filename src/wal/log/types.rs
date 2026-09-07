//! Data types and models for the High-Throughput Binary WAL & CRC32-C Framed Log Engine.
//!
//! Enforces zero-loss binary frame structures, deterministic magic headers,
//! CRC32-C integrity checks, and configurable fsync durability policies.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers & Versioning
// ---------------------------------------------------------------------------

/// Deterministic 64-bit transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TxId(pub u64);

impl TxId {
    /// Generates a deterministic `TxId` from a string identifier and sequence number.
    pub fn new(name: &str, seq: u64) -> Self {
        let combined = format!("{}:{}", name, seq);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tx_{:016x}", self.0)
    }
}

/// WAL protocol version number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WalVersion(pub u16);

impl WalVersion {
    pub const V1: Self = Self(1);

    pub fn raw(&self) -> u16 {
        self.0
    }
}

impl Default for WalVersion {
    fn default() -> Self {
        Self::V1
    }
}

impl fmt::Display for WalVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Frame Types & Fsync Policies
// ---------------------------------------------------------------------------

/// Discrete binary WAL frame types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WalFrameType {
    BeginTx = 0x01,
    SubtreeMutation = 0x02,
    CommitTx = 0x03,
    AbortTx = 0x04,
    CheckpointMarker = 0x05,
}

impl WalFrameType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::BeginTx),
            0x02 => Some(Self::SubtreeMutation),
            0x03 => Some(Self::CommitTx),
            0x04 => Some(Self::AbortTx),
            0x05 => Some(Self::CheckpointMarker),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for WalFrameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::BeginTx => "BEGIN_TX",
            Self::SubtreeMutation => "SUBTREE_MUTATION",
            Self::CommitTx => "COMMIT_TX",
            Self::AbortTx => "ABORT_TX",
            Self::CheckpointMarker => "CHECKPOINT_MARKER",
        };
        write!(f, "{}", s)
    }
}

/// Configurable disk synchronization durability policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsyncPolicy {
    /// Immediate fsync per frame append (maximum ACID durability).
    Immediate,
    /// Batched fsync at a fixed millisecond interval.
    BatchIntervalMs(u32),
    /// Asynchronous memory buffer drain.
    AsyncDrain,
}

impl Default for FsyncPolicy {
    fn default() -> Self {
        Self::Immediate
    }
}

// ---------------------------------------------------------------------------
// Configuration & Binary Headers
// ---------------------------------------------------------------------------

/// Configuration for the Binary WAL subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalConfig {
    pub log_directory: PathBuf,
    pub max_segment_size_bytes: usize,
    pub fsync_policy: FsyncPolicy,
    pub segment_prefix: String,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            log_directory: PathBuf::from(".locus_wal"),
            max_segment_size_bytes: 64 * 1024 * 1024, // 64 MB
            fsync_policy: FsyncPolicy::Immediate,
            segment_prefix: "segment".to_string(),
        }
    }
}

/// 19-byte binary header for each WAL frame:
/// `[Magic (4B) | Version (2B) | TxID (8B) | FrameType (1B) | PayloadLen (4B)]`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalFrameHeader {
    pub magic: [u8; 4],
    pub version: WalVersion,
    pub tx_id: TxId,
    pub frame_type: WalFrameType,
    pub payload_len: u32,
}

/// Complete binary WAL frame with payload and CRC32-C checksum:
/// `[Header (19B) | Payload (NB) | CRC32C (4B)]`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalFrame {
    pub header: WalFrameHeader,
    pub payload: Vec<u8>,
    pub checksum: u32,
}

impl WalFrame {
    /// Total binary byte size of the frame when encoded.
    pub fn total_encoded_size(&self) -> usize {
        19 + self.payload.len() + 4
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalError {
    #[error("Corrupted WAL frame: {0}")]
    CorruptedFrame(String),

    #[error("Checksum mismatch: expected 0x{expected:08X}, found 0x{found:08X}")]
    ChecksumMismatch { expected: u32, found: u32 },

    #[error("Torn write detected: expected {expected_bytes} bytes, but only {available_bytes} bytes available")]
    TornWriteDetected {
        expected_bytes: usize,
        available_bytes: usize,
    },

    #[error("Invalid WAL magic bytes: {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Unsupported WAL version: {0}")]
    UnsupportedVersion(u16),

    #[error("Unknown frame type byte: 0x{0:02X}")]
    UnknownFrameType(u8),

    #[error("WAL segment full: current size exceeds {0} bytes")]
    SegmentFull(usize),

    #[error("I/O error during WAL operation: {0}")]
    IoError(String),

    #[error("Internal WAL error: {0}")]
    InternalError(String),
}
