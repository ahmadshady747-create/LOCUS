//! Universal CST Driver Trait Definition.
//!
//! Enforces the sovereign, polyglot contract across all compiler backends and languages.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::ResolvedSymbol;

/// Sovereign abstraction trait for polyglot Concrete Syntax Tree drivers.
///
/// Any supported language (Systems: Rust, C, C++, Go; Applications: TypeScript, Dart, Python, PHP, Java)
/// must implement this trait with 100% losslessness, sub-microsecond symbol extraction,
/// accurate control-flow graphing, and deterministic safety invariant checks.
pub trait UniversalCstDriver: Send + Sync {
    /// The unique language identifier handled by this driver.
    fn language_id(&self) -> LanguageId;

    /// Parses arbitrary source text into an immutable, 100% lossless Green Tree root.
    /// Retains all whitespace, comments, newlines, and punctuation.
    fn parse_lossless(&self, source: &str) -> Result<GreenTreeRoot, CstParseError>;

    /// Surgically extracts top-level and nested declarations (functions, structs, classes,
    /// traits, interfaces, enums, methods) into resolved symbol metadata.
    fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol>;

    /// Computes the deterministic Control Flow Graph (CFG) for the syntax tree.
    fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph;

    /// Evaluates language-specific and enterprise AST safety invariants, populating the bitset.
    fn check_language_invariants(&self, tree: &GreenTreeRoot, bitset: &mut InvariantBitset);
}

/// Convenience extension methods for any `UniversalCstDriver`.
pub trait UniversalCstDriverExt: UniversalCstDriver {
    /// Parses source text, extracts symbols, and maps them by name.
    fn extract_symbol_map(&self, tree: &GreenTreeRoot) -> HashMap<String, ResolvedSymbol> {
        let symbols = self.extract_symbols(tree);
        let mut map = HashMap::with_capacity(symbols.len());
        for sym in symbols {
            map.insert(sym.name.clone(), sym);
        }
        map
    }

    /// Parses and evaluates invariants in a single compound operation.
    fn parse_and_validate(&self, source: &str) -> Result<(GreenTreeRoot, InvariantBitset), CstParseError> {
        let tree = self.parse_lossless(source)?;
        let mut bitset = InvariantBitset::clean();
        self.check_language_invariants(&tree, &mut bitset);
        Ok((tree, bitset))
    }
}

impl<T: UniversalCstDriver + ?Sized> UniversalCstDriverExt for T {}
