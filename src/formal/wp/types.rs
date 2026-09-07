//! Data types and models for Dijkstra's Weakest Precondition Calculus & Hoare Logic.
//!
//! Enforces mathematical invariant representations, structured AST statements,
//! Hoare triples, and formal proof verdicts in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Semantic Predicates
// ---------------------------------------------------------------------------

/// First-order logic predicates used for preconditions, postconditions, and wp-calculus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemanticPredicate {
    True,
    False,
    Atom(String),
    And(Box<SemanticPredicate>, Box<SemanticPredicate>),
    Or(Box<SemanticPredicate>, Box<SemanticPredicate>),
    Not(Box<SemanticPredicate>),
    Imply(Box<SemanticPredicate>, Box<SemanticPredicate>),
    /// Variable bounds invariant: min <= var <= max
    BoundsCheck {
        var: String,
        min: i64,
        max: i64,
    },
    /// Division by zero safety: var != 0
    NonZero(String),
    /// Relational comparisons
    Gt(String, i64),
    Lt(String, i64),
    Eq(String, i64),
}

impl SemanticPredicate {
    pub fn and(self, other: SemanticPredicate) -> Self {
        match (self, other) {
            (Self::True, p) | (p, Self::True) => p,
            (Self::False, _) | (_, Self::False) => Self::False,
            (a, b) => Self::And(Box::new(a), Box::new(b)),
        }
    }

    pub fn or(self, other: SemanticPredicate) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, p) | (p, Self::False) => p,
            (a, b) => Self::Or(Box::new(a), Box::new(b)),
        }
    }

    pub fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Not(inner) => *inner,
            p => Self::Not(Box::new(p)),
        }
    }

    pub fn imply(self, other: SemanticPredicate) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::True) => Self::True,
            (Self::True, p) => p,
            (a, b) => Self::Imply(Box::new(a), Box::new(b)),
        }
    }
}

impl fmt::Display for SemanticPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Atom(s) => write!(f, "{}", s),
            Self::And(a, b) => write!(f, "({} ∧ {})", a, b),
            Self::Or(a, b) => write!(f, "({} ∨ {})", a, b),
            Self::Not(p) => write!(f, "¬{}", p),
            Self::Imply(a, b) => write!(f, "({} ⟹ {})", a, b),
            Self::BoundsCheck { var, min, max } => write!(f, "({} ≤ {} ≤ {})", min, var, max),
            Self::NonZero(var) => write!(f, "({} ≠ 0)", var),
            Self::Gt(var, val) => write!(f, "({} > {})", var, val),
            Self::Lt(var, val) => write!(f, "({} < {})", var, val),
            Self::Eq(var, val) => write!(f, "({} = {})", var, val),
        }
    }
}

// ---------------------------------------------------------------------------
// Abstract Syntax Expressions & Statements
// ---------------------------------------------------------------------------

/// Arithmetic and variable expressions in formal ASTs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstExpression {
    Var(String),
    Const(i64),
    Add(Box<AstExpression>, Box<AstExpression>),
    Sub(Box<AstExpression>, Box<AstExpression>),
    Mul(Box<AstExpression>, Box<AstExpression>),
    Div(Box<AstExpression>, Box<AstExpression>),
}

/// Imperative program statements for formal verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstStatement {
    Skip,
    Assign {
        var: String,
        expr: AstExpression,
    },
    Seq(Box<AstStatement>, Box<AstStatement>),
    IfElse {
        cond_var: String,
        cond_op: String,
        cond_val: i64,
        then_branch: Box<AstStatement>,
        else_branch: Box<AstStatement>,
    },
    Assert(SemanticPredicate),
}

// ---------------------------------------------------------------------------
// Hoare Triples & Proof Verdicts
// ---------------------------------------------------------------------------

/// A formal Hoare Triple: `{Precondition} Statement {Postcondition}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoareTriple {
    pub precondition: SemanticPredicate,
    pub statement: AstStatement,
    pub postcondition: SemanticPredicate,
}

impl HoareTriple {
    pub fn new(
        precondition: SemanticPredicate,
        statement: AstStatement,
        postcondition: SemanticPredicate,
    ) -> Self {
        Self {
            precondition,
            statement,
            postcondition,
        }
    }
}

/// Formal mathematical proof result for invariant verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofVerdict {
    /// Invariant mathematically proven safe for all valid inputs.
    ProvenSafe,
    /// Invariant violated with concrete counterexample inputs.
    ViolationFound {
        counterexample: String,
        failed_predicate: SemanticPredicate,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalError {
    #[error("Proof failure: {0}")]
    ProofFailure(String),

    #[error("Evaluation timeout during semantic proof solver")]
    SolverTimeout,

    #[error("Internal formal prover error: {0}")]
    InternalError(String),
}
