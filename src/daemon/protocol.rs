//! Sub-Microsecond Binary IPC Wire Protocol & Framing Codec.
//!
//! Encodes and decodes framed binary IPC commands and responses with Castagnoli CRC32-C
//! integrity checks in 100% pure safe Rust.

#![forbid(unsafe_code)]

use crate::hud::telemetry_collector::TelemetrySnapshot;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Calculates the standard CRC32-C checksum over bytes in 100% safe Rust.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        let idx = ((crc ^ (byte as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[idx];
    }
    !crc
}

// ---------------------------------------------------------------------------
// Protocol Commands & Responses
// ---------------------------------------------------------------------------

/// High-level IPC commands accepted by the LOCUS daemon server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IpcCommand {
    /// Latency probe ping with 64-bit nonce.
    Ping { nonce: u64 },
    /// Submits a task to the sovereign swarm execution pool.
    ExecuteSwarmTask {
        task_id: u64,
        prompt: String,
        target_path: String,
    },
    /// Requests an instant telemetry snapshot for HUD/IDE dashboards.
    GetTelemetrySnapshot,
    /// Requests mid-execution freeze and contract delta creation.
    TriggerContractFreeze { contract_id: String },
    /// Requests graceful shutdown of the daemon server.
    Shutdown,
}

/// High-level IPC responses returned by the LOCUS daemon server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IpcResponse {
    /// Response to `Ping` carrying timestamp and nonce.
    Pong { nonce: u64, timestamp_ns: u64 },
    /// Task execution acknowledgment.
    TaskAck { task_id: u64, status: String },
    /// Telemetry data snapshot.
    TelemetryData(TelemetrySnapshot),
    /// Freeze operation acknowledgment.
    FreezeAck { contract_id: String, success: bool },
    /// Server shutdown acknowledgment.
    ShutdownAck,
    /// Detailed error receipt.
    ErrorReceipt { code: u32, message: String },
}

// ---------------------------------------------------------------------------
// Frame Types & Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    CmdPing = 0x0001,
    CmdExecuteSwarmTask = 0x0002,
    CmdGetTelemetrySnapshot = 0x0003,
    CmdTriggerContractFreeze = 0x0004,
    CmdShutdown = 0x0005,

    RespPong = 0x8001,
    RespTaskAck = 0x8002,
    RespTelemetryData = 0x8003,
    RespFreezeAck = 0x8004,
    RespShutdownAck = 0x8005,
    RespErrorReceipt = 0x80FF,
}

impl FrameType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            0x0001 => Some(Self::CmdPing),
            0x0002 => Some(Self::CmdExecuteSwarmTask),
            0x0003 => Some(Self::CmdGetTelemetrySnapshot),
            0x0004 => Some(Self::CmdTriggerContractFreeze),
            0x0005 => Some(Self::CmdShutdown),
            0x8001 => Some(Self::RespPong),
            0x8002 => Some(Self::RespTaskAck),
            0x8003 => Some(Self::RespTelemetryData),
            0x8004 => Some(Self::RespFreezeAck),
            0x8005 => Some(Self::RespShutdownAck),
            0x80FF => Some(Self::RespErrorReceipt),
            _ => None,
        }
    }
}

/// Errors occurring during IPC framing, encoding, or decoding.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IpcProtocolError {
    #[error("Invalid magic header bytes: expected b\"LOCUSIPC\"")]
    InvalidMagic,
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u16),
    #[error("Unknown frame type discriminator: {0:#06x}")]
    UnknownFrameType(u16),
    #[error("Frame truncated: expected at least {expected} bytes, got {actual}")]
    TruncatedFrame { expected: usize, actual: usize },
    #[error("CRC32 checksum mismatch: expected {expected:#010x}, calculated {calculated:#010x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },
    #[error("JSON serialization/deserialization failed: {0}")]
    CodecFailure(String),
}

// ---------------------------------------------------------------------------
// Binary Frame Codec
// ---------------------------------------------------------------------------

pub struct IpcFrameCodec;

impl IpcFrameCodec {
    /// Magic header: 8 bytes ASCII `b"LOCUSIPC"`.
    pub const MAGIC: [u8; 8] = [0x4C, 0x4F, 0x43, 0x55, 0x53, 0x49, 0x50, 0x43];
    pub const VERSION: u16 = 1;
    /// Header size: 8 (Magic) + 2 (Version) + 2 (Type) + 4 (Len) + 4 (CRC32) = 20 bytes.
    pub const HEADER_SIZE: usize = 20;

    /// Encodes an `IpcCommand` into a framed binary byte vector.
    pub fn encode_command(cmd: &IpcCommand) -> Vec<u8> {
        let (ft, payload) = match cmd {
            IpcCommand::Ping { nonce } => (
                FrameType::CmdPing,
                serde_json::to_vec(nonce).unwrap_or_default(),
            ),
            IpcCommand::ExecuteSwarmTask { .. } => (
                FrameType::CmdExecuteSwarmTask,
                serde_json::to_vec(cmd).unwrap_or_default(),
            ),
            IpcCommand::GetTelemetrySnapshot => (FrameType::CmdGetTelemetrySnapshot, Vec::new()),
            IpcCommand::TriggerContractFreeze { .. } => (
                FrameType::CmdTriggerContractFreeze,
                serde_json::to_vec(cmd).unwrap_or_default(),
            ),
            IpcCommand::Shutdown => (FrameType::CmdShutdown, Vec::new()),
        };

        Self::encode_raw_frame(ft as u16, &payload)
    }

