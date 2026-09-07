//! Binary Frame Codec & CRC32-C Integrity Verifier.
//!
//! Encodes and decodes framed binary WAL records with torn-write detection
//! and Castagnoli CRC32-C verification in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::wal::log::types::{
    TxId, WalError, WalFrame, WalFrameHeader, WalFrameType, WalVersion,
};

// ---------------------------------------------------------------------------
// CRC32-C (Castagnoli Polynomial 0x82F63B78)
// ---------------------------------------------------------------------------

const fn make_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F63B78;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

static CRC32C_TABLE: [u32; 256] = make_crc32c_table();

/// Calculates the standard CRC32-C (Castagnoli) checksum over input bytes.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        let idx = ((crc ^ (byte as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[idx];
    }
    !crc
}

// ---------------------------------------------------------------------------
// BinaryFrameCodec Implementation
// ---------------------------------------------------------------------------

pub struct BinaryFrameCodec;

impl BinaryFrameCodec {
    /// Magic header bytes: ASCII `LOCW` (`[0x4C, 0x4F, 0x43, 0x57]`).
    pub const MAGIC_BYTES: [u8; 4] = [0x4C, 0x4F, 0x43, 0x57];
    /// Binary header size in bytes: `[Magic (4B) | Version (2B) | TxID (8B) | FrameType (1B) | PayloadLen (4B)]`.
    pub const HEADER_SIZE: usize = 19;
    /// Minimum valid frame size with 0-byte payload: Header (19B) + Checksum (4B) = 23B.
    pub const MIN_FRAME_SIZE: usize = Self::HEADER_SIZE + 4;

    /// Encodes a new WAL frame into a contiguous binary byte buffer with CRC32-C checksum.
    pub fn encode_frame(
        frame_type: WalFrameType,
        tx_id: TxId,
        payload: &[u8],
    ) -> Result<Vec<u8>, WalError> {
        let payload_len = payload.len();
        if payload_len > u32::MAX as usize {
            return Err(WalError::CorruptedFrame(
                "Payload size exceeds 4GB limit".to_string(),
            ));
        }

        let total_size = Self::HEADER_SIZE + payload_len + 4;
        let mut buf = Vec::with_capacity(total_size);

        // 1. Magic (4B)
        buf.extend_from_slice(&Self::MAGIC_BYTES);

        // 2. Version (2B, Little-Endian)
        buf.extend_from_slice(&WalVersion::V1.raw().to_le_bytes());

        // 3. TxID (8B, Little-Endian)
        buf.extend_from_slice(&tx_id.value().to_le_bytes());

        // 4. FrameType (1B)
        buf.push(frame_type.as_u8());

        // 5. PayloadLen (4B, Little-Endian)
        buf.extend_from_slice(&(payload_len as u32).to_le_bytes());

        // 6. Payload (NB)
        buf.extend_from_slice(payload);

        // 7. Checksum (4B, CRC32-C over Header + Payload)
        let checksum = crc32c(&buf);
        buf.extend_from_slice(&checksum.to_le_bytes());

        Ok(buf)
    }

    /// Decodes a framed binary record from raw bytes, verifying CRC32-C and detecting torn writes.
    pub fn decode_frame(raw_bytes: &[u8]) -> Result<(WalFrame, usize), WalError> {
        let available = raw_bytes.len();
        if available < Self::MIN_FRAME_SIZE {
            return Err(WalError::TornWriteDetected {
                expected_bytes: Self::MIN_FRAME_SIZE,
                available_bytes: available,
            });
        }

        // 1. Validate Magic
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&raw_bytes[0..4]);
        if magic != Self::MAGIC_BYTES {
            return Err(WalError::InvalidMagic(magic));
        }

        // 2. Validate Version
        let mut ver_bytes = [0u8; 2];
        ver_bytes.copy_from_slice(&raw_bytes[4..6]);
        let version_num = u16::from_le_bytes(ver_bytes);
        if version_num != WalVersion::V1.raw() {
            return Err(WalError::UnsupportedVersion(version_num));
        }
        let version = WalVersion(version_num);

        // 3. Read TxID
        let mut tx_bytes = [0u8; 8];
        tx_bytes.copy_from_slice(&raw_bytes[6..14]);
        let tx_id = TxId::from_raw(u64::from_le_bytes(tx_bytes));

        // 4. Read FrameType
        let frame_type_byte = raw_bytes[14];
        let frame_type = WalFrameType::from_u8(frame_type_byte)
            .ok_or(WalError::UnknownFrameType(frame_type_byte))?;

        // 5. Read PayloadLen
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&raw_bytes[15..19]);
        let payload_len = u32::from_le_bytes(len_bytes) as usize;

        let total_frame_len = Self::HEADER_SIZE + payload_len + 4;
        if available < total_frame_len {
            return Err(WalError::TornWriteDetected {
                expected_bytes: total_frame_len,
                available_bytes: available,
            });
        }

        // 6. Extract Payload
        let payload = raw_bytes[Self::HEADER_SIZE..Self::HEADER_SIZE + payload_len].to_vec();

        // 7. Verify CRC32-C Checksum
        let mut crc_bytes = [0u8; 4];
        crc_bytes.copy_from_slice(&raw_bytes[Self::HEADER_SIZE + payload_len..total_frame_len]);
        let expected_checksum = u32::from_le_bytes(crc_bytes);

        let computed_checksum = crc32c(&raw_bytes[..Self::HEADER_SIZE + payload_len]);
        if computed_checksum != expected_checksum {
            return Err(WalError::ChecksumMismatch {
                expected: expected_checksum,
                found: computed_checksum,
            });
        }

        let header = WalFrameHeader {
            magic,
            version,
            tx_id,
            frame_type,
            payload_len: payload_len as u32,
        };

        Ok((
            WalFrame {
                header,
                payload,
                checksum: expected_checksum,
            },
            total_frame_len,
        ))
    }
}
