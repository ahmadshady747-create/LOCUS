//! Binary WAL Writer & Sequential Append Stream.
//!
//! Manages sequential append operations, segment file management,
//! torn-write protection, and configurable disk fsync durability policies.

#![forbid(unsafe_code)]

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::wal::log::frame_encoder::BinaryFrameCodec;
use crate::wal::log::types::{
    FsyncPolicy, TxId, WalConfig, WalError, WalFrame, WalFrameType,
};

pub struct BinaryWalWriter {
    config: WalConfig,
    current_segment_index: u32,
    current_segment_size: usize,
    total_bytes_written: u64,
    file_handle: Option<File>,
    memory_buffer: Vec<u8>,
    is_in_memory: bool,
}

impl BinaryWalWriter {
    /// Opens or creates a file-backed Binary WAL writer.
    pub fn open(config: WalConfig) -> Result<Self, WalError> {
        if !config.log_directory.exists() {
            fs::create_dir_all(&config.log_directory)
                .map_err(|e| WalError::IoError(format!("Failed to create log directory: {}", e)))?;
        }

        let segment_index = 1u32;
        let segment_path = Self::compute_segment_path(&config, segment_index);

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&segment_path)
            .map_err(|e| WalError::IoError(format!("Failed to open segment file: {}", e)))?;

        let initial_size = file
            .metadata()
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        Ok(Self {
            config,
            current_segment_index: segment_index,
            current_segment_size: initial_size,
            total_bytes_written: initial_size as u64,
            file_handle: Some(file),
            memory_buffer: Vec::new(),
            is_in_memory: false,
        })
    }

    /// Opens an in-memory shadow stream WAL writer (for ultra-fast testing and shadow buffers).
    pub fn open_in_memory(config: WalConfig) -> Self {
        Self {
            config,
            current_segment_index: 1,
            current_segment_size: 0,
            total_bytes_written: 0,
            file_handle: None,
            memory_buffer: Vec::with_capacity(1024 * 1024),
            is_in_memory: true,
        }
    }

    fn compute_segment_path(config: &WalConfig, index: u32) -> PathBuf {
        config
            .log_directory
            .join(format!("{}_{:05}.wal", config.segment_prefix, index))
    }

    /// Appends a `BeginTx` marker frame to the WAL.
    pub fn append_begin(&mut self, tx_id: TxId) -> Result<u64, WalError> {
        self.append_frame(WalFrameType::BeginTx, tx_id, &[])
    }

    /// Appends a `SubtreeMutation` delta frame to the WAL.
    pub fn append_mutation(&mut self, tx_id: TxId, payload: &[u8]) -> Result<u64, WalError> {
        self.append_frame(WalFrameType::SubtreeMutation, tx_id, payload)
    }

    /// Appends a `CommitTx` marker frame to the WAL.
    pub fn append_commit(&mut self, tx_id: TxId) -> Result<u64, WalError> {
        self.append_frame(WalFrameType::CommitTx, tx_id, &[])
    }

    /// Appends an `AbortTx` rollback frame to the WAL.
    pub fn append_abort(&mut self, tx_id: TxId) -> Result<u64, WalError> {
        self.append_frame(WalFrameType::AbortTx, tx_id, &[])
    }

    /// Appends a `CheckpointMarker` frame to the WAL.
    pub fn append_checkpoint(&mut self, checkpoint_id: u64, payload: &[u8]) -> Result<u64, WalError> {
        let tx_id = TxId::from_raw(checkpoint_id);
        self.append_frame(WalFrameType::CheckpointMarker, tx_id, payload)
    }

    /// Core frame append implementation.
    pub fn append_frame(
        &mut self,
        frame_type: WalFrameType,
        tx_id: TxId,
        payload: &[u8],
    ) -> Result<u64, WalError> {
        let encoded_bytes = BinaryFrameCodec::encode_frame(frame_type, tx_id, payload)?;
        let frame_len = encoded_bytes.len();

        // Check if segment rotation is needed
        if !self.is_in_memory
            && self.current_segment_size + frame_len > self.config.max_segment_size_bytes
        {
            self.rotate_segment()?;
        }

        let write_offset = self.total_bytes_written;

        if self.is_in_memory {
            self.memory_buffer.extend_from_slice(&encoded_bytes);
        } else if let Some(ref mut file) = self.file_handle {
            file.write_all(&encoded_bytes)
                .map_err(|e| WalError::IoError(format!("Failed to write to WAL segment: {}", e)))?;

            if self.config.fsync_policy == FsyncPolicy::Immediate {
                file.sync_all()
                    .map_err(|e| WalError::IoError(format!("Failed to sync WAL to disk: {}", e)))?;
            }
        }

        self.current_segment_size += frame_len;
        self.total_bytes_written += frame_len as u64;

        Ok(write_offset)
    }

    /// Rotates to the next WAL segment file (`segment_00002.wal`).
    fn rotate_segment(&mut self) -> Result<(), WalError> {
        if let Some(ref mut file) = self.file_handle {
            let _ = file.sync_all();
        }

        self.current_segment_index += 1;
        let next_path = Self::compute_segment_path(&self.config, self.current_segment_index);

        let new_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&next_path)
            .map_err(|e| WalError::IoError(format!("Failed to rotate to new segment: {}", e)))?;

        self.file_handle = Some(new_file);
        self.current_segment_size = 0;

        Ok(())
    }

    /// Flushes all pending writes and synchronizes with persistent storage.
    pub fn flush_and_sync(&mut self) -> Result<(), WalError> {
        if let Some(ref mut file) = self.file_handle {
            file.flush()
                .map_err(|e| WalError::IoError(format!("Flush failed: {}", e)))?;
            file.sync_all()
                .map_err(|e| WalError::IoError(format!("Disk sync_all failed: {}", e)))?;
        }
        Ok(())
    }

    /// Reads and decodes all frames from the beginning of the log for recovery.
    pub fn read_all_frames(&mut self) -> Result<Vec<WalFrame>, WalError> {
        let raw_bytes = if self.is_in_memory {
            self.memory_buffer.clone()
        } else if let Some(ref mut file) = self.file_handle {
            let _ = file.flush();
            let mut content = Vec::new();
            let mut read_handle = File::open(Self::compute_segment_path(
                &self.config,
                self.current_segment_index,
            ))
            .map_err(|e| WalError::IoError(format!("Failed to open segment for read: {}", e)))?;
            read_handle
                .read_to_end(&mut content)
                .map_err(|e| WalError::IoError(format!("Failed to read segment: {}", e)))?;
            content
        } else {
            Vec::new()
        };

        let mut frames = Vec::new();
        let mut offset = 0;
        let len = raw_bytes.len();

        while offset < len {
            let remaining = &raw_bytes[offset..];
            let (frame, consumed) = BinaryFrameCodec::decode_frame(remaining)?;
            frames.push(frame);
            offset += consumed;
        }

        Ok(frames)
    }

    pub fn total_bytes_written(&self) -> u64 {
        self.total_bytes_written
    }

    pub fn current_segment_index(&self) -> u32 {
        self.current_segment_index
    }
}
