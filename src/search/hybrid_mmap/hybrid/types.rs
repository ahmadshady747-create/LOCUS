//! Domain models, search query representations, and Reciprocal Rank Fusion types.
//!
//! Provides AST-scoped filtering, hybrid query definitions, and fused search result schemas in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::types::{Language, SymbolKind};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Filtering Contracts
// ---------------------------------------------------------------------------

/// Filter restricting search candidates by syntactic symbol category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKindFilter {
    /// Allow all symbol kinds.
    All,
    /// Functions and methods only.
    FunctionsOnly,
    /// Structs, enums, classes, and type aliases.
    TypesOnly,
    /// Modules and namespaces only.
    ModulesOnly,
    /// Traits, interfaces, and abstract contracts only.
    TraitsInterfacesOnly,
}

impl Default for SymbolKindFilter {
    fn default() -> Self {
        Self::All
    }
}

impl SymbolKindFilter {
    /// Returns true if the given `SymbolKind` satisfies this filter.
    pub fn matches_kind(&self, kind: &SymbolKind) -> bool {
        match self {
            Self::All => true,
            Self::FunctionsOnly => matches!(kind, SymbolKind::Function),
            Self::TypesOnly => matches!(
                kind,
                SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias
            ),
            Self::ModulesOnly => matches!(kind, SymbolKind::Module),
            Self::TraitsInterfacesOnly => matches!(kind, SymbolKind::Trait),
        }
    }
}

/// Multi-dimensional search constraint filter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchFilter {
    /// Allowed programming languages (empty means all permitted).
    pub allowed_languages: Vec<Language>,
    /// Syntactic symbol category filter.
    pub symbol_kind: SymbolKindFilter,
    /// File path prefix filter (e.g., `src/components/`).
    pub file_path_prefix: Option<String>,
}

impl SearchFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_languages(mut self, languages: Vec<Language>) -> Self {
        self.allowed_languages = languages;
        self
    }

    pub fn with_kind(mut self, kind: SymbolKindFilter) -> Self {
        self.symbol_kind = kind;
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.file_path_prefix = Some(prefix.into());
        self
    }

    /// Evaluates whether a symbol metadata entry passes the search filter.
    pub fn matches(&self, lang: Language, kind: &SymbolKind, file_path: &str) -> bool {
        // 1. Language check
        if !self.allowed_languages.is_empty() && !self.allowed_languages.contains(&lang) {
            return false;
        }

        // 2. SymbolKind check
        if !self.symbol_kind.matches_kind(kind) {
            return false;
        }

        // 3. File path prefix check
        if let Some(prefix) = &self.file_path_prefix {
            if !file_path.starts_with(prefix) {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// RRF Configuration & Query Schema
// ---------------------------------------------------------------------------

/// Configuration parameters for the Reciprocal Rank Fusion formula.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RrfConfig {
    /// Standard smoothing constant (default: 60.0).
    pub k_constant: f32,
    /// Weight applied to lexical search ranks (default: 1.0).
    pub lexical_weight: f32,
    /// Weight applied to dense vector search ranks (default: 1.0).
    pub vector_weight: f32,
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self {
            k_constant: 60.0,
            lexical_weight: 1.0,
            vector_weight: 1.0,
        }
    }
}

/// Complete hybrid search query input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridQuery {
    /// Raw query text for BM25 and lexical keyword matching.
    pub query_text: String,
    /// Optional pre-computed query embedding vector for semantic search.
    pub query_embedding: Option<Vec<f32>>,
    /// AST-scoped and attribute filters.
    pub filter: SearchFilter,
    /// Maximum number of top candidates to return (default: 10).
    pub top_k: usize,
}

impl HybridQuery {
    pub fn new(query_text: impl Into<String>) -> Self {
        Self {
            query_text: query_text.into(),
            query_embedding: None,
            filter: SearchFilter::default(),
            top_k: 10,
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.query_embedding = Some(embedding);
        self
    }

    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }
}

// ---------------------------------------------------------------------------
// Result Models & Performance Telemetry
// ---------------------------------------------------------------------------

/// A unified ranked result produced by the Reciprocal Rank Fusion engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FusedSearchResult {
    pub symbol_id: u64,
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_kind: SymbolKind,
    pub language: Language,
    pub lexical_rank: Option<usize>,
    pub vector_rank: Option<usize>,
    pub combined_rrf_score: f32,
    pub snippet: String,
}

impl fmt::Display for FusedSearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:.4}] {} ({}) in {} (lex_rank: {:?}, vec_rank: {:?})",
            self.combined_rrf_score,
            self.symbol_name,
            self.symbol_kind,
            self.file_path,
            self.lexical_rank,
            self.vector_rank
        )
    }
}

/// Real-time execution metrics for a completed hybrid search operation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct HybridSearchStats {
    pub lexical_candidates_count: usize,
    pub vector_candidates_count: usize,
    pub fused_results_count: usize,
    pub total_latency_ms: f64,
}
