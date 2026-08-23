//! Comprehensive Verification Suite for LOCUS Phase 1.5-B.
//!
//! Verifies:
//! 1. Multi-Agent Symbol Leases & Concurrency Conflict Broker
//! 2. Cross-File Taint & Data-Flow Analysis
//! 3. Static Option / Null Propagation Tracker
//! 4. In-Memory Quantized HNSW Vector Index & Hybrid Matcher (<1ms latency)
//! 5. WebAssembly (WASM) Bridge Interface

use std::time::Instant;
use locus_engine::{
    DataFlowTracker, HnswIndex, HybridMatcher, LeaseRegistry, LeaseStatus,
    LocusWasmBridge, NullPropagationTracker, SymbolGraph, Language,
};

#[test]
fn test_multi_agent_symbol_leases_and_conflict_resolution() {
    let registry = LeaseRegistry::new();
    let fqn_login = "src/auth.rs::login";
    let fqn_logout = "src/auth.rs::logout";

    // 1. Agent A acquires lease on login
    let status_a = registry.acquire(fqn_login, "agent_alpha", 5000);
    let lease_a = match status_a {
        LeaseStatus::Acquired(l) => l,
        _ => panic!("Expected lease to be acquired by agent_alpha"),
    };
    assert_eq!(lease_a.fqn, fqn_login);
    assert_eq!(lease_a.holder_agent_id, "agent_alpha");

    // 2. Agent B attempts to acquire lease on same symbol -> Conflict
    let status_b = registry.acquire(fqn_login, "agent_beta", 5000);
    match status_b {
        LeaseStatus::Conflict { fqn, current_holder, remaining_ttl_ms } => {
            assert_eq!(fqn, fqn_login);
            assert_eq!(current_holder, "agent_alpha");
            assert!(remaining_ttl_ms > 0);
        }
        _ => panic!("Expected conflict when agent_beta acquires already leased symbol"),
    }

    // 3. Agent B acquires lease on distinct symbol -> Parallel success
    let status_b_logout = registry.acquire(fqn_logout, "agent_beta", 5000);
    assert!(matches!(status_b_logout, LeaseStatus::Acquired(_)));

    // 4. Agent A renews heartbeat
    let renew_status = registry.renew(&lease_a.lease_id, "agent_alpha", 10000);
    assert!(matches!(renew_status, LeaseStatus::Renewed(_)));

    // 5. Agent A releases lease
    let release_status = registry.release(&lease_a.lease_id, "agent_alpha");
    assert_eq!(release_status, LeaseStatus::Released);

    // 6. Agent B can now acquire login lease
    let status_b_retry = registry.acquire(fqn_login, "agent_beta", 5000);
    assert!(matches!(status_b_retry, LeaseStatus::Acquired(_)));
}

#[test]
fn test_cross_file_taint_data_flow_tracking() {
    let source = r#"
        import fs from 'fs';

        export function handleUpload(req, res) {
            const userInput = req.params.file_path;
            const data = fs.readFileSync(userInput);
            return data;
        }
    "#;

    let reports = DataFlowTracker::analyze_source("src/upload.ts", "handleUpload", source);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert_eq!(r.source.variable, "userInput");
    assert!(!r.sinks.is_empty());
    assert!(r.sinks[0].operation.contains("fs.readFileSync"));
    assert_eq!(r.violation_risk, locus_engine::RiskScore::High);
}

#[test]
fn test_static_null_option_propagation_tracking() {
    let source = r#"
        pub fn find_user(id: u64) -> Option<User> {
            None
        }

        pub fn process() {
            let u = find_user(42);
            println!("{}", u.name);
        }
    "#;

    let reports = NullPropagationTracker::scan_nullable_flows("src/service.rs", source);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].source.symbol, "find_user");
    assert_eq!(reports[0].violation_risk, locus_engine::RiskScore::Medium);
}

#[test]
fn test_in_memory_quantized_hnsw_vector_index() {
    let mut hnsw = HnswIndex::default();

    // Insert 500 vectors with distinct embeddings
    for i in 0..500 {
        let mut vec = vec![0i8; 64];
        vec[i % 64] = 100;
        vec[(i + 1) % 64] = 50;
        hnsw.insert(i as u64, vec);
    }

    assert_eq!(hnsw.len(), 500);

    // Query nearest vector
    let mut query = vec![0i8; 64];
    query[10] = 100;
    query[11] = 50;

    let start = Instant::now();
    let hits = hnsw.search(&query, 5);
    let elapsed_us = start.elapsed().as_nanos() as f64 / 1000.0;

    assert!(!hits.is_empty());
    assert_eq!(hits[0].0, 10, "Nearest neighbor ID should match vector dimension assignment");
    println!("HNSW 500-node Vector Search Latency: {:.2}µs", elapsed_us);
    assert!(elapsed_us < 1000.0, "Vector search must be sub-millisecond (< 1ms)");
}

#[test]
fn test_hybrid_ast_lexical_dense_matcher() {
    let mut graph = SymbolGraph::new();
    let code = r#"
        pub fn authenticate_jwt_user(token: &str) -> bool {
            !token.is_empty()
        }

        pub fn process_stripe_payment(amount: u64) {
            println!("paid {}", amount);
        }
    "#;

    graph.index_file_content("src/auth.rs", code, Language::Rust);

    let matcher = HybridMatcher::new();
    matcher.index_graph(&graph);

    let start = Instant::now();
    let res = matcher.search("authenticate jwt", 3);
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    assert!(!res.hits.is_empty());
    assert_eq!(res.hits[0].symbol_name, "authenticate_jwt_user");
    println!("Hybrid Search Retrieval Latency: {:.3}ms", elapsed_ms);
    assert!(elapsed_ms < 1.0, "Hybrid retrieval must be < 1ms");
}

#[test]
fn test_wasm_bridge_interface() {
    // 1. Verify Code via WASM Bridge
    let safe_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let report_json = LocusWasmBridge::verify_code(safe_code);
    assert!(report_json.contains(r#""passed":true"#));

    // 2. Skeletonize via WASM Bridge
    let tsx_code = r#"
        import React from 'react';
        export function Card({ title }: { title: string }) {
            const [open, setOpen] = useState(false);
            return <div className="card">{title}</div>;
        }
    "#;
    let skeleton = LocusWasmBridge::skeletonize(tsx_code, "tsx");
    assert!(skeleton.contains("export function Card"));

    // 3. Auto-Remediation via WASM Bridge
    let broken_jsx = "<div><p>Hello WASM";
    let fix_json = LocusWasmBridge::auto_remediate(broken_jsx);
    assert!(fix_json.contains("</p>"));
    assert!(fix_json.contains("</div>"));

    // 4. MCP Message Processing in Memory
    let ping_req = r#"{"jsonrpc":"2.0","id":"wasm-1","method":"ping"}"#;
    let resp = LocusWasmBridge::process_mcp_message(ping_req);
    assert!(resp.contains("wasm-1"));
}
