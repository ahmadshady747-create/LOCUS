//! Java Lossless CST Driver.
//!
//! Analyzes package declarations, static/standard imports, classes, interfaces,
//! records (`record`), enums, annotations (`@RestController`, `@Override`),
//! generics, and access modifiers (`public`, `protected`, `private`).

#![forbid(unsafe_code)]

use crate::cst::polyglot::apps::dataflow::{DataflowReport, DataflowTracker};
use crate::cst::polyglot::drivers::JavaCstDriver;
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::{Language, ResolvedSymbol};

/// High-performance, lossless CST Driver for Java.
#[derive(Debug, Default, Clone)]
pub struct JavaDriver {
    inner: JavaCstDriver,
}

impl JavaDriver {
    pub fn new() -> Self {
        Self {
            inner: JavaCstDriver,
        }
    }

    /// Analyzes state lifecycle and cross-component dataflow hazards.
    pub fn track_dataflow(&self, source: &str) -> DataflowReport {
        DataflowTracker::new().analyze(source, Language::Java)
    }
}

impl UniversalCstDriver for JavaDriver {
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
