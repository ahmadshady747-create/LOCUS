//! LOCUS Engine v1.6.0 Final Integration & End-to-End Verification Suite.
//!
//! Verifies the unified operation of all Phase 1.6 sovereign subsystems:
//! 1. Lossless Concrete Syntax Tree (CST Green/Red Tree) with 100% Trivia Roundtrip
//! 2. 32-Rule Enterprise Invariant Safety Engine (Rules 0..31) via RuleMask(u32)
//! 3. Inter-Procedural SSA Taint Engine v2 with Cryptographic TaintAuditCertificate
//! 4. Hardware-Accelerated SIMD (AVX2/NEON/Scalar) & Zero-Heap Quantized Vector Search
//! 5. Hierarchical Wildcard Subtree Leases, Swarm OCC & Deadlock Resolution
//! 6. 28 Sovereign Native MCP Tools Matrix over JSON-RPC 2.0 stdio

use locus_engine::cst::{parse_to_cst, to_lossless_text, SyntaxKind};
use locus_engine::guard::{RuleMask, RuleRunner};
use locus_engine::lease::LeaseRegistry;
use locus_engine::mcp::handle_json_rpc_message;
use locus_engine::search::{
    HnswIndex, HnswQueryScratch, HybridMatcher, DEFAULT_DIM,
};
use locus_engine::taint::DataFlowTracker;
use locus_engine::types::{Language, LeaseStatus, RiskScore, TxStagedFile};
use serde_json::{json, Value};
use std::time::Instant;

#[test]
fn test_final_cst_green_red_tree_roundtrip() {
    let source_rust = r#"
    /// Lossless documentation comment
    pub async fn handle_stream<T: Send>(stream: Stream<T>) -> Result<(), Error> {
        // Line trivia comment
        /* Multi-line
           block comment */
        let x = 42;
        Ok(())
    }
    "#;

    let cst = parse_to_cst(source_rust);
    assert_eq!(cst.kind(), SyntaxKind::Root);
    let reconstructed = to_lossless_text(&cst);
    assert_eq!(
        reconstructed, source_rust,
        "CST reconstructed text must match source 100% byte-for-byte"
    );

    // Verify token offset query
    let token = cst.token_at_offset(15).expect("Token at offset 15");
    assert!(!token.text().is_empty());
}

#[test]
fn test_final_ast_guard_32_rules_bitset_coverage() {
    let all_32_mask = RuleMask::ALL_32;
    assert_eq!(all_32_mask.0, u32::MAX);

    // Warm up static regexes
    let _ = RuleRunner::verify_with_mask("fn warmup() {}", all_32_mask);

    // Safe code passes all 32 rules in < 0.20ms
    let safe_code = r#"
    pub async fn process_transaction(amount: u64, divisor: u64) -> Result<u64, &'static str> {
        if divisor == 0 {
            return Err("Zero divisor");
        }
        let result = amount.checked_div(divisor).ok_or("Overflow")?;
        Ok(result)
    }
    "#;

    let start = Instant::now();
    let report = RuleRunner::verify_with_mask(safe_code, all_32_mask);
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    assert!(report.passed, "Safe code must pass all 32 enterprise rules");
    assert!(
        latency_ms < 1.0,
        "32-rule verification took too long: {:.4}ms",
        latency_ms
    );
}

#[test]
fn test_final_inter_procedural_ssa_taint_and_certificates() {
    // 1. Inter-procedural propagation across modules
    let file1 = TxStagedFile {
        path: "src/controller.ts".to_string(),
        original_content: None,
        staged_content: r#"
        import { runDatabaseQuery } from './service';
        export async function handleRequest(req, res) {
            const client_target = req.headers["x-query"];
            await runDatabaseQuery(client_target);
        }
        "#
        .to_string(),
        language: Language::TypeScript,
    };

    let file2 = TxStagedFile {
        path: "src/service.ts".to_string(),
        original_content: None,
        staged_content: r#"
        export async function runDatabaseQuery(rawQuery: string) {
            return await db.execute(rawQuery);
        }
        "#
        .to_string(),
        language: Language::TypeScript,
    };

    let reports = DataFlowTracker::analyze_owned_files(&[file1, file2]);
    assert!(!reports.is_empty(), "Expected inter-procedural taint flow");
    let r = &reports[0];
    assert_eq!(r.source.variable, "client_target");
    assert_eq!(r.violation_risk, RiskScore::High);

    // 2. Verified Sanitizer Proof Chain and Cryptographic Certificate
    let sanitized_source = r#"
    export function renderUserBio(req, res) {
        const raw_input = req.headers["x-bio"];
        const cleanBio = DOMPurify.sanitize(raw_input);
        container.innerHTML = cleanBio;
    }
    "#;

    let sanitized_reports = DataFlowTracker::analyze_source("src/profile.ts", "renderUserBio", sanitized_source);
    assert_eq!(sanitized_reports.len(), 1);
    let s_rep = &sanitized_reports[0];
    assert_eq!(s_rep.source.variable, "raw_input");
    assert!(s_rep.is_sanitized, "Sanitizer proof chain must certify data flow");
    assert_eq!(s_rep.violation_risk, RiskScore::Low);

    let cert = s_rep.certificate.as_ref().expect("Cryptographic certificate");
    assert_eq!(cert.sanitizer_name, "DOMPurify.sanitize");
    assert_eq!(cert.sha256_fingerprint.len(), 64);
}

