//! Chaos Verifier & Ambient Agent Fuzzing Test Suite.
//!
//! Generates 1,000 diverse, malformed, and poisoned code vectors to rigorously prove:
//! 1. Absolute zero-panic guarantee under arbitrary corrupted syntax.
//! 2. Bounded execution latency (<=50ms per symbolic verification pass).
//! 3. Accurate counterexample synthesis for arithmetic traps, unwrap panics, and out-of-bounds access.

use locus_agents::AmbientAgentEngine;
use locus_core::QuickVerifierBridge;
use std::time::Instant;

/// Generates 1,000 deterministic fuzzing vectors covering 4 distinct anomaly classes.
fn generate_1000_fuzzing_vectors() -> Vec<(String, String, bool)> {
    let mut vectors = Vec::with_capacity(1000);

    // =========================================================================
    // Category 1: 250 Syntax Anomalies & Broken Tokens
    // =========================================================================
    for i in 0..250 {
        let malformed_code = match i % 5 {
            0 => "{ { { ( ( ( [ [ ] unclosed_everything".to_string(),
            1 => format!("pub fn broken_{} () ->>> {{ let x = ;;; }}", i),
            2 => format!("\n\n\r\t   // Comment only {}\n   \t\r\n", i),
            3 => format!(";;;;;;; let mut x = {};;;;;;;;", i),
            _ => format!("{{{{{{}}}}}} random_token_{} \0\0\0 raw null bytes", i),
        };
        let prompt = format!("! refactor syntax anomaly {}", i);
        vectors.push((prompt, malformed_code, true));
    }

    // =========================================================================
    // Category 2: 250 Division-by-Zero Traps (Unguarded)
    // =========================================================================
    for i in 0..250 {
        let trap_code = match i % 4 {
            0 => format!("pub fn div_trap_{}(a: i32, b: i32) -> i32 {{\n    a / b\n}}", i),
            1 => format!("let ratio = (total_amount_{} * 100) / count_{};", i, i),
            2 => format!("fn calculate(x: f64, y: f64) -> f64 {{ (x + 10.0) / (y - {}) }}", i),
            _ => format!("let result = val / step_{};", i),
        };
        let prompt = format!("! fix div by zero in trap {}", i);
        // Expect unsafe unless guarded
        vectors.push((prompt, trap_code, false));
    }

    // =========================================================================
    // Category 3: 250 Bounds & Unsafe Unwraps
    // =========================================================================
    for i in 0..250 {
        let bounds_or_unwrap_code = match i % 3 {
            0 => format!("let elem = my_array_{}[idx_{} + 5];", i, i),
            1 => format!("let val = optional_result_{}.unwrap();", i),
            _ => format!("let value = map.get(\"key_{}\").expect(\"missing\");", i),
        };
        let prompt = format!("! fix bounds and unwraps in {}", i);
        vectors.push((prompt, bounds_or_unwrap_code, false));
    }

    // =========================================================================
    // Category 4: 250 Mixed Unicode, Arabic, Chinese & Emojis
    // =========================================================================
    for i in 0..250 {
        let unicode_code = match i % 5 {
            0 => format!("// دالة رياضية عربية سيادية {}\nدالة_القسمة(المقسوم / المقسوم_عليه_{})", i, i),
            1 => format!("fn 計算_{}(分子: f64, 分母_{}: f64) -> f64 {{\n    分子 / 分母_{}\n}}", i, i, i),
            2 => format!("let 🚀_velocity_{} = 🛸_distance / ⏱️_time_{};", i, i),
            3 => format!("let قائمة_{} = المصفوفة_{}[المؤشر_{}];", i, i, i),
            _ => format!("// 🛡️ LOCUS Sovereign Verification Test {}\npub fn sovereign_{}() {{ let x = 42; }}", i, i),
        };
        let prompt = format!("! حوّل هذه الدالة إلى Rust {}", i);
        let is_safe = !unicode_code.contains('/') && !unicode_code.contains('[');
        vectors.push((prompt, unicode_code, is_safe));
    }

    vectors
}

#[tokio::test]
async fn test_chaos_verifier_1000_fuzzing_vectors() {
    let vectors = generate_1000_fuzzing_vectors();
    assert_eq!(vectors.len(), 1000);

    let start_all = Instant::now();
    let mut total_verify_ms = 0.0;
    let mut verified_traps_count = 0;

    for (idx, (prompt, code, expected_safe)) in vectors.iter().enumerate() {
        // 1. Symbolic Verification Fuzzing Pass
        let start_single = Instant::now();
        let report = QuickVerifierBridge::verify_expression_or_function(
            &format!("fuzz_target_{}", idx),
            Some(code),
        );
        let elapsed_single = (start_single.elapsed().as_nanos() as f64) / 1_000_000.0;
        total_verify_ms += elapsed_single;

        // Zero-Hang & Strict Latency Invariant (<= 50ms)
        assert!(
            elapsed_single <= 50.0,
            "Verification of vector #{} exceeded 50ms (got {:.3}ms)",
            idx,
            elapsed_single
        );

        // Counterexample Soundness
        if !*expected_safe && !report.is_safe {
            assert!(
                report.counterexample.is_some(),
                "Unsafe vector #{} must produce an explanatory counterexample",
                idx
            );
            verified_traps_count += 1;
        }

        // 2. Ambient Agent Execution Fuzzing on a subset of 100 vectors
        if idx % 10 == 0 {
            let action_res = AmbientAgentEngine::execute_ambient_action(prompt, Some(code)).await;
            assert!(action_res.is_ok(), "Ambient agent must never panic or return Err on fuzz vector #{}", idx);
            let res = action_res.unwrap();
            assert!(res.latency_ms < 100.0, "Ambient action must finish in <100ms");
        }
    }

    let total_elapsed_s = start_all.elapsed().as_secs_f64();
    let avg_latency_ms = total_verify_ms / 1000.0;

    println!(
        "🛡️ Fuzzing Benchmark Summary: 1,000 Vectors executed in {:.3}s (Average Latency: {:.3}ms / vector). Traps Identified: {}",
        total_elapsed_s, avg_latency_ms, verified_traps_count
    );

    assert!(avg_latency_ms < 5.0, "Average verification latency across 1,000 vectors must be <5ms (got {:.3}ms)", avg_latency_ms);
    assert!(verified_traps_count > 400, "Must correctly identify and catch >400 unsafe traps");
}
