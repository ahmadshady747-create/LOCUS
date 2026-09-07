//! Domain Types and Verdicts for Release Audit Gate & CI/CD Verification.
//!
//! Enforces zero-regression thresholds, blocker violation isolation, and deterministic exit codes in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::sarif::types::SarifResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Audit Gate Configuration
// ---------------------------------------------------------------------------

/// Threshold constraints and quality gate policies enforced before release signoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditGateConfig {
    /// Maximum allowable error-level violations (hard-zero: 0).
    pub max_errors: usize,
    /// Maximum allowable warning-level violations.
    pub max_warnings: usize,
    /// Maximum allowable execution latency budget in milliseconds.
    pub max_latency_ms: Option<f64>,
    /// Strict mode treats all non-note findings as blockers.
    pub strict_mode: bool,
}

impl Default for AuditGateConfig {
    fn default() -> Self {
        Self {
            max_errors: 0,
            max_warnings: 0, // Strict zero-tolerance by default
            max_latency_ms: None,
            strict_mode: true,
        }
    }
}

impl AuditGateConfig {
    pub fn zero_tolerance() -> Self {
        Self::default()
    }

    pub fn permissive_warnings(max_warnings: usize) -> Self {
        Self {
            max_errors: 0,
            max_warnings,
            max_latency_ms: None,
            strict_mode: false,
        }
    }

    pub fn with_max_latency(mut self, max_latency_ms: f64) -> Self {
        self.max_latency_ms = Some(max_latency_ms);
        self
    }
}

// ---------------------------------------------------------------------------
// Audit Verdict
// ---------------------------------------------------------------------------

/// Deterministic quality gate decision issued by `ReleaseAuditGate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditVerdict {
    Approved {
        total_rules_checked: usize,
        total_results_evaluated: usize,
        error_count: usize,
        warning_count: usize,
        execution_latency_ms: f64,
    },
    Rejected {
        blocker_count: usize,
        total_violations: usize,
        violations: Vec<SarifResult>,
        reason: String,
        execution_latency_ms: f64,
    },
}

impl AuditVerdict {
    /// Returns true if the audit gate approved the release.
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    /// Returns the deterministic process exit code (0 for approved, 1 for rejected).
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Approved { .. } => 0,
            Self::Rejected { .. } => 1,
        }
    }
}

impl fmt::Display for AuditVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approved {
                total_rules_checked,
                total_results_evaluated,
                execution_latency_ms,
                ..
            } => {
                write!(
                    f,
                    "APPROVED (0 blockers, {} rules checked, {} results in {:.2}ms)",
                    total_rules_checked, total_results_evaluated, execution_latency_ms
                )
            }
            Self::Rejected {
                blocker_count,
                reason,
                execution_latency_ms,
                ..
            } => {
                write!(
                    f,
                    "REJECTED ({} blockers, reason: '{}' in {:.2}ms)",
                    blocker_count, reason, execution_latency_ms
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReleaseError {
    #[error("Release audit gate rejected build: {0}")]
    GateRejected(String),

    #[error("SARIF parsing error: {0}")]
    SarifParseError(String),

    #[error("Invalid audit configuration: {0}")]
    InvalidConfig(String),
}
