//! Socratic Intent Dialogue & Architectural Taste Engine Module.
//!
//! Converts high-entropy intent evaluations into structured, constrained Socratic questions,
//! evaluates solutions against deterministic architectural taste metrics, and produces
//! structured contract hints for autonomous compilation.

#![forbid(unsafe_code)]

pub mod question_generator;
pub mod taste_engine;
pub mod types;

pub use question_generator::SocraticQuestionGenerator;
pub use taste_engine::ArchitecturalTasteEngine;
pub use types::{
    ArchitecturalChoice, ArchitecturalPattern, IntentResolution, SocraticQuestion, TasteScore,
    TradeoffMatrix, TradeoffRating,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::socratic::entropy_calculator::AmbiguityEntropyCalculator;
    use std::time::Instant;

    #[test]
    fn test_taste_engine_scoring_formula_and_clean_arch_preference() {
        let engine = ArchitecturalTasteEngine::new();

        let clean_tradeoffs = ArchitecturalTasteEngine::create_tradeoff_matrix(
            8, 8, 10, 8, "Clean Architecture with maximum modularity",
        );
        let clean_score = engine.evaluate_taste(ArchitecturalPattern::CleanArchitecture, &clean_tradeoffs, true);

        assert!(
            clean_score.value() >= 0.80,
            "Clean Architecture with high modularity must score >= 0.80, got: {}",
            clean_score
        );
        assert!(clean_score.is_high_taste());

        let dod_tradeoffs = ArchitecturalTasteEngine::create_tradeoff_matrix(
            10, 10, 7, 6, "Zero-alloc data-oriented design",
        );
        let dod_score = engine.evaluate_taste(ArchitecturalPattern::DataOriented, &dod_tradeoffs, true);

        assert!(
            dod_score.value() >= 0.75,
            "Zero-alloc DOD must score high taste >= 0.75, got: {}",
            dod_score
        );
    }

    #[test]
    fn test_question_generator_from_entropy_report() {
        let prompt = "build a user session cache in memory";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        let generator = SocraticQuestionGenerator::new();
        let questions = generator.generate_dialogue(&report);

        assert!(
            !questions.is_empty(),
            "High-entropy data prompt must produce Socratic questions"
        );

        let q = &questions[0];
        assert_eq!(q.choices.len(), 3, "Each Socratic question must contain exactly 3 choices");

        let rec_count = q.choices.iter().filter(|c| c.is_recommended).count();
        assert_eq!(rec_count, 1, "Exactly 1 choice must be designated as recommended");

        let rec_choice = q.recommended_choice().expect("Recommended choice must exist");
        assert!(
            rec_choice.title.starts_with("(Recommended"),
            "Recommended choice title must have '(Recommended' prefix, got: {}",
            rec_choice.title
        );
        assert!(rec_choice.taste_score.value() >= 0.70);
    }

    #[test]
    fn test_tradeoff_matrix_validation_and_ratings() {
        let matrix = ArchitecturalTasteEngine::create_tradeoff_matrix(
            9, 8, 10, 7, "High modularity and strong execution speed",
        );

        assert_eq!(matrix.execution_speed.value(), 9);
        assert_eq!(matrix.memory_efficiency.value(), 8);
        assert_eq!(matrix.modularity.value(), 10);
        assert_eq!(matrix.development_velocity.value(), 7);
        assert!(!matrix.summary.is_empty());
    }

    #[test]
    fn test_intent_resolution_and_contract_hints() {
        let generator = SocraticQuestionGenerator::new();
        let prompt = "handle user payments with external gateway";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);
        let questions = generator.generate_dialogue(&report);

        assert!(!questions.is_empty());
        let q = &questions[0];
        let rec_id = &q.recommended_choice_id;

        let resolution = generator
            .resolve_choice(q, rec_id)
            .expect("Resolution for recommended choice must succeed");

        assert_eq!(resolution.question_id, q.id);
        assert_eq!(resolution.selected_choice_id, *rec_id);
        assert!(!resolution.synthesized_contract_hints.is_empty());
        assert!(!resolution.enforced_invariants.is_empty());
        assert!(resolution.final_taste_score.value() > 0.0);
    }

    #[test]
    fn test_sub_0_05ms_dialogue_generation_budget() {
        let generator = SocraticQuestionGenerator::new();
        let prompt = "Store telemetry logs and process stream";
        let report = AmbiguityEntropyCalculator::evaluate(prompt, None, None, None);

        // Warmup
        let _ = generator.generate_dialogue(&report);

        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = generator.generate_dialogue(&report);
        }
        let elapsed = start.elapsed();
        let avg_micros = (elapsed.as_micros() as f64) / (iterations as f64);

        println!("⚡ Socratic dialogue generation average latency: {:.3} µs", avg_micros);
        let limit = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
        assert!(
            avg_micros < limit,
            "Dialogue generation must complete in sub-0.05ms (<50µs in release), got: {:.3} µs",
            avg_micros
        );
    }
}
