//! locus-research — Lightweight semantic documentation extraction, official registry lookups,
//! and compiler error resolution radar for the LOCUS autonomous OS.

pub mod extractor;
pub mod issues_resolver;
pub mod registry;
pub mod types;

use anyhow::Result;
pub use extractor::{DocsCacheManager, SemanticExtractor};
pub use issues_resolver::IssueResolverRadar;
pub use registry::RegistryDispatcher;
pub use types::{
    CompilerErrorDiagnostic, DocQuery, DocSearchResult, DocSection, Ecosystem, PackageMetadata,
    ResolvedErrorSolution,
};

pub struct DocSearchEngine {
    dispatcher: RegistryDispatcher,
}

impl Default for DocSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DocSearchEngine {
    pub fn new() -> Self {
        Self {
            dispatcher: RegistryDispatcher::new(),
        }
    }

    /// Searches official registries or cached markdown documentation for a given query.
    pub async fn search_docs(&self, query: DocQuery) -> Result<DocSearchResult> {
        // 1. Check local docs cache first
        if let Some(cached) = DocsCacheManager::get_cached(&query) {
            return Ok(cached);
        }

        // 2. Fetch from official registry
        let result = self.dispatcher.fetch_package_doc(&query).await?;

        // 3. Store in local docs cache
        let _ = DocsCacheManager::store_cache(&query, &result);

        Ok(result)
    }

    /// Analyzes a raw compiler or runtime error message and provides structured fixes and ADR negative memory.
    pub fn resolve_error(&self, snippet: &str) -> ResolvedErrorSolution {
        IssueResolverRadar::resolve_error(snippet)
    }

    /// Clears the local documentation cache.
    pub fn clear_cache(&self) -> Result<u32> {
        DocsCacheManager::clear_cache()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_search_engine_error_resolution() {
        let engine = DocSearchEngine::new();
        let res = engine.resolve_error("error[E0502]: cannot borrow `data` as mutable because it is also borrowed as immutable");
        assert_eq!(res.error_code, "E0502");
        assert_eq!(res.language, "Rust");
    }
}
