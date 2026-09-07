//! Dart & Flutter Lossless CST Driver.
//!
//! Analyzes Flutter Widgets (`StatelessWidget`, `StatefulWidget`, `State<T>`),
//! `build(BuildContext context)` trees, mixins, constructors, annotations, and package imports.

#![forbid(unsafe_code)]

use crate::cst::polyglot::apps::component_tree::{ComponentNode, ComponentTreeExtractor};
use crate::cst::polyglot::apps::dataflow::{DataflowReport, DataflowTracker};
use crate::cst::polyglot::drivers::DartCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::{Language, ResolvedSymbol};

/// High-performance, lossless CST Driver for Dart and Flutter.
#[derive(Debug, Default, Clone)]
pub struct DartDriver {
    inner: DartCstDriver,
}

impl DartDriver {
    pub fn new() -> Self {
        Self {
            inner: DartCstDriver,
        }
    }

    /// Extracts visual Flutter Widget tree from source.
    pub fn extract_widget_tree(&self, source: &str) -> Vec<ComponentNode> {
        ComponentTreeExtractor::new().extract(source, Language::Dart)
    }

    /// Analyzes state lifecycle and cross-component dataflow hazards.
    pub fn track_dataflow(&self, source: &str) -> DataflowReport {
        DataflowTracker::new().analyze(source, Language::Dart)
    }
}

impl UniversalCstDriver for DartDriver {
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
