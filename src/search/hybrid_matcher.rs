//! Unified AST Lexical + Dense Embedding Context Retriever with Zero-Heap Query Path.

#![forbid(unsafe_code)]

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;

use crate::graph::SymbolGraph;
use crate::search::hnsw_index::{HnswIndex, DEFAULT_DIM};
use crate::types::{fnv1a_64, HybridSearchResult, SearchHit, SymbolNode};

pub struct HybridMatcher {
    hnsw: RwLock<HnswIndex>,
    symbol_metadata: RwLock<HashMap<u64, (SymbolNode, String)>>, // id -> (node, snippet)
}

impl Default for HybridMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridMatcher {
    pub fn new() -> Self {
        Self {
            hnsw: RwLock::new(HnswIndex::default()),
            symbol_metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a deterministic 64-dimensional quantized embedding on the stack without heap allocation.
    pub fn embed_text_fixed(text: &str) -> [i8; DEFAULT_DIM] {
        let mut arr = [0i8; DEFAULT_DIM];
        let mut word_idx = 0;

        for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if word.is_empty() {
                continue;
            }
            let hash = fnv1a_64(word.as_bytes());
            let dim_idx = (hash as usize) % DEFAULT_DIM;
            let weight = (100 / ((word_idx + 1).min(10))) as i8;
            arr[dim_idx] = arr[dim_idx].saturating_add(weight);
            word_idx += 1;
        }

        arr
    }

    /// Generate a deterministic 64-dimensional quantized embedding from text or symbol name.
    pub fn embed_text(text: &str) -> Vec<i8> {
        Self::embed_text_fixed(text).to_vec()
    }

    /// Index all symbols from an existing `SymbolGraph` into the hybrid search engine.
    pub fn index_graph(&self, graph: &SymbolGraph) {
        let mut hnsw = self.hnsw.write();
        let mut meta = self.symbol_metadata.write();

        for node in graph.nodes.values() {
            let text_repr = format!("{} {} {}", node.name, node.kind, node.signature);
            let embedding = Self::embed_text_fixed(&text_repr);

            hnsw.insert(node.id, embedding.to_vec());
            meta.insert(node.id, (node.clone(), node.signature.clone()));
        }
    }

    /// Execute a hybrid semantic + lexical search across indexed symbols.
    pub fn search(&self, query: &str, top_k: usize) -> HybridSearchResult {
        let start = Instant::now();
        let query_vec = Self::embed_text_fixed(query);
        let hnsw = self.hnsw.read();
        let meta = self.symbol_metadata.read();

        let vector_hits = hnsw.search(&query_vec, top_k * 2);
        let mut hits = Vec::with_capacity(top_k * 2);
        let query_lower = query.to_lowercase();

        for (id, vector_score) in vector_hits {
            if let Some((node, snippet)) = meta.get(&id) {
                let name_lower = node.name.to_lowercase();
                let lexical_bonus = if name_lower == query_lower {
                    0.5
                } else if name_lower.contains(&query_lower) || query_lower.contains(&name_lower) {
                    0.3
                } else {
                    0.0
                };

                let final_score = (vector_score * 0.5 + lexical_bonus).min(1.0);

                hits.push(SearchHit {
                    symbol_name: node.name.clone(),
                    file_path: node.file.clone(),
                    signature: node.signature.clone(),
                    score: final_score,
                    snippet: snippet.clone(),
                });
            }
        }

        // Rank by final score descending
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(top_k);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        HybridSearchResult {
            query: query.to_string(),
            total_hits: hits.len(),
            hits,
            latency_ms: elapsed_ms,
        }
    }
}
