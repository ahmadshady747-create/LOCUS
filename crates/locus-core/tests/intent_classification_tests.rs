//! Integration test suite for Zero-Lag Intent Classifier (OmniIntent).

use locus_core::OmniIntent;
use std::time::Instant;

#[test]
fn test_intent_prefix_dispatch() {
    // 1. Web Search
    assert_eq!(
        OmniIntent::parse("? rust tokio docs", None),
        OmniIntent::WebSearch { query: "rust tokio docs".to_string() }
    );
    assert_eq!(
        OmniIntent::parse("/w react hooks best practices", None),
        OmniIntent::WebSearch { query: "react hooks best practices".to_string() }
    );

    // 2. Terminal Command
    assert_eq!(
        OmniIntent::parse("> cargo build --release", None),
        OmniIntent::TerminalCommand { command: "cargo build --release".to_string() }
    );

    // 3. Chat & Memory
    assert_eq!(
        OmniIntent::parse("@chat what did we discuss about auth?", None),
        OmniIntent::ChatMemory { description: "what did we discuss about auth?".to_string() }
    );
    assert_eq!(
        OmniIntent::parse("@memory database schema decisions", None),
        OmniIntent::ChatMemory { description: "database schema decisions".to_string() }
    );

    // 4. Formal Verification
    assert_eq!(
        OmniIntent::parse("#verify crates/locus-core/src/contracts.rs", None),
        OmniIntent::FormalVerify { target: "crates/locus-core/src/contracts.rs".to_string() }
    );

    // 5. Agent Action via '!'
    assert_eq!(
        OmniIntent::parse("! convert this python loop to rust", Some("for x in arr: pass".to_string())),
        OmniIntent::AgentAction {
            prompt: "convert this python loop to rust".to_string(),
            target_code: Some("for x in arr: pass".to_string()),
        }
    );
}

#[test]
fn test_intent_natural_language_action_verbs() {
    // English Verbs
    assert_eq!(
        OmniIntent::parse("fix syntax error in main.rs", None),
        OmniIntent::AgentAction {
            prompt: "fix syntax error in main.rs".to_string(),
            target_code: None,
        }
    );
    assert_eq!(
        OmniIntent::parse("refactor authentication handler", None),
        OmniIntent::AgentAction {
            prompt: "refactor authentication handler".to_string(),
            target_code: None,
        }
    );
    assert_eq!(
        OmniIntent::parse("explain how Weakest Precondition works", None),
        OmniIntent::AgentAction {
            prompt: "explain how Weakest Precondition works".to_string(),
            target_code: None,
        }
    );

    // Arabic Verbs
    assert_eq!(
        OmniIntent::parse("أصلح خطأ القسمة على صفر", None),
        OmniIntent::AgentAction {
            prompt: "أصلح خطأ القسمة على صفر".to_string(),
            target_code: None,
        }
    );
    assert_eq!(
        OmniIntent::parse("حوّل هذه الدالة إلى Rust", Some("def add(a, b): return a + b".to_string())),
        OmniIntent::AgentAction {
            prompt: "حوّل هذه الدالة إلى Rust".to_string(),
            target_code: Some("def add(a, b): return a + b".to_string()),
        }
    );
    assert_eq!(
        OmniIntent::parse("اشرح بنية المجدول", None),
        OmniIntent::AgentAction {
            prompt: "اشرح بنية المجدول".to_string(),
            target_code: None,
        }
    );
}

#[test]
fn test_intent_default_local_search() {
    assert_eq!(
        OmniIntent::parse("App.tsx", None),
        OmniIntent::LocalSearch { query: "App.tsx".to_string() }
    );
    assert_eq!(
        OmniIntent::parse("crates/locus-core", None),
        OmniIntent::LocalSearch { query: "crates/locus-core".to_string() }
    );
    assert_eq!(
        OmniIntent::parse("   ", None),
        OmniIntent::LocalSearch { query: "".to_string() }
    );
}

#[test]
fn test_intent_sub_millisecond_latency() {
    let start = Instant::now();
    for _ in 0..1_000 {
        let _ = OmniIntent::parse("! convert this loop into iterator pattern", Some("let mut v = vec![];".to_string()));
        let _ = OmniIntent::parse("? rust async closures", None);
        let _ = OmniIntent::parse("أصلح كود الـ Verifier", None);
    }
    let elapsed_micros = start.elapsed().as_micros();
    let per_call_micros = elapsed_micros as f64 / 3_000.0;
    assert!(
        per_call_micros < 500.0,
        "Intent classification must be sub-0.5ms per call (got {:.3}µs)",
        per_call_micros
    );
}
