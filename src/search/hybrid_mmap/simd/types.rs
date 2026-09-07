//! Domain types and distance metrics for the SIMD-Accelerated Vector Engine.
//!
//! Provides metric definitions, scored vector models, and performance telemetry in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

// ---------------------------------------------------------------------------
// Distance Metrics
// ---------------------------------------------------------------------------

/// Supported vector distance and similarity metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// Cosine Similarity $\cos(\theta) = \frac{\mathbf{u} \cdot \mathbf{v}}{\|\mathbf{u}\|_2 \|\mathbf{v}\|_2} \in [-1.0, 1.0]$.
    CosineSimilarity,
    /// Inner / Dot Product $\mathbf{u} \cdot \mathbf{v} = \sum_i u_i v_i$.
    DotProduct,
    /// Squared Euclidean Distance $\|\mathbf{u} - \mathbf{v}\|_2^2 = \sum_i (u_i - v_i)^2$.
    EuclideanSquared,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        Self::CosineSimilarity
    }
}

impl fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CosineSimilarity => write!(f, "COSINE_SIMILARITY"),
            Self::DotProduct => write!(f, "DOT_PRODUCT"),
            Self::EuclideanSquared => write!(f, "EUCLIDEAN_SQUARED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Vector Records & Scored Results
// ---------------------------------------------------------------------------

/// Storage record representing an indexed vector embedding in memory or MMAP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorRecord {
    pub vector_id: u64,
    pub symbol_id: u64,
    pub embedding: Vec<f32>,
}

impl VectorRecord {
    pub fn new(vector_id: u64, symbol_id: u64, embedding: Vec<f32>) -> Self {
        Self {
            vector_id,
            symbol_id,
            embedding,
        }
    }
}

/// A candidate vector result paired with its computed similarity or distance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredVector {
    pub vector_id: u64,
    pub symbol_id: u64,
    pub score: f32,
}

impl ScoredVector {
    pub fn new(vector_id: u64, symbol_id: u64, score: f32) -> Self {
        Self {
            vector_id,
            symbol_id,
            score,
        }
    }
}

// Min-heap ordering for CosineSimilarity and DotProduct (smallest score is at top of heap to be evicted)
impl Eq for ScoredVector {}

impl PartialOrd for ScoredVector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredVector {
    fn cmp(&self, other: &Self) -> Ordering {
        // Safe total ordering on floats without panicking on NaN
        match self.score.partial_cmp(&other.score) {
            Some(ord) => ord,
            None => {
                if self.score.is_nan() && other.score.is_nan() {
                    Ordering::Equal
                } else if self.score.is_nan() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Engine Telemetry Stats
// ---------------------------------------------------------------------------

/// Execution telemetry and throughput statistics for the SIMD batch scanner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimdEngineStats {
    pub vectors_scanned: usize,
    pub dimension: usize,
    pub total_time_ns: u64,
    pub throughput_vectors_per_sec: f64,
}

impl SimdEngineStats {
    pub fn new(vectors_scanned: usize, dimension: usize, total_time_ns: u64) -> Self {
        let secs = (total_time_ns as f64) / 1_000_000_000.0;
        let throughput = if secs > 0.0 {
            (vectors_scanned as f64) / secs
        } else {
            0.0
        };

        Self {
            vectors_scanned,
            dimension,
            total_time_ns,
            throughput_vectors_per_sec: throughput,
        }
    }
}
