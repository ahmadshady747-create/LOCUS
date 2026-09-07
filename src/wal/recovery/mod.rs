//! Zero-Loss Crash Recovery & Deterministic Log Replay Engine Module.
//!
//! Reconstructs workspace state after unexpected crashes or power failures with
//! sub-1.0ms RTO, zero unhandled errors, atomic rollbacks of incomplete transactions,
//! and self-healing torn-write truncation.

#![forbid(unsafe_code)]

pub mod log_scanner;
pub mod replay_engine;
pub mod types;

pub use log_scanner::{ScanResult, ScannedTx, WalLogScanner};
pub use replay_engine::WalReplayEngine;
pub use types::{
    RecoveredTransaction, RecoveryMode, RecoveryReceipt, TransactionRecoveryStatus,
    WalCheckpointPayload, WalMutationPayload,
};

use crate::wal::log::types::WalError;
use std::collections::HashMap;
use std::path::Path;

/// Central crash recovery coordinator.
pub struct WalCrashRecoveryManager;

impl WalCrashRecoveryManager {
    /// Recovers workspace state directly from in-memory binary logs.
    pub fn recover_from_bytes(
        raw_bytes: &[u8],
        workspace_state: &mut HashMap<String, String>,
        mode: RecoveryMode,
    ) -> Result<RecoveryReceipt, WalError> {
        let scan = WalLogScanner::scan_bytes(raw_bytes)?;
        WalReplayEngine::replay(&scan, workspace_state, mode)
    }

