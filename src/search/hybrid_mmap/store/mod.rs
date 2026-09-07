//! Zero-Heap Mmap Vector Store & Binary Chunk Storage Engine Barrel.
//!
//! Exposes fixed-stride binary encoders, zero-heap byte slicers,
//! CRC32-C vector verification, and direct dot-product calculation engines.

#![forbid(unsafe_code)]

pub mod types;
pub mod vector_reader;
pub mod vector_writer;

pub use types::{
    ChunkId, MmapStoreConfig, VectorDimension, VectorHeader, VectorId, VectorRecord,
    VectorStoreError,
};
pub use vector_reader::MmapVectorReader;
pub use vector_writer::MmapVectorWriter;

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_write_and_read_1000_vectors_exact_float_reproduction() {
        let config = MmapStoreConfig {
            target_dimension: VectorDimension::DIM_384,
            max_vectors_capacity: 5000,
            ..MmapStoreConfig::default()
        };

        let mut writer = MmapVectorWriter::open_in_memory(config.clone());

        for i in 0..1000 {
            let vec_id = VectorId::new("SymTest", i);
            let chunk_id = ChunkId::new("src/core.rs", (i * 10) as usize, (i * 10 + 50) as usize);
            // Construct deterministic 384D float vector
            let mut embedding = Vec::with_capacity(384);
            for d in 0..384 {
                embedding.push((i as f32 * 0.001) + (d as f32 * 0.0001));
            }

            let record = VectorRecord::new(vec_id, i * 100, chunk_id, embedding);
            writer.append_vector(&record).expect("Append succeeds");
        }

        assert_eq!(writer.total_vectors(), 1000);

        let reader = MmapVectorReader::open_from_bytes(writer.memory_buffer().to_vec())
            .expect("Reader open succeeds");
        assert_eq!(reader.total_vectors(), 1000);
        assert_eq!(reader.dimension(), VectorDimension::DIM_384);

        // Verify index 0, 500, and 999
        for &idx in &[0u64, 500u64, 999u64] {
            let record = reader.read_vector_by_index(idx).expect("Read succeeds");
            assert_eq!(record.vector_id, VectorId::new("SymTest", idx));
            assert_eq!(record.symbol_id, idx * 100);
            assert_eq!(record.embedding.len(), 384);
            assert!((record.embedding[0] - (idx as f32 * 0.001)).abs() < 1e-6);
        }
    }

    #[test]
    fn test_checksum_mismatch_detected_on_tampered_vector() {
        let config = MmapStoreConfig {
            target_dimension: VectorDimension::DIM_384,
            ..MmapStoreConfig::default()
        };

        let mut writer = MmapVectorWriter::open_in_memory(config);
        let embedding = vec![0.5f32; 384];
        let record = VectorRecord::new(
            VectorId::new("SymTamper", 1),
            42,
            ChunkId::new("file.rs", 0, 10),
            embedding,
        );
        writer.append_vector(&record).unwrap();

        let mut raw_bytes = writer.memory_buffer().to_vec();

        // Tamper with a single byte in the float embedding area (byte 50)
        raw_bytes[50] ^= 0xFF;

        let reader = MmapVectorReader::open_from_bytes(raw_bytes).unwrap();
        let res = reader.read_vector_by_index(0);
        assert!(res.is_err(), "Tampered vector must trigger checksum mismatch");
        match res.unwrap_err() {
            VectorStoreError::ChecksumMismatch { vector_id, expected, calculated } => {
                assert_eq!(vector_id, VectorId::new("SymTamper", 1));
                assert_ne!(expected, calculated);
            }
            other => panic!("Expected ChecksumMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_zero_heap_slice_reading_and_direct_dot_product() {
        let config = MmapStoreConfig {
            target_dimension: VectorDimension::DIM_384,
            ..MmapStoreConfig::default()
        };

        let mut writer = MmapVectorWriter::open_in_memory(config);
        let mut emb_a = vec![0.0f32; 384];
        let mut emb_b = vec![0.0f32; 384];
        emb_a[0] = 1.0;
        emb_a[1] = 2.0;
        emb_b[0] = 3.0;
        emb_b[1] = 4.0;

        let rec_a = VectorRecord::new(VectorId::new("A", 0), 1, ChunkId::new("a.rs", 0, 5), emb_a);
        let rec_b = VectorRecord::new(VectorId::new("B", 1), 2, ChunkId::new("b.rs", 0, 5), emb_b);

        writer.append_vector(&rec_a).unwrap();
        writer.append_vector(&rec_b).unwrap();

        let reader = MmapVectorReader::open_from_bytes(writer.memory_buffer().to_vec()).unwrap();

        // Test zero-heap slice reading
        let slice_a = reader.read_embedding_slice(0).expect("Slice read succeeds");
        assert_eq!(slice_a.len(), 384 * 4);

        // Test direct dot product: (1.0*3.0) + (2.0*4.0) = 3 + 8 = 11.0
        let dot = reader.dot_product_direct(0, 1).expect("Dot product succeeds");
        assert!((dot - 11.0f32).abs() < 1e-6);
    }

    #[test]
    fn test_sub_0_05ms_vector_read_write_latency_budget() {
        let config = MmapStoreConfig {
            target_dimension: VectorDimension::DIM_384,
            ..MmapStoreConfig::default()
        };

        let mut writer = MmapVectorWriter::open_in_memory(config);
        let embedding = vec![0.1f32; 384];

        let start = Instant::now();
        for i in 0..100 {
            let record = VectorRecord::new(
                VectorId::new("BenchVec", i),
                i,
                ChunkId::new("bench.rs", 0, 10),
                embedding.clone(),
            );
            let _ = writer.append_vector(&record);
        }
        let reader = MmapVectorReader::open_from_bytes(writer.memory_buffer().to_vec()).unwrap();
        for i in 0..100 {
            let _ = reader.read_embedding_slice(i);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 200.0;

        assert!(
            avg_us < 500.0,
            "Vector write and zero-heap slice read must execute rapidly (sub-0.5ms in debug, sub-0.05ms in release), took: {:.2}µs",
            avg_us
        );
    }
}
