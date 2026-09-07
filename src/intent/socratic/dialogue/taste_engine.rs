//! Architectural Taste Engine.
//!
//! Deterministically evaluates and scores software architectures against LOCUS engineering standards:
//!
//! $$T = 0.35 \cdot S_{\text{modularity}} + 0.25 \cdot S_{\text{zero\_alloc}} + 0.20 \cdot S_{\text{testability}} + 0.20 \cdot S_{\text{safety}}$$
//!
//! Strongly favors Clean Architecture, Hexagonal / Ports & Adapters, and Zero-Alloc DOD
//! while rewarding solutions that minimize downstream blast radius.

#![forbid(unsafe_code)]

use super::types::{
    ArchitecturalChoice, ArchitecturalPattern, TasteScore, TradeoffMatrix, TradeoffRating,
};

/// High-throughput deterministic Architectural Taste Engine.
#[derive(Debug, Default, Clone)]
pub struct ArchitecturalTasteEngine;

impl ArchitecturalTasteEngine {
    pub const WEIGHT_MODULARITY: f32 = 0.35;
    pub const WEIGHT_ZERO_ALLOC: f32 = 0.25;
    pub const WEIGHT_TESTABILITY: f32 = 0.20;
    pub const WEIGHT_SAFETY: f32 = 0.20;

    pub fn new() -> Self {
        Self
    }

    /// Evaluates the architectural taste score $T \in [0.0, 1.0]$ for a specific pattern and tradeoff profile.
    pub fn evaluate_taste(
        &self,
        pattern: ArchitecturalPattern,
        tradeoffs: &TradeoffMatrix,
        blast_radius_is_localized: bool,
    ) -> TasteScore {
        // 1. Compute modularity score S_modularity in [0.0, 1.0]
        let base_modularity = (tradeoffs.modularity.value() as f32) / 10.0;
        let pattern_modularity_boost = match pattern {
            ArchitecturalPattern::CleanArchitecture => 0.15,
            ArchitecturalPattern::Hexagonal => 0.15,
            ArchitecturalPattern::MicroKernel => 0.10,
            ArchitecturalPattern::FunctionalPipeline => 0.08,
            ArchitecturalPattern::ActorMesh => 0.05,
            ArchitecturalPattern::DataOriented => 0.02,
        };
        let s_modularity = (base_modularity + pattern_modularity_boost).clamp(0.0, 1.0);

        // 2. Compute zero-alloc / memory efficiency score S_zero_alloc in [0.0, 1.0]
        let base_zero_alloc = (tradeoffs.memory_efficiency.value() as f32) / 10.0;
        let pattern_zero_alloc_boost = match pattern {
            ArchitecturalPattern::DataOriented => 0.20,
            ArchitecturalPattern::FunctionalPipeline => 0.10,
            ArchitecturalPattern::MicroKernel => 0.05,
            ArchitecturalPattern::CleanArchitecture => 0.02,
            ArchitecturalPattern::Hexagonal => 0.02,
            ArchitecturalPattern::ActorMesh => 0.0,
        };
        let s_zero_alloc = (base_zero_alloc + pattern_zero_alloc_boost).clamp(0.0, 1.0);

        // 3. Compute testability & isolation score S_testability in [0.0, 1.0]
        let base_testability = (tradeoffs.modularity.value() as f32 * 0.6
            + tradeoffs.development_velocity.value() as f32 * 0.4)
            / 10.0;
        let pattern_testability_boost = match pattern {
            ArchitecturalPattern::CleanArchitecture => 0.15,
            ArchitecturalPattern::Hexagonal => 0.15,
            ArchitecturalPattern::FunctionalPipeline => 0.12,
            ArchitecturalPattern::MicroKernel => 0.08,
            ArchitecturalPattern::DataOriented => 0.05,
            ArchitecturalPattern::ActorMesh => 0.04,
        };
        let s_testability = (base_testability + pattern_testability_boost).clamp(0.0, 1.0);

        // 4. Compute safety & blast-radius score S_safety in [0.0, 1.0]
        let base_speed = (tradeoffs.execution_speed.value() as f32) / 10.0;
        let blast_bonus = if blast_radius_is_localized { 0.15 } else { 0.0 };
        let pattern_safety_boost = match pattern {
            ArchitecturalPattern::FunctionalPipeline => 0.15,
            ArchitecturalPattern::CleanArchitecture => 0.12,
            ArchitecturalPattern::Hexagonal => 0.12,
            ArchitecturalPattern::ActorMesh => 0.10,
            ArchitecturalPattern::MicroKernel => 0.08,
            ArchitecturalPattern::DataOriented => 0.06,
        };
        let s_safety = (0.5 * base_speed + pattern_safety_boost + blast_bonus).clamp(0.0, 1.0);

        // 5. Compound formula calculation
        let raw_taste = Self::WEIGHT_MODULARITY * s_modularity
            + Self::WEIGHT_ZERO_ALLOC * s_zero_alloc
            + Self::WEIGHT_TESTABILITY * s_testability
            + Self::WEIGHT_SAFETY * s_safety;

        TasteScore::new(raw_taste)
    }

    /// Ranks a collection of candidate architectural choices and designates the highest scoring as recommended.
    pub fn rank_and_designate_recommendation(
        &self,
        choices: &mut [ArchitecturalChoice],
    ) {
        if choices.is_empty() {
            return;
        }

        // Recalculate taste scores to guarantee consistency
        for choice in choices.iter_mut() {
            let is_localized = !choice.blast_radius_impact.to_lowercase().contains("breaking")
                && !choice.blast_radius_impact.to_lowercase().contains("global");
            choice.taste_score = self.evaluate_taste(choice.pattern, &choice.tradeoffs, is_localized);
        }

        // Sort descending by taste score
        choices.sort_by(|a, b| {
            b.taste_score
                .value()
                .partial_cmp(&a.taste_score.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Mark only the top choice as recommended and tag its title
        for (idx, choice) in choices.iter_mut().enumerate() {
            if idx == 0 {
                choice.is_recommended = true;
                if !choice.title.starts_with("(Recommended") {
                    let pattern_tag = match choice.pattern {
                        ArchitecturalPattern::CleanArchitecture => "(Recommended - Clean Architecture)",
                        ArchitecturalPattern::Hexagonal => "(Recommended - Hexagonal / Ports & Adapters)",
                        ArchitecturalPattern::DataOriented => "(Recommended - Zero-GC DOD)",
                        ArchitecturalPattern::ActorMesh => "(Recommended - Actor Mesh)",
                        ArchitecturalPattern::FunctionalPipeline => "(Recommended - Functional Pipeline)",
                        ArchitecturalPattern::MicroKernel => "(Recommended - Microkernel)",
                    };
                    choice.title = format!("{} {}", pattern_tag, choice.title);
                }
            } else {
                choice.is_recommended = false;
                // Strip recommended prefix if previously present on lower choices
                if choice.title.starts_with("(Recommended") {
                    if let Some((_, rest)) = choice.title.split_once(") ") {
                        choice.title = rest.to_string();
                    }
                }
            }
        }
    }

    /// Helper to construct standard TradeoffMatrix presets.
    pub fn create_tradeoff_matrix(
        speed: u8,
        memory: u8,
        modularity: u8,
        velocity: u8,
        summary: impl Into<String>,
    ) -> TradeoffMatrix {
        TradeoffMatrix {
            execution_speed: TradeoffRating::new(speed),
            memory_efficiency: TradeoffRating::new(memory),
            modularity: TradeoffRating::new(modularity),
            development_velocity: TradeoffRating::new(velocity),
            summary: summary.into(),
        }
    }
}
