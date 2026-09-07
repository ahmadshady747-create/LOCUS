//! Thread-Safe Polyglot Driver Registry.
//!
//! Central catalog for dynamic registration, resolution, and high-throughput dispatch
//! of language-specific CST drivers.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use parking_lot::RwLock;

use crate::cst::polyglot::drivers::{
    DartCstDriver, GenericPolyglotDriver, JavaCstDriver,
    PhpCstDriver, PythonCstDriver, TypeScriptCstDriver,
};
use crate::cst::polyglot::systems::{CCstDriver, CppCstDriver, GoCstDriver, RustCstDriver};
use crate::cst::polyglot::traits::UniversalCstDriver;
use crate::cst::polyglot::types::{ControlFlowGraph, CstParseError, GreenTreeRoot, InvariantBitset, LanguageId};
use crate::types::ResolvedSymbol;

/// Global singleton registry initialized with default polyglot drivers.
pub static GLOBAL_REGISTRY: LazyLock<UniversalDriverRegistry> =
    LazyLock::new(UniversalDriverRegistry::with_defaults);

/// Thread-safe registry mapping `LanguageId` to sovereign `UniversalCstDriver` implementations.
pub struct UniversalDriverRegistry {
    drivers: RwLock<HashMap<LanguageId, Arc<dyn UniversalCstDriver>>>,
}

impl Default for UniversalDriverRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl UniversalDriverRegistry {
    /// Creates an empty registry without pre-loaded drivers.
    pub fn new() -> Self {
        Self {
            drivers: RwLock::new(HashMap::new()),
        }
    }

    /// Initializes a registry pre-loaded with standard drivers across all 14+ supported languages.
    pub fn with_defaults() -> Self {
        let registry = Self::new();

        // 1. Systems Drivers
        registry.register(Arc::new(RustCstDriver));
        registry.register(Arc::new(GoCstDriver));
        registry.register(Arc::new(CppCstDriver));
        registry.register(Arc::new(CCstDriver));

        // 2. Application & Scripting Drivers
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::TypeScript)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::JavaScript)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::Tsx)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::Jsx)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::Svelte)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::Astro)));
        registry.register(Arc::new(TypeScriptCstDriver::new(LanguageId::Vue)));
        registry.register(Arc::new(PythonCstDriver));
        registry.register(Arc::new(DartCstDriver));
        registry.register(Arc::new(JavaCstDriver));
        registry.register(Arc::new(PhpCstDriver));

        registry
    }

    /// Registers or overrides a language driver.
    pub fn register(&self, driver: Arc<dyn UniversalCstDriver>) {
        let lang = driver.language_id();
        self.drivers.write().insert(lang, driver);
    }

    /// Retrieves the driver registered for a specific language ID.
    pub fn get(&self, lang: &LanguageId) -> Option<Arc<dyn UniversalCstDriver>> {
        self.drivers.read().get(lang).cloned()
    }

    /// Retrieves the driver registered for a language, or falls back to `GenericPolyglotDriver`.
    pub fn get_or_fallback(&self, lang: &LanguageId) -> Arc<dyn UniversalCstDriver> {
        if let Some(d) = self.get(lang) {
            return d;
        }
        Arc::new(GenericPolyglotDriver::new(lang.clone()))
    }

    /// Resolves and retrieves the driver matching a file extension.
    pub fn get_for_extension(&self, ext: &str) -> Arc<dyn UniversalCstDriver> {
        let lang = LanguageId::from_extension(ext);
        self.get_or_fallback(&lang)
    }

    /// Returns a list of all currently registered language identifiers.
    pub fn registered_languages(&self) -> Vec<LanguageId> {
        self.drivers.read().keys().cloned().collect()
    }

    /// Convenient top-level dispatcher: Parses source text losslessly using the appropriate driver.
    pub fn parse_lossless(&self, source: &str, lang: &LanguageId) -> Result<GreenTreeRoot, CstParseError> {
        let driver = self.get_or_fallback(lang);
        driver.parse_lossless(source)
    }

    /// Convenient top-level dispatcher: Extracts symbols from a CST root.
    pub fn extract_symbols(&self, tree: &GreenTreeRoot) -> Vec<ResolvedSymbol> {
        let driver = self.get_or_fallback(tree.language_id());
        driver.extract_symbols(tree)
    }

    /// Convenient top-level dispatcher: Computes the Control Flow Graph.
    pub fn compute_cfg(&self, tree: &GreenTreeRoot) -> ControlFlowGraph {
        let driver = self.get_or_fallback(tree.language_id());
        driver.compute_cfg(tree)
    }

    /// Convenient top-level dispatcher: Checks invariants and returns the bitset.
    pub fn check_invariants(&self, tree: &GreenTreeRoot) -> InvariantBitset {
        let driver = self.get_or_fallback(tree.language_id());
        let mut bitset = InvariantBitset::clean();
        driver.check_language_invariants(tree, &mut bitset);
        bitset
    }
}
