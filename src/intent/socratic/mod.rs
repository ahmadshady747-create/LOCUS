//! Socratic Ambiguity Entropy Engine.
//!
//! Evaluates developer intent entropy H(intent) in [0.0, 1.0], detects specification gaps,
//! and deterministically triggers Socratic clarification dialogues.

#![forbid(unsafe_code)]

pub mod contract;
pub mod dialogue;
pub mod edge_case_detector;
pub mod entropy_calculator;
pub mod types;

pub use contract::{
    ContractDelta, ContractError, ContractId, ContractLockState, DynamicAddendumEngine,
    FrozenTransactionState, ResumeReceipt, SpecContract, SpecLockdownEngine,
};
pub use dialogue::{
    ArchitecturalChoice, ArchitecturalPattern, ArchitecturalTasteEngine, IntentResolution,
    SocraticQuestion, SocraticQuestionGenerator, TasteScore, TradeoffMatrix, TradeoffRating,
};
pub use edge_case_detector::EdgeCaseDetector;
pub use entropy_calculator::AmbiguityEntropyCalculator;
pub use types::{
    AmbiguityScore, AmbiguousTarget, EdgeCaseKind, EdgeCaseViolation, EntropyBreakdown,
    EntropyThresholdConfig, IntentEntropyReport,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_low_entropy_for_deterministic_prompt() {
        let prompt = "Implement pub fn parse_header(bytes: &[u8]) -> Result<Header, ParseError> in src/parser/header.rs handling empty bytes and invalid magic numbers";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        assert!(
            report.total_entropy.value() <= 0.15,
            "Specific technical prompt must yield low entropy <= 0.15, got: {}",
            report.total_entropy
        );
        assert!(
            !report.requires_socratic_dialogue,
            "Low-entropy intent must not trigger Socratic dialogue"
        );
        assert!(report.confidence_level >= 0.85);
        assert_eq!(report.underspecified_edge_cases.len(), 0);
    }

    #[test]
    fn test_high_entropy_for_vague_abstract_prompt() {
        let prompt = "fix the bug and make it work fast";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        assert!(
            report.total_entropy.value() >= 0.60,
            "Abstract vague prompt must yield high entropy >= 0.60, got: {}",
            report.total_entropy
        );
        assert!(
            report.requires_socratic_dialogue,
            "High-entropy intent must trigger Socratic dialogue"
        );
        assert!(!report.socratic_clarification_questions.is_empty());
    }

    #[test]
    fn test_edge_case_gap_detection_in_intent() {
        let prompt = "Fetch user profile from remote API and get item from array by index";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        assert!(!report.underspecified_edge_cases.is_empty());
        let kinds: Vec<EdgeCaseKind> = report
            .underspecified_edge_cases
            .iter()
            .map(|v| v.kind)
            .collect();
        assert!(kinds.contains(&EdgeCaseKind::MissingErrorHandling));
        assert!(kinds.contains(&EdgeCaseKind::NullabilityBoundaryGap));
    }

    #[test]
    fn test_entropy_evaluation_sub_millisecond_latency() {
        let prompt = "Create struct TokenBucket in src/ratelimit.rs with capacity: u32 and rate: f64. Implement acquire(&mut self) -> bool with mutex concurrency guards";
        
        // Warm up regex LazyLock caches
        let _ = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        let start = Instant::now();
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);
        let elapsed = start.elapsed();

        let limit_micros = if cfg!(debug_assertions) { 50_000 } else { 500 };
        assert!(
            elapsed.as_micros() < limit_micros,
            "Warmed entropy evaluation must complete within budget, took: {:?}",
            elapsed
        );
        assert!(report.latency_ms < 50.0);
    }

    #[test]
    fn test_threshold_config_behavior() {
        let config = EntropyThresholdConfig::strict();
        let score_ok = AmbiguityScore::new(0.05);
        let score_mid = AmbiguityScore::new(0.25);
        let score_high = AmbiguityScore::new(0.60);

        assert!(score_ok.is_acceptable(&config));
        assert!(score_mid.requires_dialogue(&config));
        assert!(score_high.is_critical(&config));
    }
}