    /// Decodes an `IpcCommand` from framed binary bytes.
    pub fn decode_command(bytes: &[u8]) -> Result<IpcCommand, IpcProtocolError> {
        let (frame_type, payload) = Self::decode_raw_frame(bytes)?;
        match frame_type {
            FrameType::CmdPing => {
                let nonce: u64 = serde_json::from_slice(&payload)
                    .map_err(|e| IpcProtocolError::CodecFailure(e.to_string()))?;
                Ok(IpcCommand::Ping { nonce })
            }
            FrameType::CmdExecuteSwarmTask => serde_json::from_slice(&payload)
                .map_err(|e| IpcProtocolError::CodecFailure(e.to_string())),
            FrameType::CmdGetTelemetrySnapshot => Ok(IpcCommand::GetTelemetrySnapshot),
            FrameType::CmdTriggerContractFreeze => serde_json::from_slice(&payload)
                .map_err(|e| IpcProtocolError::CodecFailure(e.to_string())),
            FrameType::CmdShutdown => Ok(IpcCommand::Shutdown),
            _ => Err(IpcProtocolError::UnknownFrameType(frame_type as u16)),
        }
    }

    /// Encodes an `IpcResponse` into a framed binary byte vector.
    pub fn encode_response(resp: &IpcResponse) -> Vec<u8> {
        let (ft, payload) = match resp {
            IpcResponse::Pong { nonce, timestamp_ns } => (
                FrameType::RespPong,
                serde_json::to_vec(&(nonce, timestamp_ns)).unwrap_or_default(),
            ),
            IpcResponse::TaskAck { .. } => (
                FrameType::RespTaskAck,
                serde_json::to_vec(resp).unwrap_or_default(),
            ),
            IpcResponse::TelemetryData(snapshot) => (
                FrameType::RespTelemetryData,
                serde_json::to_vec(snapshot).unwrap_or_default(),
            ),
            IpcResponse::FreezeAck { .. } => (
                FrameType::RespFreezeAck,
                serde_json::to_vec(resp).unwrap_or_default(),
            ),
            IpcResponse::ShutdownAck => (FrameType::RespShutdownAck, Vec::new()),
            IpcResponse::ErrorReceipt { .. } => (
                FrameType::RespErrorReceipt,
                serde_json::to_vec(resp).unwrap_or_default(),
            ),
        };

        Self::encode_raw_frame(ft as u16, &payload)
    }

    /// Decodes an `IpcResponse` from framed binary bytes.
    pub fn decode_response(bytes: &[u8]) -> Result<IpcResponse, IpcProtocolError> {
        let (frame_type, payload) = Self::decode_raw_frame(bytes)?;
        match frame_type {
            FrameType::RespPong => {
                let (nonce, timestamp_ns): (u64, u64) = serde_json::from_slice(&payload)
                    .map_err(|e| IpcProtocolError::CodecFailure(e.to_string()))?;
                Ok(IpcResponse::Pong {
                    nonce,
                    timestamp_ns,
                })
            }
            FrameType::RespTaskAck => serde_json::from_slice(&payload)
                .map_err(|e| IpcProtocolError::CodecFailure(e.to_string())),
            FrameType::RespTelemetryData => {
                let snapshot: TelemetrySnapshot = serde_json::from_slice(&payload)
                    .map_err(|e| IpcProtocolError::CodecFailure(e.to_string()))?;
                Ok(IpcResponse::TelemetryData(snapshot))
            }
            FrameType::RespFreezeAck => serde_json::from_slice(&payload)
                .map_err(|e| IpcProtocolError::CodecFailure(e.to_string())),
            FrameType::RespShutdownAck => Ok(IpcResponse::ShutdownAck),
            FrameType::RespErrorReceipt => serde_json::from_slice(&payload)
                .map_err(|e| IpcProtocolError::CodecFailure(e.to_string())),
            _ => Err(IpcProtocolError::UnknownFrameType(frame_type as u16)),
        }
    }

    /// Low-level raw frame encoder.
    pub fn encode_raw_frame(type_val: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER_SIZE + payload.len());
        out.extend_from_slice(&Self::MAGIC);
        out.extend_from_slice(&Self::VERSION.to_be_bytes());
        out.extend_from_slice(&type_val.to_be_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());

        let checksum = crc32c(payload);
        out.extend_from_slice(&checksum.to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    /// Low-level raw frame decoder.
    pub fn decode_raw_frame(bytes: &[u8]) -> Result<(FrameType, Vec<u8>), IpcProtocolError> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(IpcProtocolError::TruncatedFrame {
                expected: Self::HEADER_SIZE,
                actual: bytes.len(),
            });
        }

        if &bytes[0..8] != Self::MAGIC {
            return Err(IpcProtocolError::InvalidMagic);
        }

        let version = u16::from_be_bytes([bytes[8], bytes[9]]);
        if version != Self::VERSION {
            return Err(IpcProtocolError::UnsupportedVersion(version));
        }

        let type_val = u16::from_be_bytes([bytes[10], bytes[11]]);
        let frame_type = FrameType::from_u16(type_val)
            .ok_or(IpcProtocolError::UnknownFrameType(type_val))?;

        let payload_len = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let expected_checksum = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        let total_expected = Self::HEADER_SIZE + payload_len;
        if bytes.len() < total_expected {
            return Err(IpcProtocolError::TruncatedFrame {
                expected: total_expected,
                actual: bytes.len(),
            });
        }

        let payload = bytes[Self::HEADER_SIZE..total_expected].to_vec();
        let calculated_checksum = crc32c(&payload);

        if calculated_checksum != expected_checksum {
            return Err(IpcProtocolError::ChecksumMismatch {
                expected: expected_checksum,
                calculated: calculated_checksum,
            });
        }

        Ok((frame_type, payload))
    }
}
