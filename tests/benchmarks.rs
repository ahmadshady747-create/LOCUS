//! Comprehensive Standard Benchmark & Stress Suite for locus-engine.

use locus_engine::cst::parse_to_cst;
use locus_engine::guard::{RuleMask, RuleRunner};
use locus_engine::lease::LeaseRegistry;
use locus_engine::mcp::handle_json_rpc_message;
use locus_engine::search::{dot_product_i8, DEFAULT_DIM};
use locus_engine::taint::DataFlowTracker;
use locus_engine::types::{Language, TxStagedFile};
use locus_engine::{
    AstContextCache, AstDiffEngine, AstGuard, SymbolGraph, ViolationKind,
};
use std::time::Instant;

#[test]
fn bench_ast_guard_1000_cycles() {
    let clean_code = r#"
pub fn compute_sum(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: f64 = values.iter().sum();
    let count = values.len();
    if count != 0 {
        sum / (count as f64)
    } else {
        0.0
    }
}
"#;
    let failing_div = concat!("pub fn div(a: f64, b: f64) -> f64 { ", "a / b", " }");
    let failing_mutex = concat!(
        "use std::sync::Mutex;\nasync fn run(m: &Mutex<i32>) { let _g = m.lock().unwrap(); ",
        "some_fut().await;",
        " }"
    );
    let failing_unwrap = concat!(
        "fn get(m: &std::collections::HashMap<&str, &str>) -> &str { ",
        "m.get(\"k\").unwrap()",
        " }"
    );
    let failing_redos = concat!("const R: &str = \"", "(a+)+$", "\";");
    let failing_unbalanced = concat!("fn broken", "( { let x = 1;");

    let start = Instant::now();
    let iterations = 1000;

    for i in 0..iterations {
        match i % 6 {
            0 => {
                let rep = AstGuard::verify(clean_code);
                assert!(rep.passed);
            }
            1 => {
                let rep = AstGuard::verify(failing_div);
                assert_eq!(rep.violation, Some(ViolationKind::DivisionByZero));
            }
            2 => {
                let rep = AstGuard::verify(failing_mutex);
                assert_eq!(rep.violation, Some(ViolationKind::AsyncMutexAcrossAwait));
            }
            3 => {
                let rep = AstGuard::verify(failing_unwrap);
                assert_eq!(rep.violation, Some(ViolationKind::UnsafeUnwrap));
            }
            4 => {
                let rep = AstGuard::verify(failing_redos);
                assert_eq!(rep.violation, Some(ViolationKind::ReDoSPattern));
            }
            5 => {
                let rep = AstGuard::verify(failing_unbalanced);
                assert_eq!(rep.violation, Some(ViolationKind::UnbalancedDelimiters));
            }
            _ => unreachable!(),
        }
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "\n🛡️  AstGuard Benchmark: {} iterations executed in {:.3}ms (Average: {:.2}µs / verification)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 200.0,
        "AstGuard verification took too long: {:.2}µs",
        avg_us
    );
}

#[test]
fn bench_sha256_cache_1000_cycles() {
    let cache = AstContextCache::new(500);
    let sample = "pub struct TelemetryPacket { pub id: u64, pub payload: Vec<u8> }";

    let start = Instant::now();
    let iterations = 1000;

    for i in 0..iterations {
        let code = format!("{} // cycle {}", sample, i);
        let hash = cache.insert(&code, "pub struct TelemetryPacket;".to_string(), 1);
        assert_eq!(hash.len(), 64);
        let hit = cache.get(&code);
        assert!(hit.is_some());
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "⚡ AstContextCache Benchmark: {} insertions/lookups in {:.3}ms (Average: {:.2}µs / digest+LRU)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 200.0,
        "SHA-256 caching took too long: {:.2}µs",
        avg_us
    );
}

#[test]
fn bench_diff_engine_patch_and_skeleton() {
    let rust_source = r#"
pub struct EngineConfig {
    pub max_threads: usize,
    pub enable_cache: bool,
}

pub fn initialize_engine(config: EngineConfig) -> bool {
    println!("Init");
    true
}

pub fn teardown_engine() {
    println!("Teardown");
}
"#;

    let start = Instant::now();
    let iterations = 500;

    for _ in 0..iterations {
        // Skeletonize
        let skeleton = AstDiffEngine::skeletonize(rust_source, Language::Rust);
        assert!(skeleton.contains("pub struct EngineConfig"));
        assert!(skeleton.contains("pub fn initialize_engine(config: EngineConfig) -> bool;"));

        // Patch
        let replacement = "pub fn initialize_engine(config: EngineConfig) -> bool {\n    true\n}";
        let patched = AstDiffEngine::patch(
            rust_source,
            "initialize_engine",
            replacement,
            Language::Rust,
        )
        .expect("Patch failed");
        assert!(patched.contains("pub fn initialize_engine(config: EngineConfig) -> bool {"));
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "✂️  AstDiffEngine Benchmark: {} skeleton+patch cycles in {:.3}ms (Average: {:.2}µs / operation)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 2000.0,
        "Diff engine took too long: {:.2}µs",
        avg_us
    );
}

