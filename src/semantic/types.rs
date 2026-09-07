//! Domain types and diagnostic models for the Semantic Invariant Rules Subsystem.
//!
//! Provides structured violation reports, location pinpointing, and severity ratings in 100% pure safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Severity classification for semantic invariant violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Non-fatal style or mild performance issue.
    Low,
    /// Potential runtime bug or logic flaw.
    Medium,
    /// Definite logic bug, memory identity mismatch, or concurrency trap.
    High,
    /// Critical runtime exception or data race.
    Critical,
}

impl fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Exact physical byte and line location of a detected semantic violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationLocation {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number.
    pub column: usize,
    /// 0-indexed byte offset in the source text.
    pub byte_offset: usize,
    /// Length of the target node or token in bytes.
    pub span_len: usize,
    /// Optional code snippet or symbol name.
    pub snippet: Option<String>,
}

impl ViolationLocation {
    pub fn new(line: usize, column: usize, byte_offset: usize, span_len: usize, snippet: Option<String>) -> Self {
        Self {
            line,
            column,
            byte_offset,
            span_len,
            snippet,
        }
    }

    /// Computes 1-indexed line and column coordinates from a 0-indexed byte offset in source text.
    pub fn from_offset(source: &str, byte_offset: usize, span_len: usize, snippet: Option<String>) -> Self {
        let safe_offset = byte_offset.min(source.len());
        let prefix = &source[..safe_offset];
        let line = prefix.matches('\n').count() + 1;
        let last_nl = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let column = safe_offset - last_nl + 1;

        Self {
            line,
            column,
            byte_offset: safe_offset,
            span_len,
            snippet,
        }
    }
}

/// Complete diagnostic record of a detected semantic invariant rule violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticViolation {
    pub rule_id: String,
    pub rule_name: String,
    pub message: String,
    pub severity: ViolationSeverity,
    pub location: ViolationLocation,
    pub suggested_fix: Option<String>,
}

impl SemanticViolation {
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        message: impl Into<String>,
        severity: ViolationSeverity,
        location: ViolationLocation,
        suggested_fix: Option<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            message: message.into(),
            severity,
            location,
            suggested_fix,
        }
    }
}

/// Mathematical proof card demonstrating formal logical derivation, falsification, and remediation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProof {
    /// Canonical rule ID (e.g. "S1", "S2").
    pub rule_id: String,
    /// Canonical rule name (e.g. "AsyncIteratorRace").
    pub rule_name: String,
    /// Concrete violation record.
    pub violation: SemanticViolation,
    /// Formal mathematical proof or WP calculus derivation explaining the defect.
    pub mathematical_proof: String,
    /// Concrete counterexample input or execution trace falsifying the correctness invariant.
    pub counterexample: Option<String>,
    /// Deterministic drop-in patch applying the verified remediation.
    pub remediation_patch: String,
}

impl SemanticProof {
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        violation: SemanticViolation,
        mathematical_proof: impl Into<String>,
        counterexample: Option<String>,
        remediation_patch: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            violation,
            mathematical_proof: mathematical_proof.into(),
            counterexample,
            remediation_patch: remediation_patch.into(),
        }
    }
}

/// Consolidated semantic prover report returned by `LocusSemanticAnalyzer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticReport {
    /// Total number of semantic invariant rules evaluated.
    pub total_rules_evaluated: usize,
    /// Total count of violations detected.
    pub violations_count: usize,
    /// Synthesized proof cards for each violation.
    pub proofs: Vec<SemanticProof>,
    /// Total end-to-end execution latency in milliseconds.
    pub execution_latency_ms: f64,
    /// True if zero violations were detected.
    pub passed: bool,
}

impl SemanticReport {
    pub fn new(
        total_rules_evaluated: usize,
        proofs: Vec<SemanticProof>,
        execution_latency_ms: f64,
    ) -> Self {
        let violations_count = proofs.len();
        let passed = violations_count == 0;
        Self {
            total_rules_evaluated,
            violations_count,
            proofs,
            execution_latency_ms,
            passed,
        }
    }
}
