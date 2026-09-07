//! Data types and mathematical models for the Ambiguity Entropy Metric Engine.
//!
//! Enforces normalized bounds H(intent) in [0.0, 1.0], structured ambiguity targets,
//! and threshold configurations for autonomous execution vs Socratic clarification.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Ambiguity Score (Normalized [0.0, 1.0])
// ---------------------------------------------------------------------------

/// A normalized ambiguity entropy score H(intent) in [0.0, 1.0].
/// - `0.0`: Fully deterministic, complete specification, exact symbols, paths, and types.
/// - `1.0`: Completely ambiguous, abstract, undefined symbols, no paths, contradictory intent.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AmbiguityScore(pub f32);

impl AmbiguityScore {
    pub const ZERO: Self = Self(0.0);
    pub const MAX: Self = Self(1.0);

    /// Constructs a new `AmbiguityScore`, strictly clamping the inner value to `[0.0, 1.0]`.
    pub fn new(val: f32) -> Self {
        let clamped = if val.is_nan() {
            1.0
        } else {
            val.clamp(0.0, 1.0)
        };
        Self(clamped)
    }

    /// Returns the raw float value.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Returns true if the ambiguity is low enough for autonomous execution without dialogue.
    pub fn is_acceptable(&self, config: &EntropyThresholdConfig) -> bool {
        self.0 <= config.acceptable_threshold
    }

    /// Returns true if the ambiguity requires interactive Socratic clarification.
    pub fn requires_dialogue(&self, config: &EntropyThresholdConfig) -> bool {
        self.0 > config.socratic_dialogue_threshold
    }

    /// Returns true if the intent is critically ambiguous and must be frozen.
    pub fn is_critical(&self, config: &EntropyThresholdConfig) -> bool {
        self.0 >= config.critical_freeze_threshold
    }
}

impl Default for AmbiguityScore {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for AmbiguityScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Threshold Configuration
// ---------------------------------------------------------------------------

/// Configurable thresholds governing intent classification and Socratic interventions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntropyThresholdConfig {
    /// Maximum entropy score permitted for zero-dialogue autonomous execution (default: 0.15).
    pub acceptable_threshold: f32,
    /// Entropy score trigger point for interactive Socratic clarification (default: 0.35).
    pub socratic_dialogue_threshold: f32,
    /// Critical entropy score causing immediate freeze and rejection (default: 0.70).
    pub critical_freeze_threshold: f32,
}

impl Default for EntropyThresholdConfig {
    fn default() -> Self {
        Self {
            acceptable_threshold: 0.15,
            socratic_dialogue_threshold: 0.35,
            critical_freeze_threshold: 0.70,
        }
    }
}

impl EntropyThresholdConfig {
    /// Strict configuration for critical production systems.
    pub fn strict() -> Self {
        Self {
            acceptable_threshold: 0.08,
            socratic_dialogue_threshold: 0.20,
            critical_freeze_threshold: 0.50,
        }
    }

    /// Permissive configuration for exploratory prototyping.
    pub fn permissive() -> Self {
        Self {
            acceptable_threshold: 0.25,
            socratic_dialogue_threshold: 0.45,
            critical_freeze_threshold: 0.85,
        }
    }
}

// ---------------------------------------------------------------------------
// Ambiguous Target
// ---------------------------------------------------------------------------

/// Identifies a named entity in the prompt that resolves to multiple potential symbols or paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbiguousTarget {
    pub identifier: String,
    pub candidate_paths: Vec<String>,
    pub candidate_kinds: Vec<String>,
    pub confidence: f32,
    pub disambiguation_hint: String,
}

// ---------------------------------------------------------------------------
// Entropy Component Breakdown
// ---------------------------------------------------------------------------

/// Fine-grained breakdown of the 4 entropy dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EntropyBreakdown {
    /// E_symbols (0.0 to 1.0): Missing, unreferenced, or ambiguous function/struct symbols.
    pub symbol_entropy: f32,
    /// E_types (0.0 to 1.0): Undefined input/output type contracts, return signatures.
    pub type_entropy: f32,
    /// E_scope (0.0 to 1.0): Unspecified file paths, target modules, or workspace boundaries.
    pub scope_entropy: f32,
    /// E_spec (0.0 to 1.0): Missing edge-case specifications, error models, concurrency guards.
    pub spec_entropy: f32,
}

// ---------------------------------------------------------------------------
// Edge Case Categories & Violations
// ---------------------------------------------------------------------------

/// Categorization of common edge-case gaps in prompt specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeCaseKind {
    MissingErrorHandling,
    ConcurrencyGap,
    NullabilityBoundaryGap,
    ResourceCleanupGap,
    InputValidationGap,
    SecurityContextGap,
}

impl fmt::Display for EdgeCaseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EdgeCaseKind::MissingErrorHandling => "MISSING_ERROR_HANDLING",
            EdgeCaseKind::ConcurrencyGap => "CONCURRENCY_GAP",
            EdgeCaseKind::NullabilityBoundaryGap => "NULLABILITY_BOUNDARY_GAP",
            EdgeCaseKind::ResourceCleanupGap => "RESOURCE_CLEANUP_GAP",
            EdgeCaseKind::InputValidationGap => "INPUT_VALIDATION_GAP",
            EdgeCaseKind::SecurityContextGap => "SECURITY_CONTEXT_GAP",
        };
        write!(f, "{}", s)
    }
}

/// A specific underspecified edge-case detected in the prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeCaseViolation {
    pub kind: EdgeCaseKind,
    pub description: String,
    pub suggested_guard: String,
}

// ---------------------------------------------------------------------------
// Comprehensive Intent Entropy Report
// ---------------------------------------------------------------------------

/// Complete report of the Ambiguity Entropy evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentEntropyReport {
    /// Original developer prompt or intent string.
    pub intent: String,
    /// Compound ambiguity entropy H(intent) in [0.0, 1.0].
    pub total_entropy: AmbiguityScore,
    /// Detailed 4-dimensional entropy breakdown.
    pub breakdown: EntropyBreakdown,
    /// Symbols mentioned in prompt that do not exist in workspace CST index.
    pub missing_symbols: Vec<String>,
    /// Identifiers that match multiple conflicting candidates.
    pub ambiguous_targets: Vec<AmbiguousTarget>,
    /// Identified gaps in error handling, concurrency, or boundaries.
    pub underspecified_edge_cases: Vec<EdgeCaseViolation>,
    /// Confidence in the intent classification (1.0 - total_entropy).
    pub confidence_level: f32,
    /// True if entropy exceeds the Socratic dialogue threshold.
    pub requires_socratic_dialogue: bool,
    /// Targeted questions generated to resolve the ambiguity.
    pub socratic_clarification_questions: Vec<String>,
    /// Wall-clock evaluation latency in milliseconds.
    pub latency_ms: f64,
}

impl IntentEntropyReport {
    /// Returns true if the intent is safe for autonomous execution.
    pub fn is_executable(&self, config: &EntropyThresholdConfig) -> bool {
        self.total_entropy.is_acceptable(config) && !self.requires_socratic_dialogue
    }
}
