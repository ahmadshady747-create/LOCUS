//! Vector Writer & Fixed-Stride Binary Index Encoder.
//!
//! Serializes dense embeddings, source symbols, and chunk IDs into
//! fixed-stride binary frames protected by CRC32-C checksums in 100% safe Rust.

#![forbid(unsafe_code)]

use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};

use crate::search::hybrid_mmap::store::types::{
    MmapStoreConfig, VectorDimension, VectorHeader, VectorRecord, VectorStoreError,
};
use crate::wal::log::crc32c;

pub struct MmapVectorWriter {
    config: MmapStoreConfig,
    header: VectorHeader,
    file_handle: Option<File>,
    memory_buffer: Vec<u8>,
    is_in_memory: bool,
}

impl MmapVectorWriter {
    /// Opens or initializes a file-backed vector store writer.
    pub fn open(config: MmapStoreConfig) -> Result<Self, VectorStoreError> {
        if let Some(parent) = config.index_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    VectorStoreError::IoError(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&config.index_path)
            .map_err(|e| VectorStoreError::IoError(format!("Failed to open index file: {}", e)))?;

        let file_len = file
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);

        let header = if file_len >= VectorHeader::HEADER_SIZE as u64 {
            // Read existing header
            let mut hdr_bytes = [0u8; VectorHeader::HEADER_SIZE];
            file.seek(SeekFrom::Start(0))
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            std::io::Read::read_exact(&mut file, &mut hdr_bytes)
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;

            let mut magic = [0u8; 4];
            magic.copy_from_slice(&hdr_bytes[0..4]);
            if magic != VectorHeader::MAGIC_BYTES {
                return Err(VectorStoreError::InvalidMagic(magic));
            }

            let version = u16::from_le_bytes([hdr_bytes[4], hdr_bytes[5]]);
            let dim_num = u16::from_le_bytes([hdr_bytes[6], hdr_bytes[7]]);
            let total_count = u64::from_le_bytes(
                hdr_bytes[8..16]
                    .try_into()
                    .unwrap_or([0; 8]),
            );
            let stride = u32::from_le_bytes(
                hdr_bytes[16..20]
                    .try_into()
                    .unwrap_or([0; 4]),
            );

            if dim_num != config.target_dimension.0 {
                return Err(VectorStoreError::DimensionMismatch {
                    expected: config.target_dimension.0,
                    found: dim_num,
                });
            }

            VectorHeader {
                magic,
                version,
                dimension: VectorDimension(dim_num),
                total_vectors_count: total_count,
                stride_bytes: stride,
            }
        } else {
            // Initialize new header
            let hdr = VectorHeader::new(config.target_dimension);
            file.seek(SeekFrom::Start(0))
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            let raw_hdr = Self::encode_header(&hdr);
            file.write_all(&raw_hdr)
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            file.sync_all()
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            hdr
        };

        Ok(Self {
            config,
            header,
            file_handle: Some(file),
            memory_buffer: Vec::new(),
            is_in_memory: false,
        })
    }

    /// Opens an in-memory vector store writer for high-throughput testing and shadow indexes.
    pub fn open_in_memory(config: MmapStoreConfig) -> Self {
        let header = VectorHeader::new(config.target_dimension);
        let raw_hdr = Self::encode_header(&header);
        let mut memory_buffer = Vec::with_capacity(1024 * 1024);
        memory_buffer.extend_from_slice(&raw_hdr);

        Self {
            config,
            header,
            file_handle: None,
            memory_buffer,
            is_in_memory: true,
        }
    }

    fn encode_header(hdr: &VectorHeader) -> [u8; VectorHeader::HEADER_SIZE] {
        let mut buf = [0u8; VectorHeader::HEADER_SIZE];
        buf[0..4].copy_from_slice(&hdr.magic);
        buf[4..6].copy_from_slice(&hdr.version.to_le_bytes());
        buf[6..8].copy_from_slice(&hdr.dimension.0.to_le_bytes());
        buf[8..16].copy_from_slice(&hdr.total_vectors_count.to_le_bytes());
        buf[16..20].copy_from_slice(&hdr.stride_bytes.to_le_bytes());
        buf
    }

    /// Appends a new vector record to the store, returning its zero-based index.
    pub fn append_vector(&mut self, record: &VectorRecord) -> Result<u64, VectorStoreError> {
        let expected_dim = self.config.target_dimension.0 as usize;
        let actual_dim = record.embedding.len();
        if actual_dim != expected_dim {
            return Err(VectorStoreError::DimensionMismatch {
                expected: self.config.target_dimension.0,
                found: actual_dim as u16,
            });
        }

        if self.header.total_vectors_count as usize >= self.config.max_vectors_capacity {
            return Err(VectorStoreError::CapacityExceeded(
                self.config.max_vectors_capacity,
            ));
        }

        let stride = self.header.stride_bytes as usize;
        let mut frame_buf = Vec::with_capacity(stride);

        // 1. VectorId (8B)
        frame_buf.extend_from_slice(&record.vector_id.value().to_le_bytes());
        // 2. SymbolId (8B)
        frame_buf.extend_from_slice(&record.symbol_id.to_le_bytes());
        // 3. ChunkId (8B)
        frame_buf.extend_from_slice(&record.chunk_id.value().to_le_bytes());
        // 4. Dimension (2B)
        frame_buf.extend_from_slice(&(actual_dim as u16).to_le_bytes());
        // 5. Embedding Floats (Dim * 4B)
        for &val in &record.embedding {
            frame_buf.extend_from_slice(&val.to_le_bytes());
        }

        // 6. CRC32-C Checksum (4B over Header + Payload)
        let checksum = crc32c(&frame_buf);
        frame_buf.extend_from_slice(&checksum.to_le_bytes());

        let vector_index = self.header.total_vectors_count;

        if self.is_in_memory {
            self.memory_buffer.extend_from_slice(&frame_buf);
            self.header.total_vectors_count += 1;
            // Update header in memory buffer
            let raw_hdr = Self::encode_header(&self.header);
            self.memory_buffer[0..VectorHeader::HEADER_SIZE].copy_from_slice(&raw_hdr);
        } else if let Some(ref mut file) = self.file_handle {
            let offset = VectorHeader::HEADER_SIZE as u64 + vector_index * (stride as u64);
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            file.write_all(&frame_buf)
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;

            self.header.total_vectors_count += 1;
            // Update header in file
            file.seek(SeekFrom::Start(0))
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
            let raw_hdr = Self::encode_header(&self.header);
            file.write_all(&raw_hdr)
                .map_err(|e| VectorStoreError::IoError(e.to_string()))?;
        }

        Ok(vector_index)
    }

    /// Flushes pending buffer writes to disk.
    pub fn flush(&mut self) -> Result<(), VectorStoreError> {
        if let Some(ref mut file) = self.file_handle {
            file.flush()
                .map_err(|e| VectorStoreError::IoError(format!("Flush failed: {}", e)))?;
            file.sync_all()
                .map_err(|e| VectorStoreError::IoError(format!("Disk sync failed: {}", e)))?;
        }
        Ok(())
    }

    pub fn total_vectors(&self) -> u64 {
        self.header.total_vectors_count
    }

    pub fn memory_buffer(&self) -> &[u8] {
        &self.memory_buffer
    }
}
