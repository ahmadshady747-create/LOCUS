//! Air-Gapped Animated QR Sync Engine for LOCUS.
//!
//! Enables zero-network, sovereign data and configuration synchronization between isolated/air-gapped
//! devices via high-speed optical animated QR streams, with chunk-level CRC32 and payload SHA-256 validation.

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

// === Lightweight Standard Cryptographic & Checksum Algorithms ===

/// Standard CRC-32 (IEEE 802.3) implementation for fast per-chunk frame integrity.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Standard SHA-256 implementation in pure Rust for payload verification without external bloat.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..(i + 1) * 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    h.iter().map(|val| format!("{:08x}", val)).collect()
}

// === Data Structures ===

/// Full sovereign synchronization payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncPayload {
    pub version: String,
    pub created_at: String,
    pub checksum_sha256: String,
    pub config_json: String,
    pub slots_config: Option<String>,
    pub active_addons: Vec<String>,
    pub custom_data: Option<String>,
}

impl Default for SyncPayload {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            checksum_sha256: String::new(),
            config_json: "{}".to_string(),
            slots_config: None,
            active_addons: Vec::new(),
            custom_data: None,
        }
    }
}

/// A parsed optical frame chunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncChunk {
    pub session_id: String,
    pub index: usize,
    pub total_chunks: usize,
    pub payload_base64: String,
    pub chunk_crc: u32,
}

/// Live progress status returned to UI during optical reception.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirGapIngestProgress {
    pub session_id: String,
    pub received_chunks: usize,
    pub total_chunks: usize,
    pub percent_complete: f32,
    pub is_ready: bool,
    pub error: Option<String>,
}

#[derive(Debug, Error)]
pub enum AirGapError {
    #[error("Invalid frame format: {0}")]
    InvalidFrameFormat(String),

    #[error("CRC32 mismatch on chunk {0}: expected {1:x}, computed {2:x}")]
    CrcMismatch(usize, u32, u32),

    #[error("SHA-256 payload checksum mismatch: expected {0}, computed {1}")]
    ShaMismatch(String, String),

    #[error("Incomplete payload: {0}/{1} chunks received")]
    Incomplete(usize, usize),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("I/O error: {0}")]
    IoError(String),
}

// === Optical Exporter (Sender) ===

pub struct AirGapExporter;

impl AirGapExporter {
    /// Serializes and divides a `SyncPayload` into an ordered array of high-density QR frame strings.
    /// Default chunk size: ~180 characters per frame for rapid optical barcode scanning.
    pub fn generate_frames(
        mut payload: SyncPayload,
        session_id: &str,
        chunk_size: usize,
    ) -> Result<Vec<String>, AirGapError> {
        payload.created_at = Utc::now().to_rfc3339();
        payload.checksum_sha256 = String::new(); // Clear prior to hashing

        // Serialize payload body to compute deterministic SHA-256
        let raw_json = serde_json::to_string(&payload)
            .map_err(|e| AirGapError::SerializationError(e.to_string()))?;
        let computed_sha = sha256_hex(raw_json.as_bytes());
        payload.checksum_sha256 = computed_sha;

        // Final payload JSON string
        let final_json = serde_json::to_string(&payload)
            .map_err(|e| AirGapError::SerializationError(e.to_string()))?;

        // Base64-encode final payload
        let full_b64 = Self::base64_encode(final_json.as_bytes());
        let effective_chunk_size = if chunk_size < 50 { 180 } else { chunk_size };

        let chunks: Vec<&str> = full_b64
            .as_bytes()
            .chunks(effective_chunk_size)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        let total = chunks.len();
        let mut frames = Vec::with_capacity(total);

        for (i, chunk_str) in chunks.iter().enumerate() {
            let crc = crc32(chunk_str.as_bytes());
            // Protocol frame format: LOCUS:v1:<session_id>:<idx>/<total>:<crc_hex>:<payload_b64>
            let frame = format!(
                "LOCUS:v1:{}:{}/{}:{:08x}:{}",
                session_id,
                i + 1,
                total,
                crc,
                chunk_str
            );
            frames.push(frame);
        }

        info!(
            "Generated {} air-gap sync frame(s) for session '{}' (Payload SHA: {})",
            total, session_id, payload.checksum_sha256
        );

        Ok(frames)
    }

    /// Lightweight Base64 encoder.
    fn base64_encode(data: &[u8]) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

            result.push(CHARSET[(b0 >> 2) as usize] as char);
            result.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

