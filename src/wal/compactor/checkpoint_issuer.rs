//! Zero-Copy Checkpoint Issuer & Cryptographic State Snapshot Engine.
//!
//! Emits self-contained binary checkpoint snapshots with pure Rust FIPS 180-4 SHA-256
//! and Castagnoli CRC32-C verification in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::cache::AstContextCache;
use crate::wal::compactor::types::{CheckpointId, CheckpointSnapshot, CompactorError};
use crate::wal::log::frame_encoder::crc32c;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Engine for creating and verifying immutable checkpoint snapshots.
#[derive(Debug, Default, Clone, Copy)]
pub struct CheckpointIssuer;

impl CheckpointIssuer {
    pub fn new() -> Self {
        Self
    }

    /// Freezes and issues a new cryptographically verified `CheckpointSnapshot`.
    pub fn issue_checkpoint(
        sequence_number: u64,
        cst_root_hash: u64,
        state_payload: &[u8],
        target_dir: &Path,
    ) -> Result<CheckpointSnapshot, CompactorError> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let id = CheckpointId::new(sequence_number, timestamp_ms);

        // 1. Calculate SHA-256 digest
        let sha_bytes = AstContextCache::sha256_digest(state_payload);
        let checksum_sha256 = AstContextCache::hex_digest(&sha_bytes);

        // 2. Calculate CRC32-C checksum
        let checksum_crc32c = crc32c(state_payload);

        let snapshot = CheckpointSnapshot {
            id,
            sequence_number,
            cst_root_hash,
            serialized_state: state_payload.to_vec(),
            checksum_sha256,
            checksum_crc32c,
            timestamp_ms,
        };

        // 3. Persist to disk if target directory is specified
        if !target_dir.as_os_str().is_empty() {
            fs::create_dir_all(target_dir)
                .map_err(|e| CompactorError::IoError(format!("Failed to create snapshot directory: {}", e)))?;

            let file_name = format!("checkpoint_{:016x}.snap", sequence_number);
            let file_path = target_dir.join(file_name);

            let encoded = serde_json::to_vec(&snapshot)
                .map_err(|e| CompactorError::IoError(format!("Failed to serialize snapshot: {}", e)))?;

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&file_path)
                .map_err(|e| CompactorError::IoError(format!("Failed to open snapshot file for writing: {}", e)))?;

            file.write_all(&encoded)
                .map_err(|e| CompactorError::IoError(format!("Failed to write snapshot file: {}", e)))?;

            file.sync_all()
                .map_err(|e| CompactorError::IoError(format!("Failed to fsync snapshot file: {}", e)))?;
        }

        Ok(snapshot)
    }

    /// Loads and verifies a checkpoint snapshot from disk, validating both SHA-256 and CRC32-C.
    pub fn load_and_verify_checkpoint(snapshot_path: &Path) -> Result<CheckpointSnapshot, CompactorError> {
        let mut file = File::open(snapshot_path)
            .map_err(|e| CompactorError::IoError(format!("Failed to open snapshot file: {}", e)))?;

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| CompactorError::IoError(format!("Failed to read snapshot file: {}", e)))?;

        let snapshot: CheckpointSnapshot = serde_json::from_slice(&bytes)
            .map_err(|e| CompactorError::CorruptedSegment(format!("Corrupted snapshot JSON: {}", e)))?;

        // Verify SHA-256 checksum
        let computed_sha = AstContextCache::sha256_digest(&snapshot.serialized_state);
        let computed_sha_hex = AstContextCache::hex_digest(&computed_sha);

        if computed_sha_hex != snapshot.checksum_sha256 {
            return Err(CompactorError::ChecksumMismatch {
                expected: snapshot.checksum_sha256,
                actual: computed_sha_hex,
            });
        }

        // Verify CRC32-C checksum
        let computed_crc = crc32c(&snapshot.serialized_state);
        if computed_crc != snapshot.checksum_crc32c {
            return Err(CompactorError::CrcMismatch {
                expected: snapshot.checksum_crc32c,
                actual: computed_crc,
            });
        }

        Ok(snapshot)
    }

    /// Purges older checkpoint files, retaining only the most recent `keep_count` snapshots.
    pub fn purge_old_checkpoints(checkpoint_dir: &Path, keep_count: usize) -> Result<usize, CompactorError> {
        if !checkpoint_dir.exists() {
            return Ok(0);
        }

        let mut entries: Vec<PathBuf> = fs::read_dir(checkpoint_dir)
            .map_err(|e| CompactorError::IoError(e.to_string()))?
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("snap"))
            .collect();

        if entries.len() <= keep_count {
            return Ok(0);
        }

        // Sort ascending by file name (which includes monotonic sequence number)
        entries.sort();

        let to_remove = entries.len() - keep_count;
        let mut purged = 0;

        for path in entries.iter().take(to_remove) {
            if fs::remove_file(path).is_ok() {
                purged += 1;
            }
        }

        Ok(purged)
    }
}
