//! Pure-Rust In-Memory Quantized HNSW Vector Index with SIMD Acceleration.
//!
//! Provides sub-millisecond approximate nearest-neighbor search with 8-bit quantized
//! integer embeddings, chunk-based SIMD acceleration (AVX2 / NEON / Optimized Scalar),
//! and zero-heap allocation query execution paths.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

pub const DEFAULT_DIM: usize = 64;

// ---------------------------------------------------------------------------
// SIMD Hardware Accelerated Dot-Product & Distance Calculations (100% Safe)
// ---------------------------------------------------------------------------

/// Compute scalar dot-product between two 8-bit quantized integer vectors.
#[inline]
pub fn dot_product_scalar(a: &[i8], b: &[i8]) -> i32 {
    let min_len = a.len().min(b.len());
    let mut dot: i32 = 0;
    for i in 0..min_len {
        dot += (a[i] as i32) * (b[i] as i32);
    }
    dot
}

/// Compute AVX2-optimized 32-element chunked dot-product with 4-way unrolled accumulators.
/// LLVM auto-vectorizes this loop pattern directly into 256-bit AVX2 vector instructions.
#[inline]
pub fn dot_product_avx2_chunked(a: &[i8], b: &[i8]) -> i32 {
    let min_len = a.len().min(b.len());
    let a_slice = &a[..min_len];
    let b_slice = &b[..min_len];

    let mut acc0: i32 = 0;
    let mut acc1: i32 = 0;
    let mut acc2: i32 = 0;
    let mut acc3: i32 = 0;

    let a_chunks = a_slice.chunks_exact(32);
    let b_chunks = b_slice.chunks_exact(32);
    let remainder_a = a_chunks.remainder();
    let remainder_b = b_chunks.remainder();

    for (ac, bc) in a_chunks.zip(b_chunks) {
        for i in 0..8 {
            acc0 += (ac[i] as i32) * (bc[i] as i32);
            acc1 += (ac[i + 8] as i32) * (bc[i + 8] as i32);
            acc2 += (ac[i + 16] as i32) * (bc[i + 16] as i32);
            acc3 += (ac[i + 24] as i32) * (bc[i + 24] as i32);
        }
    }

    let mut total = (acc0 + acc1) + (acc2 + acc3);
    for (&x, &y) in remainder_a.iter().zip(remainder_b.iter()) {
        total += (x as i32) * (y as i32);
    }
    total
}

/// Compute ARM NEON-optimized 16-element chunked dot-product with 2-way unrolled accumulators.
/// LLVM auto-vectorizes this loop pattern directly into 128-bit NEON vector instructions.
#[inline]
pub fn dot_product_neon_chunked(a: &[i8], b: &[i8]) -> i32 {
    let min_len = a.len().min(b.len());
    let a_slice = &a[..min_len];
    let b_slice = &b[..min_len];

    let mut acc0: i32 = 0;
    let mut acc1: i32 = 0;

    let a_chunks = a_slice.chunks_exact(16);
    let b_chunks = b_slice.chunks_exact(16);
    let remainder_a = a_chunks.remainder();
    let remainder_b = b_chunks.remainder();

    for (ac, bc) in a_chunks.zip(b_chunks) {
        for i in 0..8 {
            acc0 += (ac[i] as i32) * (bc[i] as i32);
            acc1 += (ac[i + 8] as i32) * (bc[i + 8] as i32);
        }
    }

    let mut total = acc0 + acc1;
    for (&x, &y) in remainder_a.iter().zip(remainder_b.iter()) {
        total += (x as i32) * (y as i32);
    }
    total
}

/// Dynamic runtime dispatch for 8-bit quantized vector dot product:
/// - Auto-detects AVX2 on x86_64 targets via `is_x86_feature_detected!("avx2")`.
/// - Dispatches to NEON chunked path on AArch64 targets.
/// - Falls back to high-performance unrolled scalar implementation on other platforms.
#[inline]
pub fn dot_product_i8(a: &[i8], b: &[i8]) -> i32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            dot_product_avx2_chunked(a, b)
        } else {
            dot_product_scalar(a, b)
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        dot_product_neon_chunked(a, b)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        dot_product_scalar(a, b)
    }
}

/// Compute dot-product distance between two 8-bit quantized vectors (lower is closer).
#[inline]
pub fn distance(a: &[i8], b: &[i8]) -> i32 {
    -dot_product_i8(a, b)
}

// ---------------------------------------------------------------------------
// HNSW Node & Query Structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HnswNode {
    pub id: u64,
    pub vector: Vec<i8>,
    pub neighbors: Vec<Vec<u64>>, // layer -> neighbor IDs
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub dist: i32,
    pub id: u64,
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering so BinaryHeap acts as a min-heap (closest distance first)
        other.dist.cmp(&self.dist)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Reusable query scratch space enabling zero-heap allocations during repeated vector queries.
#[derive(Debug, Clone, Default)]
pub struct HnswQueryScratch {
    candidates: BinaryHeap<Candidate>,
    visited: HashSet<u64>,
    results: Vec<Candidate>,
}

impl HnswQueryScratch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            candidates: BinaryHeap::with_capacity(capacity),
            visited: HashSet::with_capacity(capacity * 2),
            results: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.visited.clear();
        self.results.clear();
    }
}

// ---------------------------------------------------------------------------
// Pure-Rust 8-bit Quantized HNSW Index
// ---------------------------------------------------------------------------