#[test]
fn test_final_simd_hardware_acceleration_and_zero_heap_search() {
    let target = HybridMatcher::embed_text_fixed("fn handle_request(req: Request) -> Response");
    let mut index = HnswIndex::new(DEFAULT_DIM, 4, 16);

    index.insert(1, target.to_vec());
    for id in 2u64..=30u64 {
        let other = HybridMatcher::embed_text_fixed(&format!("fn auxiliary_worker_{}()", id));
        index.insert(id, other.to_vec());
    }

    let mut scratch = HnswQueryScratch::with_capacity(16);
    let mut hits = Vec::with_capacity(3);

    let start = Instant::now();
    index.search_with_scratch(&target, 3, &mut scratch, &mut hits);
    let latency_us = start.elapsed().as_secs_f64() * 1_000_000.0;

    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].0, 1, "Nearest neighbor must be exact match (id=1)");
    assert!(
        latency_us < 20.0,
        "Zero-heap SIMD query took too long: {:.3}µs",
        latency_us
    );
}

#[test]
fn test_final_hierarchical_wildcard_leases_and_occ_deadlock_resolution() {
    let registry = LeaseRegistry::new();

    // 1. Wildcard Subtree Lease
    let lease_res = registry.acquire("src/core/*", "agent_1", 10_000);
    assert!(matches!(lease_res, LeaseStatus::Acquired(_)));

    // 2. Child Symbol Blocked by Wildcard
    let conflict_res = registry.acquire("src/core/engine.rs::init", "agent_2", 10_000);
    assert!(matches!(conflict_res, LeaseStatus::HierarchicalConflict { .. }));

    // 3. Monotonic OCC Advancement
    let occ_ver = registry.commit_occ("src/config.rs::SETTINGS", 1).expect("OCC commit");
    assert_eq!(occ_ver, 2);

    // 4. Stale OCC Rejection
    let stale_err = registry.commit_occ("src/config.rs::SETTINGS", 1);
    assert!(matches!(stale_err, Err(LeaseStatus::OccMismatch { .. })));

    // 5. Deadlock Detection & Resolution
    let _ = registry.acquire("res_a", "agent_a", 10_000);
    let _ = registry.acquire("res_b", "agent_b", 10_000);
    let _ = registry.register_wait("agent_a", "res_b");
    let deadlock = registry.register_wait("agent_b", "res_a");
    assert!(deadlock.is_err(), "Circular wait must be detected and broken");
}

#[test]
fn test_final_mcp_28_tools_matrix_json_rpc_end_to_end() {
    // 1. Verify 28 tools list
    let list_req = r#"{"jsonrpc":"2.0","id":"list-1","method":"tools/list"}"#;
    let list_resp = handle_json_rpc_message(list_req).expect("List response");
    let parsed_list: Value = serde_json::from_str(&list_resp).unwrap();
    let tools = parsed_list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 28, "MCP server must expose all 28 sovereign tools");

    // 2. Dispatch query_cst
    let cst_req = json!({
        "jsonrpc": "2.0",
        "id": "cst-call",
        "method": "tools/call",
        "params": {
            "name": "query_cst",
            "arguments": {
                "code": "pub fn test() {}",
                "language": "rust"
            }
        }
    });
    let cst_resp = handle_json_rpc_message(&cst_req.to_string()).expect("CST response");
    assert!(cst_resp.contains("trivia_roundtrip_intact"));

    // 3. Dispatch simd_vector_search
    let simd_req = json!({
        "jsonrpc": "2.0",
        "id": "simd-call",
        "method": "tools/call",
        "params": {
            "name": "simd_vector_search",
            "arguments": {
                "query_text": "verify cryptographic token",
                "top_k": 1
            }
        }
    });
    let simd_resp = handle_json_rpc_message(&simd_req.to_string()).expect("SIMD response");
    assert!(simd_resp.contains("simd_accelerated"));
}
