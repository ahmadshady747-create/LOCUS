//! Data types and domain structures for Socratic Dialogue & Architectural Taste.
//!
//! Defines normalized taste scores `TasteScore` in `[0.0, 1.0]`, architectural patterns,
//! constrained Socratic choices, tradeoff matrices, and intent resolutions.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Taste Score (Normalized [0.0, 1.0])
// ---------------------------------------------------------------------------

/// Normalized Architectural Taste Score in `[0.0, 1.0]`.
///
/// Reflects alignment with high-order engineering principles (Clean Architecture,
/// Zero-Alloc DOD, loose coupling, testability, and blast-radius minimization).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TasteScore(pub f32);

impl TasteScore {
    pub const ZERO: Self = Self(0.0);
    pub const PERFECT: Self = Self(1.0);

    /// Constructs a new `TasteScore`, strictly clamped to `[0.0, 1.0]`.
    pub fn new(val: f32) -> Self {
        let clamped = if val.is_nan() {
            0.0
        } else {
            val.clamp(0.0, 1.0)
        };
        Self(clamped)
    }

    /// Returns the raw float value.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Returns true if this score qualifies as high architectural taste (>= 0.75).
    pub fn is_high_taste(&self) -> bool {
        self.0 >= 0.75
    }
}

impl Default for TasteScore {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for TasteScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Architectural Pattern Enumeration
// ---------------------------------------------------------------------------

/// Well-established software architectural patterns supported by the LOCUS Taste Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchitecturalPattern {
    /// Domain-Driven Clean Architecture with explicit use-cases and entity boundaries.
    CleanArchitecture,
    /// Hexagonal / Ports & Adapters isolating core business logic from IO and external frameworks.
    Hexagonal,
    /// Data-Oriented Design (DOD) / Cache-locality / Zero-Allocation memory layout.
    DataOriented,
    /// Actor-Mesh / Message-Passing concurrent isolation without shared mutable state.
    ActorMesh,
    /// Pure Functional Data Pipeline with composable, deterministic transformations.
    FunctionalPipeline,
    /// Sovereign Micro-Kernel with extensible plugin hooks and minimal core footprint.
    MicroKernel,
}

impl ArchitecturalPattern {
    /// Human-readable name of the architectural pattern.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CleanArchitecture => "Clean Architecture",
            Self::Hexagonal => "Hexagonal / Ports & Adapters",
            Self::DataOriented => "Data-Oriented Zero-Alloc (DOD)",
            Self::ActorMesh => "Actor Mesh / Shared-Nothing",
            Self::FunctionalPipeline => "Pure Functional Pipeline",
            Self::MicroKernel => "Sovereign Microkernel",
        }
    }

    /// Primary architectural strength.
    pub fn primary_strength(&self) -> &'static str {
        match self {
            Self::CleanArchitecture => "Exceptional Modularity & Testability",
            Self::Hexagonal => "Zero Coupling to External Infrastructure",
            Self::DataOriented => "Sub-microsecond CPU Cache Locality & Zero GC Overhead",
            Self::ActorMesh => "Lock-free Concurrency & Fault Isolation",
            Self::FunctionalPipeline => "Deterministic Immutability & Verifiable Safety",
            Self::MicroKernel => "Extremely Low Blast Radius & Modular Extensibility",
        }
    }
}

impl fmt::Display for ArchitecturalPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ---------------------------------------------------------------------------
// Tradeoff Matrix
// ---------------------------------------------------------------------------

/// Quantified engineering rating on a normalized scale (1 = Low/Poor, 10 = Best/Ultra).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeoffRating(pub u8);