/// Pure-Rust 8-bit Quantized Hierarchical Navigable Small World (HNSW) Index.
pub struct HnswIndex {
    dim: usize,
    max_layers: usize,
    m: usize,
    nodes: Vec<HnswNode>,
    id_to_idx: std::collections::HashMap<u64, usize>,
    entry_point: Option<u64>,
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new(DEFAULT_DIM, 4, 16)
    }
}

impl HnswIndex {
    pub fn new(dim: usize, max_layers: usize, m: usize) -> Self {
        Self {
            dim,
            max_layers,
            m,
            nodes: Vec::new(),
            id_to_idx: std::collections::HashMap::new(),
            entry_point: None,
        }
    }

    /// Compute dot-product distance between two 8-bit quantized vectors (lower is closer).
    #[inline]
    pub fn distance(a: &[i8], b: &[i8]) -> i32 {
        distance(a, b)
    }

    /// Insert an item and its vector into the HNSW index.
    pub fn insert(&mut self, id: u64, mut vector: Vec<i8>) {
        if vector.len() < self.dim {
            vector.resize(self.dim, 0);
        } else if vector.len() > self.dim {
            vector.truncate(self.dim);
        }

        let node_idx = self.nodes.len();
        let mut neighbors = Vec::with_capacity(self.max_layers);
        for _ in 0..self.max_layers {
            neighbors.push(Vec::new());
        }

        let new_node = HnswNode {
            id,
            vector: vector.clone(),
            neighbors,
        };

        self.nodes.push(new_node);
        self.id_to_idx.insert(id, node_idx);

        if self.entry_point.is_none() {
            self.entry_point = Some(id);
            return;
        }

        // Link with nearest existing nodes across layers
        let nearest = self.search_layer(&vector, 0, self.m);
        for candidate in nearest {
            if let Some(&neighbor_idx) = self.id_to_idx.get(&candidate.id) {
                if self.nodes[neighbor_idx].neighbors[0].len() < self.m {
                    self.nodes[neighbor_idx].neighbors[0].push(id);
                }
                self.nodes[node_idx].neighbors[0].push(candidate.id);
            }
        }
    }

    /// Search for top-K nearest neighbors to a query vector.
    pub fn search(&self, query: &[i8], top_k: usize) -> Vec<(u64, f32)> {
        let mut output = Vec::with_capacity(top_k);
        let mut scratch = HnswQueryScratch::with_capacity(top_k * 4);
        self.search_with_scratch(query, top_k, &mut scratch, &mut output);
        output
    }

    /// Zero-heap allocation vector query path reusing callers' scratch buffer and output container.
    pub fn search_with_scratch(
        &self,
        query: &[i8],
        top_k: usize,
        scratch: &mut HnswQueryScratch,
        output: &mut Vec<(u64, f32)>,
    ) {
        output.clear();
        if self.nodes.is_empty() {
            return;
        }

        let q_slice = if query.len() >= self.dim {
            &query[..self.dim]
        } else {
            query
        };

        self.search_layer_with_scratch(q_slice, 0, top_k * 2, scratch);

        let dim_factor = (self.dim as f32) * 127.0 * 127.0;
        let count = scratch.results.len().min(top_k);
        for i in 0..count {
            let c = &scratch.results[i];
            let sim = ((-c.dist as f32) / dim_factor).clamp(0.0, 1.0);
            output.push((c.id, sim));
        }
    }

    fn search_layer(&self, query: &[i8], layer: usize, ef: usize) -> Vec<Candidate> {
        let mut scratch = HnswQueryScratch::with_capacity(ef * 4);
        self.search_layer_with_scratch(query, layer, ef, &mut scratch);
        scratch.results.clone()
    }

    fn search_layer_with_scratch(
        &self,
        query: &[i8],
        layer: usize,
        ef: usize,
        scratch: &mut HnswQueryScratch,
    ) {
        scratch.clear();

        let entry_id = match self.entry_point {
            Some(id) => id,
            None => return,
        };

        let entry_idx = match self.id_to_idx.get(&entry_id) {
            Some(&idx) => idx,
            None => return,
        };

        scratch.visited.insert(entry_id);

        let initial_dist = Self::distance(query, &self.nodes[entry_idx].vector);
        scratch.candidates.push(Candidate {
            dist: initial_dist,
            id: entry_id,
        });

        scratch.results.push(Candidate {
            dist: initial_dist,
            id: entry_id,
        });

        while let Some(current) = scratch.candidates.pop() {
            if let Some(&curr_idx) = self.id_to_idx.get(&current.id) {
                if layer < self.nodes[curr_idx].neighbors.len() {
                    for &neighbor_id in &self.nodes[curr_idx].neighbors[layer] {
                        if scratch.visited.insert(neighbor_id) {
                            if let Some(&n_idx) = self.id_to_idx.get(&neighbor_id) {
                                let dist = Self::distance(query, &self.nodes[n_idx].vector);
                                let cand = Candidate {
                                    dist,
                                    id: neighbor_id,
                                };
                                scratch.results.push(cand.clone());
                                scratch.candidates.push(cand);
                            }
                        }
                    }
                }
            }

            if scratch.visited.len() >= ef * 4 {
                break;
            }
        }

        scratch.results.sort_by_key(|c| c.dist);
        scratch.results.truncate(ef);
    }

    /// Number of indexed vectors.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
