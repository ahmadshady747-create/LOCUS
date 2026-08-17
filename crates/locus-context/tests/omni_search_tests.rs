//! Integration test suite for OmniSearch Engine.

use locus_context::OmniSearchEngine;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_omni_search_path_and_code_matching() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let root = temp_dir.path();

    // Create mock workspace directory structure
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");

    let main_rs = src_dir.join("main.rs");
    let mut file1 = File::create(&main_rs).expect("Failed to create main.rs");
    writeln!(file1, "fn main() {{\n    println!(\"LOCUS AI Engine Active\");\n}}").unwrap();

    let utils_rs = src_dir.join("utils.rs");
    let mut file2 = File::create(&utils_rs).expect("Failed to create utils.rs");
    writeln!(file2, "pub fn calculate_tokens_saved() -> u64 {{\n    42\n}}").unwrap();

    // 1. Exact / Fuzzy Filename search
    let file_results = OmniSearchEngine::search_local("main.rs", root, 10);
    assert!(!file_results.is_empty());
    assert_eq!(file_results[0].title, "main.rs");
    assert_eq!(file_results[0].category, "File");

    // 2. Code Content Search
    let code_results = OmniSearchEngine::search_local("calculate_tokens", root, 10);
    assert!(!code_results.is_empty());
    let code_hit = code_results.iter().find(|r| r.category == "Code");
    assert!(code_hit.is_some(), "Expected code snippet match");
    assert!(code_hit.unwrap().subtitle.contains("calculate_tokens_saved"));
}

#[test]
fn test_omni_search_sub_10ms_latency() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let root = temp_dir.path();

    // Create 30 mock files
    for i in 0..30 {
        let f_path = root.join(format!("file_{}.rs", i));
        let mut f = File::create(&f_path).unwrap();
        writeln!(f, "pub fn func_{}() -> i32 {{ {} }}", i, i * 2).unwrap();
    }

    let start = Instant::now();
    for i in 0..20 {
        let _ = OmniSearchEngine::search_local(&format!("func_{}", i), root, 10);
    }
    let elapsed = start.elapsed().as_millis();
    let avg_ms = elapsed as f64 / 20.0;
    assert!(avg_ms < 10.0, "OmniSearch must average sub-10ms per query (got {:.2}ms)", avg_ms);
}
