//! High-Throughput Binary WAL Scanner & Transaction Grouping Engine.
//!
//! Scans contiguous byte buffers and segment files, verifies CRC32-C checksums,
//! isolates committed vs uncommitted transactions, and handles torn-writes at crash boundaries.

#![forbid(unsafe_code)]

use super::types::{WalCheckpointPayload, WalMutationPayload};
use crate::wal::log::frame_encoder::BinaryFrameCodec;
use crate::wal::log::types::{TxId, WalError, WalFrameType};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// In-memory representation of a transaction reconstructed during scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedTx {
    pub tx_id: TxId,
    pub mutations: Vec<WalMutationPayload>,
    pub has_begin: bool,
    pub has_commit: bool,
    pub has_abort: bool,
    pub first_seen_seq: usize,
}

impl ScannedTx {
    pub fn is_committed(&self) -> bool {
        self.has_begin && self.has_commit && !self.has_abort
    }
}

/// Aggregated outcome of scanning one or more WAL log segments.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub transactions: Vec<ScannedTx>,
    pub checkpoints: Vec<WalCheckpointPayload>,
    pub total_valid_frames: usize,
    pub torn_writes_count: usize,
    pub scanned_bytes: usize,
    pub segments_count: usize,
}

/// Binary scanner for WAL segments.
pub struct WalLogScanner;

impl WalLogScanner {
    /// Scans a contiguous byte slice (e.g. mmap or in-memory segment).
    pub fn scan_bytes(raw_bytes: &[u8]) -> Result<ScanResult, WalError> {
        let mut result = ScanResult::default();
        let mut tx_map: HashMap<TxId, ScannedTx> = HashMap::new();
        let mut offset = 0;
        let total_len = raw_bytes.len();
        let mut frame_seq = 0;

        while offset < total_len {
            let slice = &raw_bytes[offset..];
            if slice.is_empty() {
                break;
            }

            match BinaryFrameCodec::decode_frame(slice) {
                Ok((frame, consumed)) => {
                    offset += consumed;
                    result.total_valid_frames += 1;
                    frame_seq += 1;

                    let tx_entry = tx_map.entry(frame.header.tx_id).or_insert_with(|| ScannedTx {
                        tx_id: frame.header.tx_id,
                        mutations: Vec::new(),
                        has_begin: false,
                        has_commit: false,
                        has_abort: false,
                        first_seen_seq: frame_seq,
                    });

                    match frame.header.frame_type {
                        WalFrameType::BeginTx => {
                            tx_entry.has_begin = true;
                        }
                        WalFrameType::SubtreeMutation => {
                            if let Ok(payload) = WalMutationPayload::from_bytes(&frame.payload) {
                                tx_entry.mutations.push(payload);
                            }
                        }
                        WalFrameType::CommitTx => {
                            tx_entry.has_commit = true;
                        }
                        WalFrameType::AbortTx => {
                            tx_entry.has_abort = true;
                        }
                        WalFrameType::CheckpointMarker => {
                            if let Ok(chk) = WalCheckpointPayload::from_bytes(&frame.payload) {
                                result.checkpoints.push(chk);
                            }
                        }
                    }
                }
                Err(WalError::TornWriteDetected { .. }) => {
                    // Reached incomplete frame at end of segment (clean crash boundary)
                    result.torn_writes_count += 1;
                    break;
                }
                Err(e) => {
                    // If magic or checksum fails, record error or terminate scan
                    return Err(e);
                }
            }
        }

        result.scanned_bytes = offset;
        let mut txs: Vec<ScannedTx> = tx_map.into_values().collect();
        txs.sort_by_key(|t| t.first_seen_seq);
        result.transactions = txs;

        Ok(result)
    }

    /// Scans all log segment files in a directory matching a prefix.
    pub fn scan_directory(log_dir: &Path, prefix: &str) -> Result<ScanResult, WalError> {
        let mut aggregated = ScanResult::default();
        if !log_dir.exists() {
            return Ok(aggregated);
        }

        let mut entries = fs::read_dir(log_dir)
            .map_err(|e| WalError::IoError(format!("Cannot read log dir: {}", e)))?
            .filter_map(|res| res.ok())
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.starts_with(prefix) && name.ends_with(".wal")
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|e| e.file_name());
        aggregated.segments_count = entries.len();

        for entry in entries {
            let bytes = fs::read(entry.path())
                .map_err(|e| WalError::IoError(format!("Failed reading WAL segment: {}", e)))?;
            let segment_res = Self::scan_bytes(&bytes)?;

            aggregated.total_valid_frames += segment_res.total_valid_frames;
            aggregated.torn_writes_count += segment_res.torn_writes_count;
            aggregated.scanned_bytes += segment_res.scanned_bytes;
            aggregated.checkpoints.extend(segment_res.checkpoints);

            // Merge transactions
            for tx in segment_res.transactions {
                if let Some(existing) = aggregated
                    .transactions
                    .iter_mut()
                    .find(|t| t.tx_id == tx.tx_id)
                {
                    existing.has_begin |= tx.has_begin;
                    existing.has_commit |= tx.has_commit;
                    existing.has_abort |= tx.has_abort;
                    existing.mutations.extend(tx.mutations);
                } else {
                    aggregated.transactions.push(tx);
                }
            }
        }

        Ok(aggregated)
    }
}
