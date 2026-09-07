//! SIMD-Accelerated Vector Distance & Dot-Product Kernel Module.
//!
//! Exposes 8-lane chunked math kernels, single-pass fused cosine distance,
//! and bounded Top-K heap selectors in pure safe Rust.

#![forbid(unsafe_code)]

pub mod kernel;
pub mod scanner;
pub mod types;

pub use kernel::SimdMathKernel;
pub use scanner::SimdBatchScanner;
pub use types::{DistanceMetric, ScoredVector, SimdEngineStats, VectorRecord};

// ---------------------------------------------------------------------------
// Unit & Performance Benchmark Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_dot_product_and_cosine_similarity_analytical_precision() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let b = vec![2.0, 0.0, 1.0, 3.0, 0.0, 4.0, 1.0, 2.0, 0.0, 5.0];

        // Analytical dot product: 1*2 + 2*0 + 3*1 + 4*3 + 5*0 + 6*4 + 7*1 + 8*2 + 9*0 + 10*5
        // = 2 + 0 + 3 + 12 + 0 + 24 + 7 + 16 + 0 + 50 = 114.0
        let dot = SimdMathKernel::dot_product(&a, &b);
        assert!((dot - 114.0).abs() < 1e-6, "Dot product must match analytical result (got {})", dot);

        // Cosine similarity between identical vectors must be 1.0
        let self_sim = SimdMathKernel::cosine_similarity(&a, &a);
        assert!((self_sim - 1.0).abs() < 1e-6, "Self cosine similarity must be 1.0 (got {})", self_sim);

        // Orthogonal vectors
        let v1 = vec![1.0, 0.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0, 0.0];
        let ortho = SimdMathKernel::cosine_similarity(&v1, &v2);
        assert!(ortho.abs() < 1e-6, "Orthogonal cosine similarity must be 0.0 (got {})", ortho);
    }

    #[test]
    fn test_numerical_stability_zero_vectors_and_edge_cases() {
        let zero_v = vec![0.0f32; 384];
        let rand_v = vec![0.5f32; 384];

        // Cosine with zero vector must return 0.0 without NaN or panic
        let sim = SimdMathKernel::cosine_similarity(&zero_v, &rand_v);
        assert_eq!(sim, 0.0);
        assert!(!sim.is_nan());

        let sim_zero = SimdMathKernel::cosine_similarity(&zero_v, &zero_v);
        assert_eq!(sim_zero, 0.0);
        assert!(!sim_zero.is_nan());

        // L2 distance to self must be 0.0
        let l2_self = SimdMathKernel::l2_distance_squared(&rand_v, &rand_v);
        assert!(l2_self < 1e-6);
    }

    #[test]
    fn test_batch_scanner_10k_vectors_384_dim_top_10() {
        let dim = 384;
        let count = 10_000;
        let mut records = Vec::with_capacity(count);

        let query = vec![1.0f32; dim];

        for i in 0..count {
            // Generate vectors where vector 9999 has the highest dot product
            let factor = (i as f32) + 1.0;
            let emb = vec![factor; dim];
            records.push(VectorRecord::new(i as u64, i as u64 * 10, emb));
        }

        let (top_10, stats) = SimdBatchScanner::find_top_k(
            &query,
            &records,
            DistanceMetric::DotProduct,
            10,
        );

        assert_eq!(top_10.len(), 10);
        assert_eq!(stats.vectors_scanned, count);
        assert_eq!(stats.dimension, dim);

        // Vector with highest factor should be rank 0
        assert_eq!(top_10[0].vector_id, 9999);
        assert_eq!(top_10[1].vector_id, 9998);

        // Scores must be descending
        for i in 0..9 {
            assert!(
                top_10[i].score >= top_10[i + 1].score,
                "Top-K must be ordered descending by score"
            );
        }
    }

    #[test]
    fn test_sub_25ns_vector_distance_latency_budget() {
        let dim = 384;
        let a = vec![0.33f32; dim];
        let b = vec![0.44f32; dim];

        // Warm up
        for _ in 0..100 {
            let _ = SimdMathKernel::cosine_similarity(&a, &b);
        }

        let iterations = if cfg!(debug_assertions) { 1_000 } else { 100_000 };
        let start = Instant::now();

        let mut acc = 0.0f32;
        for _ in 0..iterations {
            acc += SimdMathKernel::cosine_similarity(&a, &b);
        }

        let elapsed = start.elapsed();
        let avg_ns = (elapsed.as_nanos() as f64) / (iterations as f64);

        println!("⚡ Average 384-dim Cosine Similarity Latency: {:.2} ns (acc: {})", avg_ns, acc);

        let budget_ns = if cfg!(debug_assertions) { 100_000.0 } else { 25.0 };
        assert!(
            avg_ns < budget_ns,
            "Vector distance must execute within budget (<25ns in release, <100µs in debug), got: {:.2} ns",
            avg_ns
        );
    }
}
