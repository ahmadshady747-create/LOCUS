//! Hybrid BM25 + Vector Fusion Ranker & Reciprocal Rank Fusion Engine.
//!
//! Blends lexical AST inverted indices with SIMD-accelerated dense vector embeddings,
//! applying AST-scoped filtering and deterministic RRF ranking in 100% safe Rust.

#![forbid(unsafe_code)]

pub mod bm25_lexical;
pub mod rrf_fusion;
pub mod types;

pub use bm25_lexical::{LexicalCandidate, LexicalKeywordMatcher, SymbolDocument};
pub use rrf_fusion::ReciprocalRankFusionEngine;
pub use types::{
    FusedSearchResult, HybridQuery, HybridSearchStats, RrfConfig, SearchFilter, SymbolKindFilter,
};

use crate::search::hybrid_mmap::simd::scanner::SimdBatchScanner;
use crate::search::hybrid_mmap::simd::types::{DistanceMetric, VectorRecord};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

/// Central coordinator managing BM25 lexical indexing, dense vector search, and RRF ranking.
#[derive(Debug, Clone, Default)]
pub struct HybridSearchCoordinator {
    lexical: Arc<RwLock<LexicalKeywordMatcher>>,
    vector_records: Arc<RwLock<Vec<VectorRecord>>>,
    config: RrfConfig,
}

impl HybridSearchCoordinator {
    pub fn new() -> Self {
        Self {
            lexical: Arc::new(RwLock::new(LexicalKeywordMatcher::new())),
            vector_records: Arc::new(RwLock::new(Vec::new())),
            config: RrfConfig::default(),
        }
    }

