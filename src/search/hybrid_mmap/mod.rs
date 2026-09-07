//! Hybrid MMAP, SIMD-Accelerated Vector Storage & BM25 Reciprocal Rank Fusion Subsystem.

#![forbid(unsafe_code)]

pub mod hybrid;
pub mod simd;
pub mod store;

pub use hybrid::{
    FusedSearchResult, HybridQuery, HybridSearchCoordinator, HybridSearchStats,
    LexicalCandidate, LexicalKeywordMatcher, ReciprocalRankFusionEngine, RrfConfig,
    SearchFilter, SymbolDocument, SymbolKindFilter,
};
pub use simd::{
    DistanceMetric, ScoredVector, SimdBatchScanner, SimdEngineStats, SimdMathKernel,
};
pub use store::{
    ChunkId, MmapStoreConfig, MmapVectorReader, MmapVectorWriter, VectorDimension, VectorHeader,
    VectorId, VectorRecord, VectorStoreError,
};
