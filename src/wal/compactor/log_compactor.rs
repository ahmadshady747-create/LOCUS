//! Deterministic WAL Log Compactor & Latest-State Retention Engine.
//!
//! Merges successive AST symbol and file mutations across WAL segments,
//! discards superseded historical states, purges old segments, and reclaims disk volume.

#![forbid(unsafe_code)]

use crate::wal::compactor::checkpoint_issuer::CheckpointIssuer;
use crate::wal::compactor::types::{
    CheckpointSnapshot, CompactorConfig, CompactorError, CompactorStats,
};
use crate::wal::log::frame_encoder::BinaryFrameCodec;
use crate::wal::log::types::{TxId, WalFrameType};
use crate::wal::recovery::log_scanner::WalLogScanner;
use crate::wal::recovery::types::WalMutationPayload;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// Central compactor responsible for deduplicating committed transactions and shrinking WAL volume.
#[derive(Debug, Clone)]
pub struct WalLogCompactor {
    config: CompactorConfig,
    is_compacting: Arc<AtomicBool>,
}

impl WalLogCompactor {
    pub fn new(config: CompactorConfig) -> Self {
        Self {
            config,
            is_compacting: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Compacts raw in-memory segment bytes into a deduplicated, latest-state byte buffer.
    pub fn compact_bytes(raw_bytes: &[u8]) -> Result<(Vec<u8>, CompactorStats), CompactorError> {
        let start = Instant::now();

        // 1. Scan and extract committed transactions
        let scan_res = WalLogScanner::scan_bytes(raw_bytes)
            .map_err(|e| CompactorError::CorruptedSegment(e.to_string()))?;

        let mut total_input_mutations = 0;
        let mut latest_mutations: HashMap<String, WalMutationPayload> = HashMap::new();
        let mut latest_seq = 0u64;

        for (tx_idx, tx) in scan_res.transactions.iter().enumerate() {
            if tx.is_committed() {
                for mutation in &tx.mutations {
                    total_input_mutations += 1;
                    // Key by file_path and symbol_id for fine-grained deduplication
                    let key = format!("{}:{}", mutation.file_path, mutation.symbol_id);
                    latest_mutations.insert(key, mutation.clone());
                    latest_seq = latest_seq.max((tx_idx + 1) as u64);
                }
            }
        }

        if total_input_mutations == 0 {
            return Err(CompactorError::EmptyLog);
        }

        // 2. Encode surviving mutations into a fresh compacted frame buffer
        let mut compacted_bytes = Vec::with_capacity(latest_mutations.len() * 256);
        let compacted_tx_id = TxId::new("compacted_state", latest_seq);

        // BeginTx frame
        let begin_encoded = BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, compacted_tx_id, &[])
            .map_err(|e| CompactorError::CorruptedSegment(e.to_string()))?;
        compacted_bytes.extend_from_slice(&begin_encoded);

        // SubtreeMutation frames for surviving mutations
        let mut sorted_mutations: Vec<_> = latest_mutations.values().collect();
        sorted_mutations.sort_by(|a, b| a.file_path.cmp(&b.file_path).then(a.symbol_id.cmp(&b.symbol_id)));

        for mutation in &sorted_mutations {
            let payload_bytes = mutation
                .to_bytes()
                .map_err(|e| CompactorError::CorruptedSegment(e.to_string()))?;
            let mut_encoded = BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, compacted_tx_id, &payload_bytes)
                .map_err(|e| CompactorError::CorruptedSegment(e.to_string()))?;
            compacted_bytes.extend_from_slice(&mut_encoded);
        }

        // CommitTx frame
        let commit_encoded = BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, compacted_tx_id, &[])
            .map_err(|e| CompactorError::CorruptedSegment(e.to_string()))?;
        compacted_bytes.extend_from_slice(&commit_encoded);

