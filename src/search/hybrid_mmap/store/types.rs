//! Data types and models for the Zero-Heap Mmap Vector Store & Binary Chunk Storage Engine.
//!
//! Enforces fixed-stride binary framing, deterministic magic headers,
//! CRC32-C integrity verification, and zero-heap slice mapping in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers & Dimensions
// ---------------------------------------------------------------------------

/// Deterministic 64-bit vector identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VectorId(pub u64);

impl VectorId {
    /// Generates a deterministic `VectorId` from symbol/chunk name and index.
    pub fn new(name: &str, index: u64) -> Self {
        let combined = format!("{}:{}", name, index);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for VectorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vec_{:016x}", self.0)
    }
}

/// Unique 64-bit code chunk / snippet identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkId(pub u64);

impl ChunkId {
    pub fn new(path: &str, start_byte: usize, end_byte: usize) -> Self {
        let key = format!("{}:{}:{}", path, start_byte, end_byte);
        Self(fnv1a_64(key.as_bytes()))
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chunk_{:016x}", self.0)
    }
}

/// Fixed vector dimensionality (e.g., 384 for MiniLM, 768 for sovereign embeddings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VectorDimension(pub u16);

impl VectorDimension {
    pub const DIM_384: Self = Self(384);
    pub const DIM_768: Self = Self(768);
    pub const DIM_1536: Self = Self(1536);

    pub fn value(&self) -> usize {
        self.0 as usize
    }

    /// Computes the exact fixed-stride binary byte size for a record of this dimension:
    /// `[VectorId (8B) | SymbolId (8B) | ChunkId (8B) | Dim (2B) | Embedding (Dim * 4B) | CRC32C (4B)]`
    /// Header per record (26B) + Float Data (Dim * 4B) + Checksum (4B) = 30 + Dim * 4
    pub fn record_stride_bytes(&self) -> usize {
        26 + (self.0 as usize * 4) + 4
    }
}

impl fmt::Display for VectorDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}D", self.0)
    }
}

// ---------------------------------------------------------------------------
// Binary Headers & Vector Records
// ---------------------------------------------------------------------------

/// 20-byte primary binary header for the mapped vector store index file:
/// `[Magic (4B) | Version (2B) | Dimension (2B) | TotalVectors (8B) | StrideBytes (4B)]`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorHeader {
    /// Magic bytes: ASCII `LOVS` (`[0x4C, 0x4F, 0x56, 0x53]`).
    pub magic: [u8; 4],
    /// Binary format version (starts at 1).
    pub version: u16,
    /// Fixed dimensionality of all vectors stored in this file.
    pub dimension: VectorDimension,
    /// Total count of appended vector records.
    pub total_vectors_count: u64,
    /// Exact binary stride in bytes per record.
    pub stride_bytes: u32,
}

impl VectorHeader {
    pub const MAGIC_BYTES: [u8; 4] = [0x4C, 0x4F, 0x56, 0x53];
    pub const CURRENT_VERSION: u16 = 1;
    pub const HEADER_SIZE: usize = 20;

    pub fn new(dimension: VectorDimension) -> Self {
        Self {
            magic: Self::MAGIC_BYTES,
            version: Self::CURRENT_VERSION,
            dimension,
            total_vectors_count: 0,
            stride_bytes: dimension.record_stride_bytes() as u32,
        }
    }
}

/// Individual vector record with source symbol metadata and integrity checksum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorRecord {
    pub vector_id: VectorId,
    pub symbol_id: u64,
    pub chunk_id: ChunkId,
    pub embedding: Vec<f32>,
    pub checksum: u32,
}

impl VectorRecord {
    pub fn new(
        vector_id: VectorId,
        symbol_id: u64,
        chunk_id: ChunkId,
        embedding: Vec<f32>,
    ) -> Self {
        Self {
            vector_id,
            symbol_id,
            chunk_id,
            embedding,
            checksum: 0,
        }
    }
}

/// Configuration parameters for Mmap Vector Store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MmapStoreConfig {
    pub index_path: PathBuf,
    pub target_dimension: VectorDimension,
    pub max_vectors_capacity: usize,
}

impl Default for MmapStoreConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from(".locus_vectors.idx"),
            target_dimension: VectorDimension::DIM_384,
            max_vectors_capacity: 1_000_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorStoreError {
    #[error("Corrupted vector store header: {0}")]
    CorruptedHeader(String),

    #[error("Invalid vector store magic bytes: {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Unsupported vector store version: {0}")]
    UnsupportedVersion(u16),

    #[error("Dimension mismatch: expected {expected}D, but record has {found}D")]
    DimensionMismatch { expected: u16, found: u16 },

    #[error("Vector index out of bounds: index {index} >= total vectors {total}")]
    IndexOutOfBounds { index: u64, total: u64 },

    #[error("Checksum mismatch on vector {vector_id}: expected 0x{expected:08X}, calculated 0x{calculated:08X}")]
    ChecksumMismatch {
        vector_id: VectorId,
        expected: u32,
        calculated: u32,
    },

    #[error("Torn record detected at offset {offset}: expected {expected_bytes} bytes, found {available_bytes} bytes")]
    TornRecordDetected {
        offset: u64,
        expected_bytes: usize,
        available_bytes: usize,
    },

    #[error("Vector store capacity exceeded: maximum capacity is {0} vectors")]
    CapacityExceeded(usize),

    #[error("I/O error during vector store operation: {0}")]
    IoError(String),

    #[error("Internal vector store error: {0}")]
    InternalError(String),
}
