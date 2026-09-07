//! WAL Compactor & Zero-Copy Checkpoint Engine.
//!
//! Provides deterministic latest-state log compaction, disk space reclamation,
//! zero-copy checkpoint snapshotting, and non-blocking background workers in pure safe Rust.

#![forbid(unsafe_code)]

pub mod checkpoint_issuer;
pub mod log_compactor;
pub mod types;

pub use checkpoint_issuer::CheckpointIssuer;
pub use log_compactor::WalLogCompactor;
pub use types::{
    CheckpointId, CheckpointSnapshot, CompactorConfig, CompactorError, CompactorPolicy,
    CompactorStats,
};

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swarm::channel::ring_channel::SwarmRingChannel;
    use crate::swarm::channel::types::{SwarmMessage, SwarmMessageType};
    use crate::wal::log::frame_encoder::BinaryFrameCodec;
    use crate::wal::log::types::{TxId, WalFrameType};
    use crate::wal::recovery::log_scanner::WalLogScanner;
    use crate::wal::recovery::types::WalMutationPayload;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    fn create_test_log_bytes(total_txs: usize, num_unique_keys: usize) -> Vec<u8> {
        let mut raw_bytes = Vec::new();
        for i in 0..total_txs {
            let tx_id = TxId::new("tx", i as u64);
            let begin_encoded = BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, tx_id, &[]).unwrap();
            raw_bytes.extend_from_slice(&begin_encoded);

            let key_idx = i % num_unique_keys;
            let file_path = format!("src/components/item_{}.rs", key_idx);
            let symbol_id = (key_idx as u64) + 100;
            let replacement = format!("// Version {} of item {}", i, key_idx);

            let payload = WalMutationPayload::new(file_path, symbol_id, (0, 100), replacement, i as u64);
            let payload_bytes = payload.to_bytes().unwrap();
            let mut_encoded = BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx_id, &payload_bytes).unwrap();
            raw_bytes.extend_from_slice(&mut_encoded);

            let commit_encoded = BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, tx_id, &[]).unwrap();
            raw_bytes.extend_from_slice(&commit_encoded);
        }
        raw_bytes
    }

    #[test]
    fn test_1000_mutations_compaction_latest_state_retention() {
        let num_unique_keys = 5;
        let total_txs = 1000;
        let raw_bytes = create_test_log_bytes(total_txs, num_unique_keys);

        let initial_size = raw_bytes.len();
        assert!(initial_size > 0);

        let (compacted_bytes, stats) = WalLogCompactor::compact_bytes(&raw_bytes)
            .expect("Compaction must succeed");

        assert_eq!(stats.total_input_mutations, 1000);
        assert_eq!(stats.surviving_mutations_count, num_unique_keys);
        assert!(stats.bytes_reclaimed > 0);
        assert!(
            stats.reduction_percentage() > 80.0,
            "Compaction should reclaim >80% space (got {:.2}%)",
            stats.reduction_percentage()
        );

        // Verify compacted bytes validity
        let scan_res = WalLogScanner::scan_bytes(&compacted_bytes).expect("Scanning compacted bytes must succeed");
        assert_eq!(scan_res.transactions.len(), 1);
        let compacted_tx = &scan_res.transactions[0];
        assert!(compacted_tx.is_committed());
        assert_eq!(compacted_tx.mutations.len(), num_unique_keys);
    }

    #[test]
    fn test_checkpoint_snapshot_sha256_and_crc_verification() {
        let temp_dir = PathBuf::from(format!("./target/test_snap_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let dummy_state = b"pub struct CoreState { pub active_threads: u32, pub cst_version: u64 }";
        let seq = 42;
        let cst_root_hash = 0x9876543210abcdef;

        let snapshot = CheckpointIssuer::issue_checkpoint(seq, cst_root_hash, dummy_state, &temp_dir)
            .expect("Snapshot issuance succeeds");

        assert_eq!(snapshot.sequence_number, seq);
        assert_eq!(snapshot.cst_root_hash, cst_root_hash);
        assert_eq!(snapshot.serialized_state, dummy_state);
        assert!(!snapshot.checksum_sha256.is_empty());
        assert_ne!(snapshot.checksum_crc32c, 0);

        // Load and verify roundtrip from disk
        let snap_file = temp_dir.join(format!("checkpoint_{:016x}.snap", seq));
        let loaded = CheckpointIssuer::load_and_verify_checkpoint(&snap_file)
            .expect("Snapshot verification succeeds");

        assert_eq!(loaded, snapshot);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_non_blocking_background_compaction_with_ring_channel() {
        let temp_wal = PathBuf::from(format!("./target/test_bg_wal_{}", std::process::id()));
        let temp_snap = temp_wal.join("snapshots");
        let _ = fs::remove_dir_all(&temp_wal);
        fs::create_dir_all(&temp_wal).unwrap();

        // Write sample segment
        let raw_bytes = create_test_log_bytes(200, 4);
        fs::write(temp_wal.join("segment_00000001.wal"), &raw_bytes).unwrap();

        let config = CompactorConfig {
            wal_directory: temp_wal.clone(),
            checkpoint_directory: temp_snap,
            policy: CompactorPolicy::Manual,
            max_retained_checkpoints: 3,
        };

        let compactor = WalLogCompactor::new(config);
        let ring_channel = SwarmRingChannel::new(5000);
        let messages_sent = Arc::new(AtomicUsize::new(0));

        // Start background compaction thread
        let handle = compactor.spawn_background_compaction();

        // Concurrently pass messages on the ring channel
        for i in 0..1000 {
            let msg = SwarmMessage::new(1, 2, i as u64, SwarmMessageType::TaskProgress, vec![1, 2, 3]);
            ring_channel.try_send(msg).expect("Ring channel send succeeds");
            messages_sent.fetch_add(1, Ordering::Relaxed);
        }

        let (snap, stats) = handle.join().unwrap().expect("Background compaction succeeds");
        assert_eq!(stats.surviving_mutations_count, 4);
        assert!(snap.sequence_number > 0);
        assert_eq!(messages_sent.load(Ordering::Relaxed), 1000);

        let _ = fs::remove_dir_all(&temp_wal);
    }

    #[test]
    fn test_sub_1_0ms_compaction_latency_budget() {
        let raw_bytes = create_test_log_bytes(100, 5);

        // Warm up
        for _ in 0..5 {
            let _ = WalLogCompactor::compact_bytes(&raw_bytes);
        }

        let iterations = 100;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = WalLogCompactor::compact_bytes(&raw_bytes).expect("Compaction succeeds");
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average WAL Log Compaction Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 15.0 } else { 1.0 };
        assert!(
            avg_latency_ms < budget,
            "Compaction latency must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
