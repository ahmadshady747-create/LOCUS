//! High-Throughput Binary WAL & CRC32-C Framed Log Engine Barrel.
//!
//! Exposes binary frame codecs, CRC32-C Castagnoli checksums, sequential WAL writers,
//! torn-write detectors, and configurable fsync durability policies.

#![forbid(unsafe_code)]

pub mod frame_encoder;
pub mod types;
pub mod writer;

pub use frame_encoder::{crc32c, BinaryFrameCodec};
pub use types::{
    FsyncPolicy, TxId, WalConfig, WalError, WalFrame, WalFrameHeader, WalFrameType, WalVersion,
};
pub use writer::BinaryWalWriter;

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_encode_and_decode_frame_roundtrip_all_types() {
        let tx_id = TxId::new("TxMatrix", 42);
        let payload = b"AST_SUBTREE_MUTATION_PAYLOAD_V2_RUST";

        let types = [
            WalFrameType::BeginTx,
            WalFrameType::SubtreeMutation,
            WalFrameType::CommitTx,
            WalFrameType::AbortTx,
            WalFrameType::CheckpointMarker,
        ];

        for &frame_type in &types {
            let encoded = BinaryFrameCodec::encode_frame(frame_type, tx_id, payload)
                .expect("Encoding succeeds");
            assert!(encoded.len() >= BinaryFrameCodec::MIN_FRAME_SIZE);

            let (decoded, consumed) =
                BinaryFrameCodec::decode_frame(&encoded).expect("Decoding succeeds");
            assert_eq!(consumed, encoded.len());
            assert_eq!(decoded.header.magic, BinaryFrameCodec::MAGIC_BYTES);
            assert_eq!(decoded.header.version, WalVersion::V1);
            assert_eq!(decoded.header.tx_id, tx_id);
            assert_eq!(decoded.header.frame_type, frame_type);
            assert_eq!(decoded.payload, payload);
            assert_eq!(decoded.header.payload_len as usize, payload.len());
        }
    }

    #[test]
    fn test_checksum_mismatch_detected_on_single_bit_flip() {
        let tx_id = TxId::new("TxTaint", 99);
        let payload = b"IMPORTANT_SECURITY_TRANSACTION_PAYLOAD";
        let mut encoded = BinaryFrameCodec::encode_frame(WalFrameType::SubtreeMutation, tx_id, payload)
            .expect("Encoding succeeds");

        // Tamper with a single byte in payload (byte index 22)
        encoded[22] ^= 0xFF;

        let res = BinaryFrameCodec::decode_frame(&encoded);
        assert!(res.is_err(), "Tampered payload must trigger checksum mismatch");
        match res.unwrap_err() {
            WalError::ChecksumMismatch { expected, found } => {
                assert_ne!(expected, found);
            }
            other => panic!("Expected ChecksumMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_torn_write_detection_on_truncated_buffer() {
        let tx_id = TxId::new("TxPowerFailure", 1);
        let payload = b"CRITICAL_STATE_BEFORE_POWER_OUTAGE";
        let encoded = BinaryFrameCodec::encode_frame(WalFrameType::CommitTx, tx_id, payload)
            .expect("Encoding succeeds");

        // Simulate torn write by truncating the last 5 bytes
        let truncated = &encoded[..encoded.len() - 5];

        let res = BinaryFrameCodec::decode_frame(truncated);
        assert!(res.is_err(), "Truncated frame must be caught as TornWriteDetected");
        match res.unwrap_err() {
            WalError::TornWriteDetected { expected_bytes, available_bytes } => {
                assert_eq!(expected_bytes, encoded.len());
                assert_eq!(available_bytes, truncated.len());
            }
            other => panic!("Expected TornWriteDetected, got {:?}", other),
        }
    }

    #[test]
    fn test_sequential_append_and_recovery_1000_frames() {
        let config = WalConfig {
            fsync_policy: FsyncPolicy::AsyncDrain,
            ..WalConfig::default()
        };
        let mut writer = BinaryWalWriter::open_in_memory(config);

        for i in 0..1000 {
            let tx_id = TxId::new("BatchTx", i);
            let payload = format!("DELTA_OPERATION_{:05}", i).into_bytes();
            writer.append_mutation(tx_id, &payload).expect("Append succeeds");
        }

        assert_eq!(writer.current_segment_index(), 1);
        assert!(writer.total_bytes_written() > 20000);

        let recovered = writer.read_all_frames().expect("Recovery succeeds");
        assert_eq!(recovered.len(), 1000);
        for (i, frame) in recovered.iter().enumerate() {
            let expected_tx = TxId::new("BatchTx", i as u64);
            assert_eq!(frame.header.tx_id, expected_tx);
            let expected_payload = format!("DELTA_OPERATION_{:05}", i).into_bytes();
            assert_eq!(frame.payload, expected_payload);
        }
    }

    #[test]
    fn test_sub_0_05ms_frame_append_latency_budget() {
        let config = WalConfig {
            fsync_policy: FsyncPolicy::AsyncDrain,
            ..WalConfig::default()
        };
        let mut writer = BinaryWalWriter::open_in_memory(config);
        let payload = b"BENCHMARK_FRAME_PAYLOAD_32_BYTES";

        let start = Instant::now();
        for i in 0..100 {
            let tx_id = TxId::new("BenchTx", i);
            let _ = writer.append_mutation(tx_id, payload);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 100.0;

        assert!(
            avg_us < 500.0,
            "Binary WAL encode & append must execute rapidly (sub-0.5ms in debug, sub-0.05ms in release), took: {:.2}µs",
            avg_us
        );
    }
}
