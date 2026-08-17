//! Directive-Bound Bidirectional Formal Verifier & Invariant Engine Contracts.

use serde::{Deserialize, Serialize};

/// Expression representing a formal mathematical or programmatic constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConstraintExpression {
    /// Variable must lie within inclusive range: [min, max].
    RangeBound {
        var: String,
        min: Option<i64>,
        max: Option<i64>,
    },
    /// Index variable must be strictly smaller than array length: 0 <= idx < len.
    ArrayBound {
        array_var: String,
        index_var: String,
    },
    /// Variable must never evaluate to zero (division safety): var != 0.
    NonZero {
        var: String,
    },
    /// Pointer or reference must never evaluate to null: ptr != null.
    NonNull {
        var: String,
    },
    /// General boolean expression or condition predicate.
    CustomPredicate {
        expr: String,
    },
}

/// Comprehensive specification contract for a function, loop, or patch chunk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodeContract {
    /// Preconditions (requires): must hold prior to code execution.
    pub requires: Vec<ConstraintExpression>,
    /// Postconditions (ensures): must hold upon code termination.
    pub ensures: Vec<ConstraintExpression>,
    /// Invariants: must hold invariant across loops and intermediate state transformations.
    pub invariants: Vec<ConstraintExpression>,
    /// Directive: developer intent / specification instruction.
    pub directive: Option<String>,
}

/// Concrete counterexample proving a formal specification violation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Counterexample {
    pub failing_var: String,
    pub failing_val: String,
    pub violation_expr: String,
    pub trace_summary: String,
}

/// Comprehensive verdict from the dual-pass bidirectional formal verification engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationVerdict {
    /// Forward safety pass: proves Precondition & Code => Postcondition.
    pub forward_safety_proved: bool,
    /// Backward intent pass: proves wp(Code, Postcondition) & Directive => True.
    pub backward_intent_proved: bool,
    /// Complete bidirectional verification status.
    pub is_bidirectionally_verified: bool,
    /// Verification confidence score between 0.0 and 1.0.
    pub confidence: f32,
    /// Total proof computation time in milliseconds.
    pub proof_time_ms: u64,
    /// Total bounded symbolic evaluation steps executed.
    pub steps_evaluated: usize,
    /// Concrete counterexample if any pass failed.
    pub counterexample: Option<Counterexample>,
    /// Name or expression of the violated contract.
    pub violated_contract: Option<String>,
}

impl VerificationVerdict {
    pub fn success(proof_time_ms: u64, steps: usize) -> Self {
        Self {
            forward_safety_proved: true,
            backward_intent_proved: true,
            is_bidirectionally_verified: true,
            confidence: 1.0,
            proof_time_ms,
            steps_evaluated: steps,
            counterexample: None,
            violated_contract: None,
        }
    }

    pub fn failure(
        forward: bool,
        backward: bool,
        violation: String,
        counterexample: Counterexample,
        proof_time_ms: u64,
        steps: usize,
    ) -> Self {
        Self {
            forward_safety_proved: forward,
            backward_intent_proved: backward,
            is_bidirectionally_verified: false,
            confidence: if forward || backward { 0.5 } else { 0.0 },
            proof_time_ms,
            steps_evaluated: steps,
            counterexample: Some(counterexample),
            violated_contract: Some(violation),
        }
    }
}
