//! Trait definitions for Semantic Invariant Rule evaluation.
//!
//! Enforces unified evaluation signatures accepting source text, CST GreenTree roots, and ControlFlowGraphs.

#![forbid(unsafe_code)]

use super::types::SemanticViolation;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;

/// Unified trait implemented by all LOCUS Semantic Invariant Rules (S1..S5).
pub trait SemanticRule: Send + Sync {
    /// Canonical rule identifier (e.g. "S1", "S2").
    fn id(&self) -> &'static str;

    /// Human-readable rule title.
    fn name(&self) -> &'static str;

    /// Detailed description of the semantic hazard and rationale.
    fn description(&self) -> &'static str;

    /// Evaluates the rule across the source text, lossless CST Green Tree, and Control Flow Graph.
    fn check_source(
        &self,
        source: &str,
        tree: &GreenTreeRoot,
        cfg: &ControlFlowGraph,
    ) -> Vec<SemanticViolation>;
}
