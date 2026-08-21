//! Comprehensive Standard Benchmark & Stress Suite for locus-engine.

use std::time::Instant;
use locus_engine::mcp::handle_json_rpc_message;
use locus_engine::{
    AstContextCache, AstDiffEngine, AstGuard, Language, SymbolGraph, ViolationKind,
};

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
    let failing_div = "pub fn div(a: f64, b: f64) -> f64 { a / b }";
    let failing_mutex = "use std::sync::Mutex;\nasync fn run(m: &Mutex<i32>) { let _g = m.lock().unwrap(); some_fut().await; }";
    let failing_unwrap = "fn get(m: &std::collections::HashMap<&str, &str>) -> &str { m.get(\"k\").unwrap() }";
    let failing_redos = "const R: &str = \"(a+)+$\";";
    let failing_unbalanced = "fn broken( { let x = 1;";

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
    assert!(avg_us < 200.0, "AstGuard verification took too long: {:.2}µs", avg_us);
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
    assert!(avg_us < 200.0, "SHA-256 caching took too long: {:.2}µs", avg_us);
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
        let patched = AstDiffEngine::patch(rust_source, "initialize_engine", replacement, Language::Rust)
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
    assert!(avg_us < 2000.0, "Diff engine took too long: {:.2}µs", avg_us);
}

#[test]
fn bench_polyglot_symbol_graph() {
    let mut graph = SymbolGraph::new();

    let rust_code = "pub fn handle_req() {}\npub struct Session {}\npub enum State {}";
    let ts_code = "export function parseInput() {}\nexport interface Config {}\nexport class Router {}";
    let py_code = "def process_data():\n    pass\nclass DataModel:\n    pass\n";

    let start = Instant::now();
    let iterations = 200;

    for i in 0..iterations {
        graph.index_file_content(&format!("src/mod_{}.rs", i), rust_code, Language::Rust);
        graph.index_file_content(&format!("src/client_{}.ts", i), ts_code, Language::TypeScript);
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
    assert!(avg_us < 2000.0, "MCP dispatch took too long: {:.2}µs", avg_us);
}