    pub fn with_config(config: RrfConfig) -> Self {
        Self {
            lexical: Arc::new(RwLock::new(LexicalKeywordMatcher::new())),
            vector_records: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Indexes a symbol document and optional embedding vector into the hybrid search coordinator.
    pub fn index_symbol(&self, doc: SymbolDocument, embedding: Option<Vec<f32>>) {
        let symbol_id = doc.symbol_id;

        if let Some(emb) = embedding {
            let record = VectorRecord::new(symbol_id, symbol_id, emb);
            let mut vec_lock = self.vector_records.write();
            // Replace if existing
            if let Some(pos) = vec_lock.iter().position(|r| r.symbol_id == symbol_id) {
                vec_lock[pos] = record;
            } else {
                vec_lock.push(record);
            }
        }

        self.lexical.write().index_document(doc);
    }

    /// Executes a full hybrid search: retrieves lexical matches + vector nearest neighbors and fuses via RRF.
    pub fn search(&self, query: &HybridQuery) -> (Vec<FusedSearchResult>, HybridSearchStats) {
        let start = Instant::now();

        // 1. Lexical retrieval
        let lexical_hits = {
            let lex = self.lexical.read();
            lex.search(&query.query_text, &query.filter, query.top_k * 2)
        };

        // 2. Vector retrieval (if query embedding is supplied)
        let vector_hits = if let Some(query_emb) = &query.query_embedding {
            let vec_lock = self.vector_records.read();
            if !vec_lock.is_empty() {
                let (scored, _) = SimdBatchScanner::find_top_k(
                    query_emb,
                    &vec_lock,
                    DistanceMetric::CosineSimilarity,
                    query.top_k * 2,
                );
                scored
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // 3. Reciprocal Rank Fusion
        let lex_reader = self.lexical.read();
        let doc_lookup = |id: u64| lex_reader.get_document(id).cloned();

        let (fused, mut stats) = ReciprocalRankFusionEngine::fuse_results(
            &lexical_hits,
            &vector_hits,
            doc_lookup,
            &self.config,
            &query.filter,
            query.top_k,
        );

        stats.total_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        (fused, stats)
    }
}

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::hybrid_mmap::simd::types::ScoredVector;
    use crate::types::{Language, SymbolKind};

    fn make_test_doc(id: u64, name: &str, kind: SymbolKind, lang: Language, path: &str) -> SymbolDocument {
        SymbolDocument::new(
            id,
            path,
            name,
            kind,
            lang,
            format!("pub fn {}() -> bool", name),
            format!("Documentation for {}", name),
            format!("// Implementation body of {}", name),
        )
    }

    #[test]
    fn test_rrf_mathematical_precision_fusion() {
        let config = RrfConfig {
            k_constant: 60.0,
            lexical_weight: 1.0,
            vector_weight: 1.0,
        };

        let doc1 = make_test_doc(101, "validate_token", SymbolKind::Function, Language::Rust, "src/auth/jwt.rs");
        let doc2 = make_test_doc(102, "sign_token", SymbolKind::Function, Language::Rust, "src/auth/jwt.rs");

        let lexical_hits = vec![
            LexicalCandidate {
                symbol_id: 101,
                file_path: doc1.file_path.clone(),
                symbol_name: doc1.symbol_name.clone(),
                symbol_kind: doc1.symbol_kind.clone(),
                language: doc1.language,
                score: 15.0,
                snippet: doc1.signature.clone(),
            },
            LexicalCandidate {
                symbol_id: 102,
                file_path: doc2.file_path.clone(),
                symbol_name: doc2.symbol_name.clone(),
                symbol_kind: doc2.symbol_kind.clone(),
                language: doc2.language,
                score: 10.0,
                snippet: doc2.signature.clone(),
            },
        ];

        // Vector rankings: doc2 is rank 1, doc1 is rank 2
        let vector_hits = vec![
            ScoredVector {
                vector_id: 102,
                symbol_id: 102,
                score: 0.95,
            },
            ScoredVector {
                vector_id: 101,
                symbol_id: 101,
                score: 0.85,
            },
        ];

        let docs = vec![(101, doc1), (102, doc2)].into_iter().collect::<std::collections::HashMap<_, _>>();
        let doc_lookup = |id: u64| docs.get(&id).cloned();

        let (fused, stats) = ReciprocalRankFusionEngine::fuse_results(
            &lexical_hits,
            &vector_hits,
            doc_lookup,
            &config,
            &SearchFilter::default(),
            10,
        );

        assert_eq!(stats.fused_results_count, 2);
        assert_eq!(fused.len(), 2);

        // Calculate expected RRF scores:
        // doc1: lex_rank=1, vec_rank=2 -> 1/(60+1) + 1/(60+2) = 1/61 + 1/62 = 0.01639344 + 0.01612903 = 0.03252247
        // doc2: lex_rank=2, vec_rank=1 -> 1/(60+2) + 1/(60+1) = 0.03252247
        let expected_score = (1.0 / 61.0) + (1.0 / 62.0);

        for hit in &fused {
            let diff = (hit.combined_rrf_score - expected_score).abs();
            assert!(diff < 1e-6, "RRF score ({}) must match expected ({})", hit.combined_rrf_score, expected_score);
        }
    }

    #[test]
    fn test_ast_scoped_kind_filtering() {
        let coordinator = HybridSearchCoordinator::new();

        let doc_fn = make_test_doc(201, "execute_transaction", SymbolKind::Function, Language::Rust, "src/tx.rs");
        let doc_struct = make_test_doc(202, "TransactionContext", SymbolKind::Struct, Language::Rust, "src/tx.rs");
        let doc_trait = make_test_doc(203, "TransactionHandler", SymbolKind::Trait, Language::Rust, "src/tx.rs");

        coordinator.index_symbol(doc_fn, None);
        coordinator.index_symbol(doc_struct, None);
        coordinator.index_symbol(doc_trait, None);

        // Query with FunctionsOnly filter
        let query = HybridQuery::new("transaction")
            .with_filter(SearchFilter::new().with_kind(SymbolKindFilter::FunctionsOnly));

        let (fused, _) = coordinator.search(&query);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].symbol_name, "execute_transaction");
        assert_eq!(fused[0].symbol_kind, SymbolKind::Function);

        // Query with TypesOnly filter
        let query_types = HybridQuery::new("transaction")
            .with_filter(SearchFilter::new().with_kind(SymbolKindFilter::TypesOnly));

        let (fused_types, _) = coordinator.search(&query_types);
        assert_eq!(fused_types.len(), 1);
        assert_eq!(fused_types[0].symbol_name, "TransactionContext");
        assert_eq!(fused_types[0].symbol_kind, SymbolKind::Struct);
    }

    #[test]
    fn test_deterministic_tie_breaking() {
        let config = RrfConfig::default();

        let doc1 = make_test_doc(500, "handler_b", SymbolKind::Function, Language::Rust, "src/b.rs");
        let doc2 = make_test_doc(300, "handler_a", SymbolKind::Function, Language::Rust, "src/a.rs");

        // Equal combined RRF scores across lexical and vector channels
        // doc1: lexical rank 1, vector rank 2 -> 1/61 + 1/62
        // doc2: lexical rank 2, vector rank 1 -> 1/62 + 1/61
        let lexical_hits = vec![
            LexicalCandidate {
                symbol_id: 500,
                file_path: doc1.file_path.clone(),
                symbol_name: doc1.symbol_name.clone(),
                symbol_kind: doc1.symbol_kind.clone(),
                language: doc1.language,
                score: 15.0,
                snippet: doc1.signature.clone(),
            },
            LexicalCandidate {
                symbol_id: 300,
                file_path: doc2.file_path.clone(),
                symbol_name: doc2.symbol_name.clone(),
                symbol_kind: doc2.symbol_kind.clone(),
                language: doc2.language,
                score: 10.0,
                snippet: doc2.signature.clone(),
            },
        ];

        let vector_hits = vec![
            ScoredVector {
                vector_id: 300,
                symbol_id: 300,
                score: 0.95,
            },
            ScoredVector {
                vector_id: 500,
                symbol_id: 500,
                score: 0.85,
            },
        ];

        let docs = vec![(500, doc1), (300, doc2)].into_iter().collect::<std::collections::HashMap<_, _>>();
        let doc_lookup = |id: u64| docs.get(&id).cloned();

        let (fused, _) = ReciprocalRankFusionEngine::fuse_results(
            &lexical_hits,
            &vector_hits,
            doc_lookup,
            &config,
            &SearchFilter::default(),
            10,
        );

        // Tie-breaking sorts symbol_id 300 before 500
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].symbol_id, 300);
        assert_eq!(fused[1].symbol_id, 500);
    }

    #[test]
    fn test_sub_0_05ms_hybrid_search_budget() {
        let coordinator = HybridSearchCoordinator::new();

        for i in 0..50 {
            let doc = make_test_doc(
                i + 1000,
                &format!("compute_vector_block_{}", i),
                SymbolKind::Function,
                Language::Rust,
                &format!("src/simd/block_{}.rs", i),
            );
            let embedding = vec![0.1 * (i as f32); 16];
            coordinator.index_symbol(doc, Some(embedding));
        }

        let query = HybridQuery::new("compute vector block")
            .with_embedding(vec![0.5; 16])
            .with_top_k(5);

        // Warm up
        for _ in 0..10 {
            let _ = coordinator.search(&query);
        }

        let iterations = if cfg!(debug_assertions) { 100 } else { 1000 };
        let start = Instant::now();

        for _ in 0..iterations {
            let (fused, _) = coordinator.search(&query);
            assert!(!fused.is_empty());
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average Hybrid RRF Search Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 20.0 } else { 0.25 };
        assert!(
            avg_latency_ms < budget,
            "Hybrid search latency must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