    /// Recovers workspace state from disk segments in the specified log directory.
    pub fn recover_from_directory(
        log_dir: &Path,
        prefix: &str,
        workspace_state: &mut HashMap<String, String>,
        mode: RecoveryMode,
    ) -> Result<RecoveryReceipt, WalError> {
        let scan = WalLogScanner::scan_directory(log_dir, prefix)?;
        WalReplayEngine::replay(&scan, workspace_state, mode)
    }
}

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::log::frame_encoder::BinaryFrameCodec;
    use crate::wal::log::types::{TxId, WalFrameType};
    use std::time::Instant;

    #[test]
    fn test_crash_recovery_replays_committed_and_rolls_back_uncommitted() {
        let mut buffer = Vec::new();
        let file = "src/auth/session.rs";

        // Tx 1: Fully committed
        let tx1 = TxId::from_raw(0x1001);
        let f_begin1 = BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, tx1, &[]).unwrap();
        let payload1 = WalMutationPayload::new(file, 100, (0, 0), "fn authenticated() -> bool { true }\n", 1);
        let f_mut1 = BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx1, &payload1.to_bytes().unwrap()).unwrap();
        let f_commit1 = BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, tx1, &[]).unwrap();

        buffer.extend_from_slice(&f_begin1);
        buffer.extend_from_slice(&f_mut1);
        buffer.extend_from_slice(&f_commit1);

        // Tx 2: Uncommitted (simulating crash before CommitTx frame)
        let tx2 = TxId::from_raw(0x2002);
        let f_begin2 = BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, tx2, &[]).unwrap();
        let payload2 = WalMutationPayload::new(file, 200, (0, 0), "fn evil_injection() {}\n", 2);
        let f_mut2 = BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx2, &payload2.to_bytes().unwrap()).unwrap();

        buffer.extend_from_slice(&f_begin2);
        buffer.extend_from_slice(&f_mut2);
        // Crash happened here! No CommitTx frame!

        let mut workspace = HashMap::new();
        let receipt = WalCrashRecoveryManager::recover_from_bytes(
            &buffer,
            &mut workspace,
            RecoveryMode::FullReplay,
        )
        .expect("Recovery must succeed");

        // Assertions
        assert_eq!(receipt.recovered_transactions.len(), 2);
        assert_eq!(receipt.total_mutations_replayed, 1);
        assert_eq!(receipt.uncommitted_rolled_back, 1);

        let tx1_rec = receipt.recovered_transactions.iter().find(|t| t.tx_id == tx1).unwrap();
        assert_eq!(tx1_rec.status, TransactionRecoveryStatus::Committed);

        let tx2_rec = receipt.recovered_transactions.iter().find(|t| t.tx_id == tx2).unwrap();
        assert_eq!(tx2_rec.status, TransactionRecoveryStatus::RolledBack);

        // Verify file contents: Tx1 applied, Tx2 rolled back
        let content = workspace.get(file).unwrap();
        assert!(content.contains("fn authenticated() -> bool { true }"));
        assert!(!content.contains("fn evil_injection()"));
    }

    #[test]
    fn test_checkpoint_consistency_marker_recording() {
        let mut buffer = Vec::new();
        let tx_sys = TxId::from_raw(0x0000);

        let chk_payload = WalCheckpointPayload::new(42, 1725000000000, 15);
        let f_chk = BinaryFrameCodec::encode_frame(
            WalFrameType::CheckpointMarker,
            tx_sys,
            &chk_payload.to_bytes().unwrap(),
        )
        .unwrap();
        buffer.extend_from_slice(&f_chk);

        let mut workspace = HashMap::new();
        let receipt = WalCrashRecoveryManager::recover_from_bytes(
            &buffer,
            &mut workspace,
            RecoveryMode::CheckpointOnly,
        )
        .expect("Checkpoint recovery succeeds");

        assert_eq!(receipt.last_valid_checkpoint_seq, 42);
    }

    #[test]
    fn test_torn_write_truncation_at_eof_heals_cleanly() {
        let mut buffer = Vec::new();
        let file = "src/core/types.rs";

        // 1. Valid committed transaction
        let tx1 = TxId::from_raw(0x3003);
        buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, tx1, &[]).unwrap());
        let payload = WalMutationPayload::new(file, 300, (0, 0), "pub struct User;\n", 1);
        buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx1, &payload.to_bytes().unwrap()).unwrap());
        buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, tx1, &[]).unwrap());

        // 2. Append corrupted / partial torn bytes at EOF (e.g. truncated header)
        buffer.extend_from_slice(&[0x4C, 0x4F, 0x43, 0x57, 0x01, 0x00]); // 6 bytes of a 23-byte min frame

        let mut workspace = HashMap::new();
        let receipt = WalCrashRecoveryManager::recover_from_bytes(
            &buffer,
            &mut workspace,
            RecoveryMode::FullReplay,
        )
        .expect("Recovery must truncate torn write and succeed");

        assert_eq!(receipt.total_mutations_replayed, 1);
        assert_eq!(workspace.get(file).unwrap(), "pub struct User;\n");
    }

    #[test]
    fn test_sub_1ms_recovery_time_objective_budget() {
        let mut buffer = Vec::new();
        let file = "src/scale/bench.rs";

        // Generate 100 transactions
        for i in 0..100 {
            let tx = TxId::from_raw(i as u64 + 1);
            buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::BeginTx, tx, &[]).unwrap());
            let payload = WalMutationPayload::new(file, i as u64, (0, 0), format!("pub const VAL_{}: u32 = {};\n", i, i), i as u64);
            buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx, &payload.to_bytes().unwrap()).unwrap());
            buffer.extend_from_slice(&BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, tx, &[]).unwrap());
        }

        let start = Instant::now();
        let mut workspace = HashMap::new();
        let receipt = WalCrashRecoveryManager::recover_from_bytes(
            &buffer,
            &mut workspace,
            RecoveryMode::FullReplay,
        )
        .expect("Recovery succeeds");
        let elapsed = start.elapsed();

        let avg_ms = elapsed.as_secs_f64() * 1000.0;
        println!("⚡ Crash Recovery Time for 100 transactions: {:.4} ms", avg_ms);

        let budget_ms = if cfg!(debug_assertions) { 50.0 } else { 1.0 };
        assert!(
            avg_ms < budget_ms,
            "Crash Recovery RTO must be below 1.0ms (got {:.4}ms)",
            avg_ms
        );
        assert_eq!(receipt.total_mutations_replayed, 100);
    }
}