            if chunk.len() > 1 {
                result.push(CHARSET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(CHARSET[(b2 & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }
}

// === Optical Receiver (Ingestion & Reassembly) ===

pub struct AirGapReceiver {
    sessions: RwLock<HashMap<String, IngestionSession>>,
    target_locus_dir: Option<PathBuf>,
}

#[allow(dead_code)]
struct IngestionSession {
    total_chunks: usize,
    chunks: HashMap<usize, String>,
    created_at: String,
}

impl AirGapReceiver {
    pub fn new() -> Self {
        let locus_dir = dirs::home_dir().map(|h| h.join(".locus"));
        Self::with_dir(locus_dir)
    }

    pub fn with_dir(target_locus_dir: Option<PathBuf>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            target_locus_dir,
        }
    }

    /// Ingests an incoming optical frame string and updates session progress.
    pub fn ingest_frame(&self, frame_raw: &str) -> Result<AirGapIngestProgress, AirGapError> {
        let chunk = Self::parse_frame(frame_raw)?;

        let (received_count, total, is_ready) = {
            let mut map = self.sessions.write();
            let session = map.entry(chunk.session_id.clone()).or_insert_with(|| IngestionSession {
                total_chunks: chunk.total_chunks,
                chunks: HashMap::new(),
                created_at: Utc::now().to_rfc3339(),
            });

            session.total_chunks = chunk.total_chunks;
            session.chunks.insert(chunk.index, chunk.payload_base64);

            let count = session.chunks.len();
            let total = session.total_chunks;
            (count, total, count >= total && total > 0)
        };

        let percent = if total > 0 {
            (received_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Ok(AirGapIngestProgress {
            session_id: chunk.session_id,
            received_chunks: received_count,
            total_chunks: total,
            percent_complete: percent,
            is_ready,
            error: None,
        })
    }

    /// Parses and verifies the CRC32 of a single raw frame string.
    pub fn parse_frame(frame: &str) -> Result<SyncChunk, AirGapError> {
        let parts: Vec<&str> = frame.splitn(6, ':').collect();
        if parts.len() != 6 || parts[0] != "LOCUS" || parts[1] != "v1" {
            return Err(AirGapError::InvalidFrameFormat(frame.to_string()));
        }

        let session_id = parts[2].to_string();
        let idx_parts: Vec<&str> = parts[3].split('/').collect();
        if idx_parts.len() != 2 {
            return Err(AirGapError::InvalidFrameFormat("Invalid chunk index fraction".to_string()));
        }

        let index: usize = idx_parts[0]
            .parse()
            .map_err(|_| AirGapError::InvalidFrameFormat("Non-numeric chunk index".to_string()))?;
        let total_chunks: usize = idx_parts[1]
            .parse()
            .map_err(|_| AirGapError::InvalidFrameFormat("Non-numeric total chunks".to_string()))?;

        let expected_crc = u32::from_str_radix(parts[4], 16)
            .map_err(|_| AirGapError::InvalidFrameFormat("Invalid CRC hex".to_string()))?;
        let payload_base64 = parts[5].to_string();

        let computed_crc = crc32(payload_base64.as_bytes());
        if computed_crc != expected_crc {
            return Err(AirGapError::CrcMismatch(index, expected_crc, computed_crc));
        }

        Ok(SyncChunk {
            session_id,
            index,
            total_chunks,
            payload_base64,
            chunk_crc: computed_crc,
        })
    }

    /// Reassembles, validates SHA-256, and returns the deserialized `SyncPayload`.
    pub fn get_assembled_payload(&self, session_id: &str) -> Result<SyncPayload, AirGapError> {
        let (total, chunks_map) = {
            let map = self.sessions.read();
            let session = map
                .get(session_id)
                .ok_or_else(|| AirGapError::Incomplete(0, 0))?;
            (session.total_chunks, session.chunks.clone())
        };

        if chunks_map.len() < total || total == 0 {
            return Err(AirGapError::Incomplete(chunks_map.len(), total));
        }

        // Reconstruct base64 string in sequential index order
        let mut full_b64 = String::new();
        for i in 1..=total {
            let chunk_str = chunks_map
                .get(&i)
                .ok_or_else(|| AirGapError::Incomplete(chunks_map.len(), total))?;
            full_b64.push_str(chunk_str);
        }

        // Base64 decode
        let raw_bytes = Self::base64_decode(&full_b64)
            .map_err(|e| AirGapError::SerializationError(e))?;
        let json_str = std::str::from_utf8(&raw_bytes)
            .map_err(|e| AirGapError::SerializationError(e.to_string()))?;

        let mut payload: SyncPayload = serde_json::from_str(json_str)
            .map_err(|e| AirGapError::SerializationError(e.to_string()))?;

        let expected_sha = payload.checksum_sha256.clone();

        // Verify SHA-256 by clearing checksum field and hashing
        payload.checksum_sha256 = String::new();
        let payload_raw = serde_json::to_string(&payload)
            .map_err(|e| AirGapError::SerializationError(e.to_string()))?;
        let computed_sha = sha256_hex(payload_raw.as_bytes());

        if computed_sha != expected_sha {
            return Err(AirGapError::ShaMismatch(expected_sha, computed_sha));
        }

        payload.checksum_sha256 = expected_sha;
        info!(
            "Successfully assembled and verified sync payload for session '{}'",
            session_id
        );
        Ok(payload)
    }

    /// Applies the assembled payload to the local `~/.locus/` configuration store.
    pub fn apply_payload(&self, session_id: &str) -> Result<bool, AirGapError> {
        let payload = self.get_assembled_payload(session_id)?;

        if let Some(ref dir) = self.target_locus_dir {
            let _ = fs::create_dir_all(dir);

            // 1. Write config.json
            let config_path = dir.join("config.json");
            let _ = fs::write(config_path, &payload.config_json);

            // 2. Write slots.json if present
            if let Some(ref slots_cfg) = payload.slots_config {
                let slots_path = dir.join("slots.json");
                let _ = fs::write(slots_path, slots_cfg);
            }

            info!(
                "Applied synced configuration from air-gap stream into {:?}",
                dir
            );
        }

        Ok(true)
    }

    /// Resets or clears a session from memory.
    pub fn reset_session(&self, session_id: Option<&str>) {
        let mut map = self.sessions.write();
        if let Some(id) = session_id {
            map.remove(id);
        } else {
            map.clear();
        }
    }

    /// Lightweight Base64 decoder.
    fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
        let mut clean = input.replace(['\r', '\n', ' '], "");
        while clean.len() % 4 != 0 {
            clean.push('=');
        }

        let decode_table = |c: u8| -> Result<u8, String> {
            match c {
                b'A'..=b'Z' => Ok(c - b'A'),
                b'a'..=b'z' => Ok(c - b'a' + 26),
                b'0'..=b'9' => Ok(c - b'0' + 52),
                b'+' => Ok(62),
                b'/' => Ok(63),
                b'=' => Ok(0),
                _ => Err(format!("Invalid base64 character: {}", c as char)),
            }
        };

        let bytes = clean.as_bytes();
        let mut output = Vec::with_capacity((bytes.len() / 4) * 3);

        for chunk in bytes.chunks_exact(4) {
            let b0 = decode_table(chunk[0])?;
            let b1 = decode_table(chunk[1])?;
            let b2 = decode_table(chunk[2])?;
            let b3 = decode_table(chunk[3])?;

            output.push((b0 << 2) | (b1 >> 4));
            if chunk[2] != b'=' {
                output.push(((b1 & 0x0F) << 4) | (b2 >> 2));
            }
            if chunk[3] != b'=' {
                output.push(((b2 & 0x03) << 6) | b3);
            }
        }

        Ok(output)
    }
}

impl Default for AirGapReceiver {
    fn default() -> Self {
        Self::new()
    }
}

// === Unit Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_crc32_and_sha256() {
        let sample = b"hello locus world";
        let sample_crc = crc32(sample);
        assert_ne!(sample_crc, 0);

        let sample_sha = sha256_hex(sample);
        assert_eq!(sample_sha.len(), 64);
        assert_eq!(sample_sha, sha256_hex(sample)); // Deterministic
    }

    #[test]
    fn test_airgap_export_and_reassemble_roundtrip() {
        let mut payload = SyncPayload::default();
        payload.config_json = r#"{"model":"llama3:8b","theme":"dark","port":4100}"#.to_string();
        payload.slots_config = Some(r#"{"active_context_driver":"bm25"}"#.to_string());
        payload.active_addons = vec!["rust_tools".to_string(), "git_radar".to_string()];

        let session_id = "test_sess_42";
        let frames = AirGapExporter::generate_frames(payload.clone(), session_id, 40).unwrap();

        assert!(frames.len() >= 3);

        let receiver = AirGapReceiver::with_dir(None);

        // Ingest frames out of order
        let mut shuffled = frames.clone();
        shuffled.reverse();

        for frame in shuffled {
            let progress = receiver.ingest_frame(&frame).unwrap();
            assert_eq!(progress.session_id, session_id);
        }

        let assembled = receiver.get_assembled_payload(session_id).unwrap();
        assert_eq!(assembled.config_json, payload.config_json);
        assert_eq!(assembled.slots_config, payload.slots_config);
        assert_eq!(assembled.active_addons, payload.active_addons);
    }

    #[test]
    fn test_corrupted_crc_frame_rejection() {
        let payload = SyncPayload::default();
        let frames = AirGapExporter::generate_frames(payload, "sess_err", 50).unwrap();
        let receiver = AirGapReceiver::with_dir(None);

        // Corrupt frame payload
        let mut bad_frame = frames[0].clone();
        bad_frame.push_str("X"); // Will cause CRC mismatch

        let result = receiver.ingest_frame(&bad_frame);
        assert!(result.is_err());
    }
}