#[test]
fn bench_polyglot_symbol_graph() {
    let mut graph = SymbolGraph::new();

    let rust_code = "pub fn handle_req() {}\npub struct Session {}\npub enum State {}";
    let ts_code =
        "export function parseInput() {}\nexport interface Config {}\nexport class Router {}";
    let py_code = "def process_data():\n    pass\nclass DataModel:\n    pass\n";

    let start = Instant::now();
    let iterations = 200;

    for i in 0..iterations {
        graph.index_file_content(&format!("src/mod_{}.rs", i), rust_code, Language::Rust);
        graph.index_file_content(
            &format!("src/client_{}.ts", i),
            ts_code,
            Language::TypeScript,
        );
        graph.index_file_content(&format!("src/worker_{}.py", i), py_code, Language::Python);
    }

    let elapsed = start.elapsed();
    let total_symbols = graph.nodes.len();
    let total_files = graph.file_to_symbols.len();

    println!(
        "🧠 SymbolGraph Benchmark: Indexed {} files ({} AST symbols) across Rust, TS, Python in {:.3}ms",
        total_files,
        total_symbols,
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(total_files, 600);
    assert_eq!(total_symbols, 1600);
}

#[test]
fn bench_mcp_json_rpc_dispatch() {
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"check_safety","arguments":{"code":"pub fn safe() -> i32 { 10 }"}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"skeletonize","arguments":{"code":"pub fn f() {}","language":"rust"}}}"#,
    ];

    let start = Instant::now();
    let iterations = 1000;

    for i in 0..iterations {
        let req = requests[i % requests.len()];
        let resp = handle_json_rpc_message(req);
        assert!(resp.is_some());
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "🔌 MCP Stdio JSON-RPC Benchmark: {} dispatches in {:.3}ms (Average: {:.2}µs / dispatch)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 2000.0,
        "MCP dispatch took too long: {:.2}µs",
        avg_us
    );
}

#[test]
fn bench_cst_green_red_tree_parsing_latency() {
    let code = r#"
    /// Lossless function
    pub async fn process_packet(header: Header, payload: &[u8]) -> Result<Response, Error> {
        // Compute checksum
        let checksum = fnv1a_64(payload);
        Ok(Response::new(checksum))
    }
    "#;

    let start = Instant::now();
    let iterations = 5000;

    for _ in 0..iterations {
        let root = parse_to_cst(code);
        assert_eq!(root.kind(), locus_engine::cst::SyntaxKind::Root);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "🌲 Green/Red Tree CST Benchmark: {} parses in {:.3}ms (Average: {:.3}µs / parse)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 100.0,
        "CST parsing latency ({:.3}µs) exceeded threshold",
        avg_us
    );
}

#[test]
fn bench_guard_32_rules_verification_latency() {
    let code = r#"
    pub async fn secure_handler(req: Request) -> Response {
        let auth = req.headers().get("authorization");
        if auth.is_none() {
            return Response::unauthorized();
        }
        Response::ok()
    }
    "#;

    let mask = RuleMask::ALL_32;
    let start = Instant::now();
    let iterations = 2000;

    for _ in 0..iterations {
        let rep = RuleRunner::verify_with_mask(code, mask);
        assert!(rep.passed);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "🛡️  AstGuard 32-Rule Invariants Benchmark: {} scans in {:.3}ms (Average: {:.3}µs / 32-pass scan)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 200.0,
        "32-rule verification ({:.3}µs) exceeded 200µs threshold",
        avg_us
    );
}

#[test]
fn bench_simd_dot_product_latency() {
    let a: Vec<i8> = (0..DEFAULT_DIM).map(|i| (i as i8).wrapping_mul(7)).collect();
    let b: Vec<i8> = (0..DEFAULT_DIM).map(|i| (i as i8).wrapping_mul(13)).collect();

    let start = Instant::now();
    let iterations = 50_000;
    let mut sink = 0;

    for _ in 0..iterations {
        sink += dot_product_i8(&a, &b);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "⚡ SIMD 64-Dim Dot Product Benchmark: {} operations in {:.3}ms (Average: {:.4}µs / dot-product)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert_ne!(sink, 0);
    assert!(
        avg_us < 0.05,
        "SIMD dot-product latency ({:.4}µs) exceeded 0.05µs",
        avg_us
    );
}

#[test]
fn bench_subtree_lease_and_occ_latency() {
    let registry = LeaseRegistry::new();
    let fqn = "crate::distributed::worker";

    let start = Instant::now();
    let iterations = 10_000;

    for i in 1u64..=iterations {
        let _ = registry.acquire("crate::distributed/*", "agent_bench", 10_000);
        let _ = registry.commit_occ(fqn, i);
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "🐝 Subtree Lease & OCC Benchmark: {} iterations in {:.3}ms (Average: {:.3}µs / operation)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 5.0,
        "Lease & OCC latency ({:.3}µs) exceeded threshold",
        avg_us
    );
}

#[test]
fn bench_inter_procedural_taint_and_certificate_latency() {
    let file = TxStagedFile {
        path: "src/handler.ts".to_string(),
        original_content: None,
        staged_content: r#"
        export async function handle(req, res) {
            const input = req.headers["x-input"];
            const clean = DOMPurify.sanitize(input);
            await db.execute(clean);
        }
        "#
        .to_string(),
        language: Language::TypeScript,
    };

    let start = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        let reports = DataFlowTracker::analyze_owned_files(std::slice::from_ref(&file));
        assert!(!reports.is_empty());
    }

    let elapsed = start.elapsed();
    let avg_us = (elapsed.as_secs_f64() * 1_000_000.0) / (iterations as f64);

    println!(
        "🌊 Inter-Procedural Taint & Certificate Benchmark: {} scans in {:.3}ms (Average: {:.3}µs / scan)",
        iterations,
        elapsed.as_secs_f64() * 1000.0,
        avg_us
    );
    assert!(
        avg_us < 400.0,
        "Taint & Certificate generation ({:.3}µs) exceeded 400µs",
        avg_us
    );
}
