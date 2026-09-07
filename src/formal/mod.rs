//! Formal Verification Subsystem — Dijkstra's Weakest Precondition Calculus, Hoare Triples & Invariant Provers.

#![forbid(unsafe_code)]

pub mod wp;

pub use wp::{
    AstExpression, AstStatement, FormalError, HoareTriple, ProofVerdict, SemanticPredicate,
    SemanticSolver, WeakestPreconditionEngine,
};
