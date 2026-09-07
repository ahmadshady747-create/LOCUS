//! In-Memory Hybrid AST + Quantized HNSW Vector Search & BM25 Reciprocal Rank Fusion module.

#![forbid(unsafe_code)]

pub mod hnsw_index;
pub mod hybrid_matcher;
pub mod hybrid_mmap;
pub mod index;

pub use hybrid_matcher::HybridMatcher;
pub use hybrid_mmap::{
    ChunkId, DistanceMetric, FusedSearchResult, HybridQuery, HybridSearchCoordinator,
    HybridSearchStats, LexicalCandidate, LexicalKeywordMatcher, MmapStoreConfig, MmapVectorReader,
    MmapVectorWriter, ReciprocalRankFusionEngine, RrfConfig, ScoredVector, SearchFilter,
    SimdBatchScanner, SimdEngineStats, SimdMathKernel, SymbolDocument, SymbolKindFilter,
    VectorDimension, VectorHeader, VectorId, VectorRecord, VectorStoreError,
};
pub use index::*;
