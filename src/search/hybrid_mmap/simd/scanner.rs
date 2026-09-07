//! High-Throughput SIMD Batch Vector Scanner & Bounded Top-K Heap Selector.
//!
//! Scans contiguous arrays of vector embeddings in memory, calculates similarities via `SimdMathKernel`,
//! and maintains a bounded Top-K priority queue with zero allocations on the inner loop.

#![forbid(unsafe_code)]

use super::kernel::SimdMathKernel;
use super::types::{DistanceMetric, ScoredVector, SimdEngineStats, VectorRecord};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Instant;

/// Batch scanner executing vectorized nearest-neighbor candidate selection.
pub struct SimdBatchScanner;

impl SimdBatchScanner {
    /// Scans an array of `VectorRecord` items and extracts the top-$K$ best matches.
    pub fn find_top_k(
        query_embedding: &[f32],
        records: &[VectorRecord],
        metric: DistanceMetric,
        k: usize,
    ) -> (Vec<ScoredVector>, SimdEngineStats) {
        let start = Instant::now();

        if k == 0 || records.is_empty() || query_embedding.is_empty() {
            let stats = SimdEngineStats::new(records.len(), query_embedding.len(), 0);
            return (Vec::new(), stats);
        }

        let results = match metric {
            DistanceMetric::CosineSimilarity | DistanceMetric::DotProduct => {
                Self::scan_max_scores(query_embedding, records, metric, k)
            }
            DistanceMetric::EuclideanSquared => {
                Self::scan_min_distances(query_embedding, records, k)
            }
        };

        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let stats = SimdEngineStats::new(records.len(), query_embedding.len(), elapsed_ns);

        (results, stats)
    }

    /// Handles similarity metrics (Cosine, DotProduct) where HIGHER scores are better.
    /// Uses a Min-Heap of size K to retain the largest K scores.
    fn scan_max_scores(
        query: &[f32],
        records: &[VectorRecord],
        metric: DistanceMetric,
        k: usize,
    ) -> Vec<ScoredVector> {
        let mut min_heap: BinaryHeap<Reverse<ScoredVector>> = BinaryHeap::with_capacity(k);

        for record in records {
            let score = match metric {
                DistanceMetric::CosineSimilarity => {
                    SimdMathKernel::cosine_similarity(query, &record.embedding)
                }
                DistanceMetric::DotProduct => {
                    SimdMathKernel::dot_product(query, &record.embedding)
                }
                DistanceMetric::EuclideanSquared => unreachable!(),
            };

            let candidate = ScoredVector::new(record.vector_id, record.symbol_id, score);

            if min_heap.len() < k {
                min_heap.push(Reverse(candidate));
            } else if let Some(worst) = min_heap.peek() {
                if candidate.score > worst.0.score {
                    min_heap.pop();
                    min_heap.push(Reverse(candidate));
                }
            }
        }

        let mut out: Vec<ScoredVector> = min_heap.into_iter().map(|r| r.0).collect();
        // Sort descending (highest score first)
        out.sort_by(|a, b| b.cmp(a));
        out
    }

    /// Handles distance metrics (Euclidean) where LOWER distances are better.
    /// Uses a Max-Heap of size K to retain the smallest K distances.
    fn scan_min_distances(
        query: &[f32],
        records: &[VectorRecord],
        k: usize,
    ) -> Vec<ScoredVector> {
        let mut max_heap: BinaryHeap<ScoredVector> = BinaryHeap::with_capacity(k);

        for record in records {
            let dist = SimdMathKernel::l2_distance_squared(query, &record.embedding);
            let candidate = ScoredVector::new(record.vector_id, record.symbol_id, dist);

            if max_heap.len() < k {
                max_heap.push(candidate);
            } else if let Some(worst) = max_heap.peek() {
                if candidate.score < worst.score {
                    max_heap.pop();
                    max_heap.push(candidate);
                }
            }
        }

        let mut out: Vec<ScoredVector> = max_heap.into_iter().collect();
        // Sort ascending (lowest distance first)
        out.sort_by(|a, b| a.cmp(b));
        out
    }
}
