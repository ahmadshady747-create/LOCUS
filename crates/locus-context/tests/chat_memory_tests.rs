//! Integration test suite for Deep Chat Memory Indexer (ChatMemoryIndex).

use locus_context::ChatMemoryIndex;
use std::time::Instant;

#[test]
fn test_chat_memory_indexing_and_retrieval() {
    let mut index = ChatMemoryIndex::new();

    let transcript = r#"
User: كيف نقوم بفحص تسريب الذاكرة في Rust؟

Assistant: يمكننا استخدام Memory Profiler أو كتابة دالة اختبار مخصصة عبر alloc track.
DECISION: اعتمدنا استخدام Arc<RwLock<AmbientState>> لمنع التعليق وتتبع الذاكرة لحظياً.

User: أريد دالة التحقق الشكلي ثنائي الاتجاه.

Assistant: ها هي الدالة:
pub fn verify_bidirectional_contract(code: &str) -> bool {
    let forward_ok = check_forward_safety(code);
    let backward_ok = check_weakest_precondition(code);
    forward_ok && backward_ok
}
"#;

    let chunks_indexed = index.index_session("session_101", transcript);
    assert!(chunks_indexed >= 3, "Expected at least 3 chat chunks indexed");

    // 1. Search in Arabic
    let ar_results = index.search("تتبع الذاكرة", 5);
    assert!(!ar_results.is_empty(), "Expected match for Arabic memory tracking query");
    assert!(ar_results[0].snippet.contains("AmbientState") || ar_results[0].snippet.contains("الذاكرة"));

    // 2. Search in English
    let en_results = index.search("weakest precondition", 5);
    assert!(!en_results.is_empty(), "Expected match for English wp query");
    assert!(en_results[0].snippet.contains("check_weakest_precondition") || en_results[0].snippet.contains("verify_bidirectional_contract"));
}

#[test]
fn test_large_session_indexing_and_sub_10ms_search() {
    let mut index = ChatMemoryIndex::new();

    // Generate large session with > 10,000 words
    for i in 0..200 {
        let block = format!(
            "User: What about architectural decision number {}?\n\nAssistant: For step {}, we configured module locus_{} with parameter bound {}.\nDECISION: Architectural milestone {} approved with zero panic.",
            i, i, i, i * 10, i
        );
        index.index_session(&format!("session_{}", i), &block);
    }

    let start = Instant::now();
    for i in 0..50 {
        let res = index.search(&format!("parameter bound {}", i * 10), 5);
        assert!(!res.is_empty());
    }
    let elapsed_ms = start.elapsed().as_millis() as f64;
    let avg_ms = elapsed_ms / 50.0;
    assert!(avg_ms < 10.0, "ChatMemory search must average sub-10ms (got {:.2}ms)", avg_ms);
}