impl TradeoffRating {
    pub fn new(val: u8) -> Self {
        Self(val.clamp(1, 10))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Comprehensive 4-dimensional engineering tradeoff matrix for a solution choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeoffMatrix {
    /// Runtime Execution Speed & CPU Efficiency (1 = Slow/Interpreted, 10 = Zero-Overhead SIMD/Bare-Metal).
    pub execution_speed: TradeoffRating,
    /// Memory Footprint & Allocation Efficiency (1 = Heavy GC/Allocations, 10 = Zero-Allocation DOD).
    pub memory_efficiency: TradeoffRating,
    /// Codebase Modularity & Testability (1 = Monolithic Spagetti, 10 = Pure Isolable Units).
    pub modularity: TradeoffRating,
    /// Cognitive Load & Development Velocity (1 = High Ceremony/Boilerplate, 10 = Immediate/Low Complexity).
    pub development_velocity: TradeoffRating,
    /// Concise summary rationale explaining the specific engineering tradeoffs.
    pub summary: String,
}

impl Default for TradeoffMatrix {
    fn default() -> Self {
        Self {
            execution_speed: TradeoffRating(7),
            memory_efficiency: TradeoffRating(7),
            modularity: TradeoffRating(8),
            development_velocity: TradeoffRating(7),
            summary: "Balanced general-purpose engineering tradeoffs.".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Architectural Choice
// ---------------------------------------------------------------------------

/// A concrete, structured architectural path presented in a Socratic Dialogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchitecturalChoice {
    /// Unique identifier for this choice (e.g. `choice_clean_repo`).
    pub id: String,
    /// Concise title (e.g. `Hexagonal Port & Adapter with Result Domain Error`).
    pub title: String,
    /// Technical description explaining how this choice resolves the intent.
    pub description: String,
    /// Architectural pattern represented.
    pub pattern: ArchitecturalPattern,
    /// Computed architectural taste score $T \in [0.0, 1.0]$.
    pub taste_score: TasteScore,
    /// True if the Taste Engine selects this as the primary recommended solution.
    pub is_recommended: bool,
    /// Detailed 4-dimensional tradeoff matrix.
    pub tradeoffs: TradeoffMatrix,
    /// Concrete structural code snippet sketch demonstrating the design.
    pub code_sketch: Option<String>,
    /// Estimated blast radius impact on existing downstream callers.
    pub blast_radius_impact: String,
}

// ---------------------------------------------------------------------------
// Socratic Question
// ---------------------------------------------------------------------------

/// A focused, constrained Socratic clarification question containing exactly 3 choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocraticQuestion {
    /// Unique question identifier (e.g. `sq_data_layer_storage`).
    pub id: String,
    /// Contextual description of the developer's original prompt.
    pub prompt_context: String,
    /// The specific ambiguity source detected (e.g. `Storage Layer & Mutation Flow`).
    pub ambiguity_source: String,
    /// Exactly 3 structured architectural choices (1 Strongly Recommended + 2 Alternatives).
    pub choices: Vec<ArchitecturalChoice>,
    /// ID of the recommended choice.
    pub recommended_choice_id: String,
}

impl SocraticQuestion {
    /// Returns the recommended choice if present.
    pub fn recommended_choice(&self) -> Option<&ArchitecturalChoice> {
        self.choices.iter().find(|c| c.is_recommended)
    }

    /// Finds a choice by its unique ID.
    pub fn get_choice(&self, id: &str) -> Option<&ArchitecturalChoice> {
        self.choices.iter().find(|c| c.id == id)
    }
}

// ---------------------------------------------------------------------------
// Intent Resolution
// ---------------------------------------------------------------------------

/// Final resolution of a developer's intent after resolving Socratic choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentResolution {
    /// ID of the answered Socratic question.
    pub question_id: String,
    /// ID of the selected choice.
    pub selected_choice_id: String,
    /// Architectural pattern applied.
    pub applied_pattern: ArchitecturalPattern,
    /// Specific type and signature contract hints to feed into LOCUS synthesis.
    pub synthesized_contract_hints: Vec<String>,
    /// Mandatory safety invariants locked down by this choice.
    pub enforced_invariants: Vec<String>,
    /// Resolved taste score.
    pub final_taste_score: TasteScore,
}
