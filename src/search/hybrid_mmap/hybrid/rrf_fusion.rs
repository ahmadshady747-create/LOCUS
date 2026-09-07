//! Reciprocal Rank Fusion (RRF) & Multi-Channel Ranking Engine.
//!
//! Fuses BM25 lexical search rankings and SIMD dense vector rankings using deterministic
//! reciprocal rank scores with AST-scoped filtering in 100% pure safe Rust.

#![forbid(unsafe_code)]

use crate::search::hybrid_mmap::hybrid::bm25_lexical::{LexicalCandidate, SymbolDocument};
use crate::search::hybrid_mmap::hybrid::types::{
    FusedSearchResult, HybridSearchStats, RrfConfig, SearchFilter,
};
use crate::search::hybrid_mmap::simd::types::ScoredVector;
use std::collections::HashMap;

/// Pure, deterministic Reciprocal Rank Fusion computation engine.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReciprocalRankFusionEngine;

impl ReciprocalRankFusionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Fuses lexical candidate rankings and vector candidate rankings into a unified top-K list.
    pub fn fuse_results<F>(
        lexical_hits: &[LexicalCandidate],
        vector_hits: &[ScoredVector],
        doc_lookup: F,
        config: &RrfConfig,
        filter: &SearchFilter,
        top_k: usize,
    ) -> (Vec<FusedSearchResult>, HybridSearchStats)
    where
        F: Fn(u64) -> Option<SymbolDocument>,
    {
        let k = config.k_constant;
        let w_lex = config.lexical_weight;
        let w_vec = config.vector_weight;

        // Map: symbol_id -> (lexical_rank, vector_rank, accumulated_rrf_score)
        let mut rank_map: HashMap<u64, (Option<usize>, Option<usize>, f32)> = HashMap::new();

        // 1. Process Lexical Candidates (1-based rank)
        for (idx, cand) in lexical_hits.iter().enumerate() {
            let rank = idx + 1;
            let entry = rank_map.entry(cand.symbol_id).or_insert((None, None, 0.0));
            entry.0 = Some(rank);
            entry.2 += w_lex / (k + rank as f32);
        }

        // 2. Process Vector Candidates (1-based rank)
        for (idx, cand) in vector_hits.iter().enumerate() {
            let rank = idx + 1;
            let entry = rank_map.entry(cand.symbol_id).or_insert((None, None, 0.0));
            entry.1 = Some(rank);
            entry.2 += w_vec / (k + rank as f32);
        }

        // 3. Assemble Fused Candidates and enforce SearchFilter
        let mut fused_candidates: Vec<FusedSearchResult> = Vec::with_capacity(rank_map.len());

        for (symbol_id, (lex_rank, vec_rank, rrf_score)) in rank_map {
            if let Some(doc) = doc_lookup(symbol_id) {
                if filter.matches(doc.language, &doc.symbol_kind, &doc.file_path) {
                    fused_candidates.push(FusedSearchResult {
                        symbol_id,
                        file_path: doc.file_path,
                        symbol_name: doc.symbol_name,
                        symbol_kind: doc.symbol_kind,
                        language: doc.language,
                        lexical_rank: lex_rank,
                        vector_rank: vec_rank,
                        combined_rrf_score: rrf_score,
                        snippet: doc.signature,
                    });
                }
            }
        }

        // 4. Deterministic Sort: score descending, then symbol_id ascending
        fused_candidates.sort_by(|a, b| {
            let diff = b.combined_rrf_score - a.combined_rrf_score;
            if diff.abs() > 1e-7 {
                b.combined_rrf_score
                    .partial_cmp(&a.combined_rrf_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.symbol_id.cmp(&b.symbol_id)
            }
        });

        let total_fused = fused_candidates.len();
        if top_k > 0 && fused_candidates.len() > top_k {
            fused_candidates.truncate(top_k);
        }

        let stats = HybridSearchStats {
            lexical_candidates_count: lexical_hits.len(),
            vector_candidates_count: vector_hits.len(),
            fused_results_count: total_fused,
            total_latency_ms: 0.0,
        };

        (fused_candidates, stats)
    }
}
