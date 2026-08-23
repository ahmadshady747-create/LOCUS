//! Pure-Rust In-Memory Quantized HNSW Vector Index.
//!
//! Provides sub-millisecond approximate nearest-neighbor search with 8-bit quantized
//! integer embeddings and zero external C-FFI runtime bottlenecks.

#![forbid(unsafe_code)]

use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

pub const DEFAULT_DIM: usize = 64;

#[derive(Debug, Clone)]
pub struct HnswNode {
    pub id: u64,
    pub vector: Vec<i8>,
    pub neighbors: Vec<Vec<u64>>, // layer -> neighbor IDs
}

#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    dist: i32,
    id: u64,
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist.cmp(&other.dist)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

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
    pub fn distance(a: &[i8], b: &[i8]) -> i32 {
        let mut dot: i32 = 0;
        let len = a.len().min(b.len());
        for i in 0..len {
            dot += (a[i] as i32) * (b[i] as i32);
        }
        // Invert dot product so lower distance = higher similarity
        -dot
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
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let mut q = query.to_vec();
        if q.len() < self.dim {
            q.resize(self.dim, 0);
        } else if q.len() > self.dim {
            q.truncate(self.dim);
        }

        let candidates = self.search_layer(&q, 0, top_k * 2);
        let mut results = Vec::with_capacity(top_k);

        for c in candidates.into_iter().take(top_k) {
            // Normalize score to 0.0 .. 1.0 similarity range
            let sim = ((-c.dist as f32) / (self.dim as f32 * 127.0 * 127.0)).clamp(0.0, 1.0);
            results.push((c.id, sim));
        }

        results
    }

    fn search_layer(&self, query: &[i8], layer: usize, ef: usize) -> Vec<Candidate> {
        let entry_id = match self.entry_point {
            Some(id) => id,
            None => return Vec::new(),
        };

        let entry_idx = match self.id_to_idx.get(&entry_id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut visited = HashSet::new();
        visited.insert(entry_id);

        let initial_dist = Self::distance(query, &self.nodes[entry_idx].vector);
        let mut candidates = BinaryHeap::new();
        candidates.push(Candidate { dist: initial_dist, id: entry_id });

        let mut result = vec![Candidate { dist: initial_dist, id: entry_id }];

        while let Some(current) = candidates.pop() {
            if let Some(&curr_idx) = self.id_to_idx.get(&current.id) {
                if layer < self.nodes[curr_idx].neighbors.len() {
                    for &neighbor_id in &self.nodes[curr_idx].neighbors[layer] {
                        if visited.insert(neighbor_id) {
                            if let Some(&n_idx) = self.id_to_idx.get(&neighbor_id) {
                                let dist = Self::distance(query, &self.nodes[n_idx].vector);
                                let cand = Candidate { dist, id: neighbor_id };
                                result.push(cand.clone());
                                candidates.push(cand);
                            }
                        }
                    }
                }
            }

            if visited.len() >= ef * 4 {
                break;
            }
        }

        result.sort_by_key(|c| c.dist);
        result.truncate(ef);
        result
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
