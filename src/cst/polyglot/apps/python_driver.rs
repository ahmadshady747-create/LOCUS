//! Python Lossless CST Driver.
//!
//! Handles indentation-sensitive blocks, classes, functions, async defs,
//! type hints, decorators (`@dataclass`, `@property`), docstrings, and imports.

#![forbid(unsafe_code)]

use crate::cst::polyglot::apps::dataflow::{DataflowReport, DataflowTracker};
use crate::cst::polyglot::drivers::PythonCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::{Language, ResolvedSymbol};

/// High-performance, lossless CST Driver for Python.
#[derive(Debug, Default, Clone)]
pub struct PythonDriver {
    inner: PythonCstDriver,
}

impl PythonDriver {
    pub fn new() -> Self {
        Self {
            inner: PythonCstDriver,
        }
    }

    /// Analyzes state lifecycle and cross-component dataflow hazards.
    pub fn track_dataflow(&self, source: &str) -> DataflowReport {
        DataflowTracker::new().analyze(source, Language::Python)
    }
}

impl UniversalCstDriver for PythonDriver {
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
