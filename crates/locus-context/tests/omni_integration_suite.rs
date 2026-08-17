//! End-to-End Integration & Stress Test for OmniSearch and Chat Memory Indexer.
//!
//! Validates concurrent retrieval performance across files, code, and chat transcripts.

use locus_context::{ChatMemoryIndex, OmniSearchEngine};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_omni_and_chat_memory_concurrent_stress_e2e() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let root = temp_dir.path().to_path_buf();

    // Create 30 mock files with varying extensions and contents
    for i in 0..30 {
        let file_path = root.join(format!("module_{}.rs", i));
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            "pub fn execute_operation_{}() -> usize {{\n    let value = {};\n    value * 2\n}}",
            i, i
        )
        .unwrap();
    }

    // Initialize Chat Memory Index with 50 conversational turns
    let mut chat_index = ChatMemoryIndex::new();
    for i in 0..50 {
        let transcript = format!(
            "User: How is state managed in module {}?\n\nAssistant: Module {} uses lock-free atomics with zero overhead.\nDECISION: Architectural decision {} approved.",
            i, i, i
        );
        chat_index.index_session(&format!("session_{}", i), &transcript);
    }

    let shared_chat_index = Arc::new(chat_index);
    let mut handles = Vec::new();

    let start_total = Instant::now();

    // Spawn 4 concurrent threads executing mixed searches
    for t_idx in 0..4 {
        let root_clone = root.clone();
        let chat_clone = Arc::clone(&shared_chat_index);

        handles.push(thread::spawn(move || {
            let mut total_omni_time_ms = 0.0;
            let mut total_mem_time_ms = 0.0;
            let count = 15;

            for i in 0..count {
                let query = format!("execute_operation_{}", (t_idx * 5 + i) % 30);

                // 1. OmniSearch
                let start_omni = Instant::now();
                let results = OmniSearchEngine::search_local(&query, &root_clone, 5);
                let omni_latency = (start_omni.elapsed().as_nanos() as f64) / 1_000_000.0;
                total_omni_time_ms += omni_latency;
                assert!(!results.is_empty(), "OmniSearch must find matching code for {}", query);

                // 2. Chat Memory
                let start_mem = Instant::now();
                let mem_results = chat_clone.search("lock-free atomics", 3);
                let mem_latency = (start_mem.elapsed().as_nanos() as f64) / 1_000_000.0;
                total_mem_time_ms += mem_latency;
                assert!(!mem_results.is_empty(), "Chat memory must find decision");
            }

            let avg_omni = total_omni_time_ms / (count as f64);
            let avg_mem = total_mem_time_ms / (count as f64);
            assert!(avg_omni < 25.0, "Average OmniSearch latency under concurrent load must be <25ms (got {:.2}ms)", avg_omni);
            assert!(avg_mem < 10.0, "Average ChatMemory latency must be <10ms (got {:.2}ms)", avg_mem);
        }));
    }

    for h in handles {
        h.join().expect("Concurrent search thread failed");
    }

    let total_elapsed = start_total.elapsed().as_millis();
    assert!(total_elapsed > 0);
}
