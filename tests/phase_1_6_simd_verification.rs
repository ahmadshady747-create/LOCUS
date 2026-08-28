//! Phase 1.6 Verification: SIMD Vector Acceleration & Zero-Heap Quantized Search.

use locus_engine::search::{
    distance, dot_product_avx2_chunked, dot_product_i8, dot_product_neon_chunked,
    dot_product_scalar, HnswIndex, HnswQueryScratch, HybridMatcher, DEFAULT_DIM,
};
use std::time::Instant;

#[test]
fn test_simd_dot_product_equivalence_scalar_vs_avx2_vs_neon() {
    let lengths = [8, 16, 32, 64, 128, 256, 300];

    for &len in &lengths {
        let a: Vec<i8> = (0..len)
            .map(|i| (((i * 17) % 255) as i32 - 127) as i8)
            .collect();
        let b: Vec<i8> = (0..len)
            .map(|i| (((i * 31) % 255) as i32 - 127) as i8)
            .collect();

        let scalar_res = dot_product_scalar(&a, &b);
        let avx2_res = dot_product_avx2_chunked(&a, &b);
        let neon_res = dot_product_neon_chunked(&a, &b);
        let dynamic_res = dot_product_i8(&a, &b);

        assert_eq!(
            scalar_res, avx2_res,
            "Scalar and AVX2 results diverge at len {}",
            len
        );
        assert_eq!(
            scalar_res, neon_res,
            "Scalar and NEON results diverge at len {}",
            len
        );
        assert_eq!(
            scalar_res, dynamic_res,
            "Scalar and dynamic dispatch results diverge at len {}",
            len
        );
    }
}

#[test]
fn test_simd_dot_product_distance_inversion() {
    let a: Vec<i8> = (0..64)
        .map(|i| (((i * 7) % 200) as i32 - 100) as i8)
        .collect();
    let b: Vec<i8> = (0..64)
        .map(|i| (((i * 13) % 200) as i32 - 100) as i8)
        .collect();

    let dot = dot_product_i8(&a, &b);
    let dist = distance(&a, &b);
    assert_eq!(dist, -dot);
}

#[test]
fn test_zero_heap_embed_text_fixed() {
    let sample = "pub async fn handle_request(req: HttpRequest) -> HttpResponse";
    let dynamic_vec = HybridMatcher::embed_text(sample);
    let fixed_arr = HybridMatcher::embed_text_fixed(sample);

    assert_eq!(dynamic_vec.len(), DEFAULT_DIM);
    assert_eq!(&dynamic_vec[..], &fixed_arr[..]);
}

#[test]
fn test_simd_zero_heap_scratch_search_consistency() {
    let mut index = HnswIndex::new(DEFAULT_DIM, 4, 16);

    for id in 1u64..=50u64 {
        let vec: Vec<i8> = (0..DEFAULT_DIM)
            .map(|i| ((((id as usize + i) * 11) % 255) as i32 - 127) as i8)
            .collect();
        index.insert(id, vec);
    }

    let query: Vec<i8> = (0..DEFAULT_DIM)
        .map(|i| (((i * 19) % 255) as i32 - 127) as i8)
        .collect();

    let standard_results = index.search(&query, 5);

    let mut scratch = HnswQueryScratch::with_capacity(32);
    let mut zero_alloc_results = Vec::with_capacity(5);
    index.search_with_scratch(&query, 5, &mut scratch, &mut zero_alloc_results);

    assert_eq!(standard_results.len(), zero_alloc_results.len());
    for (std_hit, zero_hit) in standard_results.iter().zip(zero_alloc_results.iter()) {
        assert_eq!(std_hit.0, zero_hit.0);
        assert!((std_hit.1 - zero_hit.1).abs() < 1e-6);
    }
}

#[test]
fn test_simd_dot_product_latency_sub_20us() {
    let a: Vec<i8> = (0..64)
        .map(|i| (((i * 23) % 255) as i32 - 127) as i8)
        .collect();
    let b: Vec<i8> = (0..64)
        .map(|i| (((i * 29) % 255) as i32 - 127) as i8)
        .collect();

    let iterations = 20_000;
    let start = Instant::now();
    let mut sink = 0;

    for _ in 0..iterations {
        sink += dot_product_i8(&a, &b);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    assert_ne!(sink, 0);
    assert!(
        avg_us < 20.0,
        "SIMD dot-product latency ({:.4}µs) exceeded 20µs threshold",
        avg_us
    );
}

#[test]
fn test_hnsw_top_k_nearest_neighbor_accuracy() {
    let mut index = HnswIndex::new(DEFAULT_DIM, 4, 16);

    let target_vec: Vec<i8> = (0..DEFAULT_DIM)
        .map(|i| if i % 2 == 0 { 80 } else { -80 })
        .collect();
    index.insert(999, target_vec.clone());

    for id in 1u64..=40u64 {
        let other_vec: Vec<i8> = (0..DEFAULT_DIM)
            .map(|i| if (i + id as usize) % 2 == 0 { -40 } else { 40 })
            .collect();
        index.insert(id, other_vec);
    }

    let hits = index.search(&target_vec, 1);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, 999, "Expected exact match for vector 999");
}
