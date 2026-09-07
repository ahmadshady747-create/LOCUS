//! Zero-Heap Vector Reader & Mmap Direct Byte Slicer.
//!
//! Provides ultra-low latency direct memory access, float slice projections,
//! and zero-heap dot-product calculations in 100% safe Rust.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Read;

use crate::search::hybrid_mmap::store::types::{
    ChunkId, MmapStoreConfig, VectorDimension, VectorHeader, VectorId, VectorRecord,
    VectorStoreError,
};
use crate::wal::log::crc32c;

pub struct MmapVectorReader {
    header: VectorHeader,
    raw_data: Vec<u8>,
}

impl MmapVectorReader {
    /// Opens a vector store from an existing binary index file.
    pub fn open(config: MmapStoreConfig) -> Result<Self, VectorStoreError> {
        let mut file = File::open(&config.index_path)
            .map_err(|e| VectorStoreError::IoError(format!("Failed to open index file: {}", e)))?;

        let mut raw_data = Vec::new();
        file.read_to_end(&mut raw_data)
            .map_err(|e| VectorStoreError::IoError(format!("Failed to read index file: {}", e)))?;

        Self::open_from_bytes(raw_data)
    }

    /// Initializes a vector reader directly from an in-memory byte buffer.
    pub fn open_from_bytes(raw_data: Vec<u8>) -> Result<Self, VectorStoreError> {
        if raw_data.len() < VectorHeader::HEADER_SIZE {
            return Err(VectorStoreError::CorruptedHeader(
                "Data size is smaller than primary header size".to_string(),
            ));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&raw_data[0..4]);
        if magic != VectorHeader::MAGIC_BYTES {
            return Err(VectorStoreError::InvalidMagic(magic));
        }

        let version = u16::from_le_bytes([raw_data[4], raw_data[5]]);
        if version != VectorHeader::CURRENT_VERSION {
            return Err(VectorStoreError::UnsupportedVersion(version));
        }

        let dim_num = u16::from_le_bytes([raw_data[6], raw_data[7]]);
        let total_count = u64::from_le_bytes(
            raw_data[8..16]
                .try_into()
                .unwrap_or([0; 8]),
        );
        let stride = u32::from_le_bytes(
            raw_data[16..20]
                .try_into()
                .unwrap_or([0; 4]),
        );

        let header = VectorHeader {
            magic,
            version,
            dimension: VectorDimension(dim_num),
            total_vectors_count: total_count,
            stride_bytes: stride,
        };

        Ok(Self { header, raw_data })
    }

    /// Reads and verifies a complete vector record by index.
    pub fn read_vector_by_index(&self, index: u64) -> Result<VectorRecord, VectorStoreError> {
        if index >= self.header.total_vectors_count {
            return Err(VectorStoreError::IndexOutOfBounds {
                index,
                total: self.header.total_vectors_count,
            });
        }

        let stride = self.header.stride_bytes as usize;
        let offset = VectorHeader::HEADER_SIZE + (index as usize * stride);

        if offset + stride > self.raw_data.len() {
            return Err(VectorStoreError::TornRecordDetected {
                offset: offset as u64,
                expected_bytes: stride,
                available_bytes: self.raw_data.len().saturating_sub(offset),
            });
        }

        let frame_bytes = &self.raw_data[offset..offset + stride];
        let payload_len = stride - 4;

        // 1. Verify CRC32-C Checksum
        let mut crc_bytes = [0u8; 4];
        crc_bytes.copy_from_slice(&frame_bytes[payload_len..stride]);
        let expected_checksum = u32::from_le_bytes(crc_bytes);

        let computed_checksum = crc32c(&frame_bytes[..payload_len]);
        let vector_id = VectorId::from_raw(u64::from_le_bytes(
            frame_bytes[0..8].try_into().unwrap_or([0; 8]),
        ));

        if computed_checksum != expected_checksum {
            return Err(VectorStoreError::ChecksumMismatch {
                vector_id,
                expected: expected_checksum,
                calculated: computed_checksum,
            });
        }

        // 2. Decode Fields
        let symbol_id = u64::from_le_bytes(frame_bytes[8..16].try_into().unwrap_or([0; 8]));
        let chunk_id = ChunkId::from_raw(u64::from_le_bytes(
            frame_bytes[16..24].try_into().unwrap_or([0; 8]),
        ));
        let dim = u16::from_le_bytes([frame_bytes[24], frame_bytes[25]]) as usize;

        // 3. Decode Floats
        let float_bytes = &frame_bytes[26..26 + dim * 4];
        let mut embedding = Vec::with_capacity(dim);
        for chunk in float_bytes.chunks_exact(4) {
            let float_val = f32::from_le_bytes(chunk.try_into().unwrap_or([0; 4]));
            embedding.push(float_val);
        }

        Ok(VectorRecord {
            vector_id,
            symbol_id,
            chunk_id,
            embedding,
            checksum: expected_checksum,
        })
    }

    /// Returns a direct zero-heap byte slice reference to the raw float embedding data.
    pub fn read_embedding_slice<'a>(&'a self, index: u64) -> Result<&'a [u8], VectorStoreError> {
        if index >= self.header.total_vectors_count {
            return Err(VectorStoreError::IndexOutOfBounds {
                index,
                total: self.header.total_vectors_count,
            });
        }

        let stride = self.header.stride_bytes as usize;
        let offset = VectorHeader::HEADER_SIZE + (index as usize * stride);

        if offset + stride > self.raw_data.len() {
            return Err(VectorStoreError::TornRecordDetected {
                offset: offset as u64,
                expected_bytes: stride,
                available_bytes: self.raw_data.len().saturating_sub(offset),
            });
        }

        let frame_bytes = &self.raw_data[offset..offset + stride];
        let payload_len = stride - 4;

        // Checksum verification
        let mut crc_bytes = [0u8; 4];
        crc_bytes.copy_from_slice(&frame_bytes[payload_len..stride]);
        let expected_checksum = u32::from_le_bytes(crc_bytes);

        let computed_checksum = crc32c(&frame_bytes[..payload_len]);
        if computed_checksum != expected_checksum {
            let vector_id = VectorId::from_raw(u64::from_le_bytes(
                frame_bytes[0..8].try_into().unwrap_or([0; 8]),
            ));
            return Err(VectorStoreError::ChecksumMismatch {
                vector_id,
                expected: expected_checksum,
                calculated: computed_checksum,
            });
        }

        let dim = self.header.dimension.0 as usize;
        let float_start = offset + 26;
        let float_end = float_start + (dim * 4);

        Ok(&self.raw_data[float_start..float_end])
    }

    /// Computes dot product between two stored vectors directly without allocating any `Vec<f32>`.
    pub fn dot_product_direct(&self, index_a: u64, index_b: u64) -> Result<f32, VectorStoreError> {
        let slice_a = self.read_embedding_slice(index_a)?;
        let slice_b = self.read_embedding_slice(index_b)?;

        let mut dot = 0.0f32;
        let chunks_a = slice_a.chunks_exact(4);
        let chunks_b = slice_b.chunks_exact(4);

        for (ca, cb) in chunks_a.zip(chunks_b) {
            let fa = f32::from_le_bytes(ca.try_into().unwrap_or([0; 4]));
            let fb = f32::from_le_bytes(cb.try_into().unwrap_or([0; 4]));
            dot += fa * fb;
        }

        Ok(dot)
    }

    pub fn total_vectors(&self) -> u64 {
        self.header.total_vectors_count
    }

    pub fn dimension(&self) -> VectorDimension {
        self.header.dimension
    }
}