        let surviving_mutations_count = latest_mutations.len();
        let bytes_reclaimed = (raw_bytes.len() as u64).saturating_sub(compacted_bytes.len() as u64);
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let stats = CompactorStats {
            compacted_segments_count: 1,
            bytes_reclaimed,
            surviving_mutations_count,
            total_input_mutations,
            compaction_duration_ms: duration_ms,
        };

        Ok((compacted_bytes, stats))
    }

    /// Compacts all log segments in `wal_directory` and generates a persistent `CheckpointSnapshot`.
    pub fn compact_directory(&self) -> Result<(CheckpointSnapshot, CompactorStats), CompactorError> {
        if self
            .is_compacting
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(CompactorError::CompactionInProgress);
        }

        let result = self.execute_directory_compaction();
        self.is_compacting.store(false, Ordering::SeqCst);
        result
    }

    /// Internal execution of directory compaction.
    fn execute_directory_compaction(&self) -> Result<(CheckpointSnapshot, CompactorStats), CompactorError> {
        let start = Instant::now();

        if !self.config.wal_directory.exists() {
            return Err(CompactorError::EmptyLog);
        }

        // 1. Collect all segment files
        let mut segment_files: Vec<PathBuf> = fs::read_dir(&self.config.wal_directory)
            .map_err(|e| CompactorError::IoError(e.to_string()))?
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name.starts_with("segment_") && name.ends_with(".wal")
            })
            .collect();

        if segment_files.is_empty() {
            return Err(CompactorError::EmptyLog);
        }

        segment_files.sort();

        // 2. Read and aggregate all segment bytes
        let mut total_uncompacted_bytes = 0u64;
        let mut aggregated_bytes = Vec::new();

        for seg_path in &segment_files {
            let mut file = File::open(seg_path)
                .map_err(|e| CompactorError::IoError(format!("Failed to read segment {}: {}", seg_path.display(), e)))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| CompactorError::IoError(e.to_string()))?;
            total_uncompacted_bytes += buf.len() as u64;
            aggregated_bytes.extend_from_slice(&buf);
        }

        // 3. Compact aggregated bytes
        let (compacted_bytes, mut stats) = Self::compact_bytes(&aggregated_bytes)?;
        stats.compacted_segments_count = segment_files.len();

        // 4. Issue checkpoint snapshot
        let seq = stats.surviving_mutations_count as u64;
        let cst_root_hash = crate::types::fnv1a_64(&compacted_bytes);

        let snapshot = CheckpointIssuer::issue_checkpoint(
            seq,
            cst_root_hash,
            &compacted_bytes,
            &self.config.checkpoint_directory,
        )?;

        // 5. Write single compacted replacement segment
        let compacted_segment_path = self.config.wal_directory.join("segment_compacted_00000001.wal");
        let mut comp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&compacted_segment_path)
            .map_err(|e| CompactorError::IoError(e.to_string()))?;

        comp_file
            .write_all(&compacted_bytes)
            .map_err(|e| CompactorError::IoError(e.to_string()))?;
        comp_file
            .sync_all()
            .map_err(|e| CompactorError::IoError(e.to_string()))?;

        // 6. Purge obsolete original segment files
        for seg_path in &segment_files {
            if seg_path != &compacted_segment_path {
                let _ = fs::remove_file(seg_path);
            }
        }

        // Purge old checkpoints exceeding retention limit
        let _ = CheckpointIssuer::purge_old_checkpoints(
            &self.config.checkpoint_directory,
            self.config.max_retained_checkpoints,
        );

        stats.bytes_reclaimed = total_uncompacted_bytes.saturating_sub(compacted_bytes.len() as u64);
        stats.compaction_duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok((snapshot, stats))
    }

    /// Spawns background worker thread to execute compaction without blocking swarm IPC channels.
    pub fn spawn_background_compaction(&self) -> JoinHandle<Result<(CheckpointSnapshot, CompactorStats), CompactorError>> {
        let compactor = self.clone();
        thread::spawn(move || compactor.compact_directory())
    }
}
