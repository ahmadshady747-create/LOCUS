//! TypeScript / JavaScript Lossless CST Driver.
//!
//! Handles TS, TSX, JS, and JSX parsing, structural symbol extraction,
//! decorator tracking, interfaces, type aliases, control-flow graphing, and safety invariants.

#![forbid(unsafe_code)]

use crate::cst::polyglot::apps::component_tree::{ComponentNode, ComponentTreeExtractor};
use crate::cst::polyglot::apps::dataflow::{DataflowReport, DataflowTracker};
use crate::cst::polyglot::drivers::TypeScriptCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::{Language, ResolvedSymbol};

/// High-performance, lossless CST Driver for TypeScript and JavaScript.
#[derive(Debug, Default, Clone)]
pub struct TypeScriptDriver {
    inner: TypeScriptCstDriver,
}

impl TypeScriptDriver {
    pub fn new() -> Self {
        Self {
            inner: TypeScriptCstDriver::new(LanguageId::TypeScript),
        }
    }

    pub fn with_language(language_id: LanguageId) -> Self {
        Self {
            inner: TypeScriptCstDriver::new(language_id),
        }
    }

    /// Extracts visual React/JSX component tree from the source code.
    pub fn extract_component_tree(&self, source: &str) -> Vec<ComponentNode> {
        ComponentTreeExtractor::new().extract(source, Language::Tsx)
    }

    /// Analyzes state lifecycle and cross-component dataflow hazards.
    pub fn track_dataflow(&self, source: &str) -> DataflowReport {
        DataflowTracker::new().analyze(source, Language::Tsx)
    }
}

impl UniversalCstDriver for TypeScriptDriver {
    fn language_id(&self) -> LanguageId {
        self.inner.language_id()
    }

    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError> {
        self.inner.parse_lossless(source)
    }

    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        self.inner.extract_symbols(tree)
    }

    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        self.inner.compute_cfg(tree)
    }

    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset) {
        self.inner.check_language_invariants(tree, bitset);
    }
}
