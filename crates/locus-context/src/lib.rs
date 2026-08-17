mod error_context;
mod prompt_builder;
pub mod types;
pub mod vector_types;
pub mod embeddings;
pub mod vector_index;
pub mod semantic_indexer;
pub mod ignore_engine;
pub mod ast_cache;
pub mod skeletonizer;
pub mod adr_ledger;
pub mod symbol_graph;
pub mod bm25;
pub mod mentions;
pub mod fim_engine;
pub mod omni_search;
pub mod chat_index;

pub use ast_cache::{AstContextCache, CachedSymbol, FileAnalysisEntry, CacheStats};
pub use skeletonizer::{extract_skeleton, calculate_skeleton_savings, SkeletonStats};
pub use ignore_engine::{IgnoreEngine, DEFAULT_IGNORED_DIRS, BINARY_EXTENSIONS};
pub use embeddings::{create_embedder, EmbeddingEngine, SubwordHashingEmbedder, OllamaEmbedder};
pub use vector_index::{cosine_similarity, VectorIndex};
pub use semantic_indexer::{SemanticIndexer, IndexDirectoryReport};
pub use vector_types::{EmbeddingConfig, EmbeddingProvider, SemanticSearchResult, VectorDocument};
pub use adr_ledger::{
    AdrLedger, AdrLedgerManager, AdrRecord, DecisionKind, NegativeMemoryEntry, NegativeSeverity,
};
pub use symbol_graph::SymbolGraph;
pub use bm25::Bm25Engine;
pub use mentions::{resolve_mention_query, MentionCandidate};
pub use fim_engine::{
    format_fim_prompt, get_fim_stop_tokens, truncate_cursor_context, FimCompletionRequest,
    FimCompletionResponse, FimTemplateFormat,
};
pub use omni_search::{OmniSearchEngine, OmniSearchResult};
pub use chat_index::{ChatMemoryEntry, ChatMemoryIndex, ChatMemoryMatch};
pub use types::{
    Bm25Document, Bm25SearchResult, HybridContextPayload, SymbolEdge, SymbolKind, SymbolNode,
};

use crate::types::{AssembledContext, PromptSection, Template, ErrorLog};
use anyhow::Result;
use tiktoken_rs::{get_bpe_from_model, CoreBPE};

pub struct ContextAssembler {
    tokenizer: Option<CoreBPE>,
    max_tokens: usize,
}

impl ContextAssembler {
    pub fn new() -> Result<Self> {
        let tokenizer = get_bpe_from_model("gpt-4").ok();
        Ok(Self {
            tokenizer,
            max_tokens: 32000,
        })
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn assemble(&self, user_request: &str, templates: Vec<Template>) -> String {
        self.assemble_with_errors(user_request, templates, vec![])
    }

    pub fn assemble_with_errors(
        &self,
        user_request: &str,
        templates: Vec<Template>,
        errors: Vec<ErrorLog>,
    ) -> String {
        let sections = prompt_builder::build_prompt_sections(user_request, &templates, &errors);
        self.build_full_prompt(sections)
    }

    pub fn assemble_detailed(
        &self,
        user_request: &str,
        templates: Vec<Template>,
        errors: Vec<ErrorLog>,
    ) -> AssembledContext {
        let sections = prompt_builder::build_prompt_sections(user_request, &templates, &errors);
        let full_prompt = self.build_full_prompt(sections.clone());

        let system_prompt = prompt_builder::build_system_prompt(user_request);
        let user_prompt = prompt_builder::build_user_prompt(user_request, &templates);
        let template_context = if templates.is_empty() {
            String::new()
        } else {
            prompt_builder::build_template_context(&templates)
        };
        let error_context = if errors.is_empty() {
            String::new()
        } else {
            error_context::format_errors_for_prompt(errors)
        };

        let token_estimate = self.estimate_tokens(&full_prompt);

        AssembledContext {
            system_prompt,
            user_prompt,
            template_context,
            error_context,
            full_prompt,
            token_estimate,
        }
    }

    fn build_full_prompt(&self, mut sections: Vec<PromptSection>) -> String {
        sections.sort_by_key(|s| -s.priority);

        let mut included = Vec::new();
        let mut current_tokens = 0;

        for section in sections {
            let section_tokens = section.token_estimate;
            if current_tokens + section_tokens > self.max_tokens {
                if included.is_empty() {
                    included.push(section);
                }
                break;
            }
            current_tokens += section_tokens;
            included.push(section);
        }

        included
            .into_iter()
            .map(|s| s.content)
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    pub fn estimate_tokens(&self, text: &str) -> usize {
        if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.encode_with_special_tokens(text).len()
        } else {
            (text.len() as f32 / 3.5).ceil() as usize
        }
    }

    pub fn fits_in_context(&self, text: &str) -> bool {
        self.estimate_tokens(text) <= self.max_tokens
    }

    pub fn truncate_to_fit(&self, text: &str) -> String {
        if self.fits_in_context(text) {
            return text.to_string();
        }

        let tokens = if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.encode_with_special_tokens(text)
        } else {
            return text.chars().take(self.max_tokens * 3).collect();
        };

        if tokens.len() <= self.max_tokens {
            return text.to_string();
        }

        if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.decode(tokens[..self.max_tokens].to_vec()).unwrap_or_default()
        } else {
            text.chars().take(self.max_tokens * 3).collect()
        }
    }

    pub fn truncate_to_fit_max(&self, text: &str, max_tokens: usize) -> String {
        if max_tokens == 0 {
            return String::new();
        }

        let tokens = if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.encode_with_special_tokens(text)
        } else {
            return text.chars().take(max_tokens * 3).collect();
        };

        if tokens.len() <= max_tokens {
            return text.to_string();
        }

        if let Some(ref tokenizer) = self.tokenizer {
            tokenizer.decode(tokens[..max_tokens].to_vec()).unwrap_or_default()
        } else {
            text.chars().take(max_tokens * 3).collect()
        }
    }
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            tokenizer: None,
            max_tokens: 32000,
        })
    }
}

// ============================================================================
// Hybrid Context Engine (In-Memory SymbolGraph + BM25 + Skeletonizer)
// ============================================================================

use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub struct HybridContextEngine {
    pub symbol_graph: Arc<RwLock<SymbolGraph>>,
    pub bm25: Arc<RwLock<Bm25Engine>>,
}

impl Default for HybridContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridContextEngine {
    pub fn new() -> Self {
        Self {
            symbol_graph: Arc::new(RwLock::new(SymbolGraph::new())),
            bm25: Arc::new(RwLock::new(Bm25Engine::new())),
        }
    }

    /// Indexes a file into both the SymbolGraph and the BM25 inverted index.
    pub fn index_file(&self, path: &Path, content: &str) {
        self.symbol_graph.write().index_file(path, content);

        let path_str = path.to_string_lossy().replace('\\', "/");
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());

        let doc = Bm25Document {
            id: path_str.clone(),
            file_path: path_str,
            title,
            content: content.to_string(),
            token_count: content.len() / 4,
        };
        self.bm25.write().add_document(doc);
    }

    /// Queries symbols by name.
    pub fn query_symbol(&self, name: &str) -> Vec<SymbolNode> {
        self.symbol_graph.read().find_symbol(name)
    }

    /// Performs sub-5ms BM25 lexical ranking search.
    pub fn bm25_search(&self, query: &str, limit: usize) -> Vec<Bm25SearchResult> {
        self.bm25.read().search(query, limit)
    }

    /// Builds a dense, token-optimized hybrid context payload.
    pub fn build_hybrid_context(
        &self,
        prompt: &str,
        target_files: &[PathBuf],
        _max_tokens: usize,
    ) -> HybridContextPayload {
        let start = Instant::now();
        let mut symbols = Vec::new();

        // 1. Gather symbols from target files & related imports
        {
            let graph = self.symbol_graph.read();
            for file in target_files {
                let file_syms = graph.resolve_symbol_context("", file);
                for s in file_syms {
                    if !symbols.iter().any(|existing: &SymbolNode| {
                        existing.name == s.name && existing.file_path == s.file_path
                    }) {
                        symbols.push(s);
                    }
                }
            }
        }

        // 2. Perform BM25 search for the prompt
        let bm25_results = self.bm25.read().search(prompt, 5);

        // 3. Construct Dense Skeleton Context
        let mut sections = Vec::new();

        // Add Symbol Signatures Section
        if !symbols.is_empty() {
            let mut sym_text = String::from("### Relevant Symbol Signatures\n```\n");
            for sym in symbols.iter().take(20) {
                sym_text.push_str(&format!(
                    "// [{}] {}\n{}\n\n",
                    sym.file_path,
                    sym.kind.display_name(),
                    sym.signature
                ));
            }
            sym_text.push_str("```\n");
            sections.push(sym_text);
        }

        // Add Top BM25 File Skeletons Section
        if !bm25_results.is_empty() {
            let mut bm25_text = String::from("### Codebase Snippets (BM25 Lexical Matches)\n");
            for res in &bm25_results {
                let ext = Path::new(&res.file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let skeleton = extract_skeleton(&res.snippet, ext);
                bm25_text.push_str(&format!(
                    "- **{}** (score: {:.2}):\n  `{}`\n",
                    res.file_path, res.score, skeleton
                ));
            }
            sections.push(bm25_text);
        }

        let dense_context = sections.join("\n\n");
        let token_estimate = (dense_context.len() + 3) / 4;
        let latency_ms = start.elapsed().as_millis() as u64;

        HybridContextPayload {
            query: prompt.to_string(),
            symbols,
            bm25_results,
            dense_context,
            token_estimate,
            latency_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_core::types::SecurityLevel;

    fn sample_template() -> Template {
        Template::new(
            "auth/jwt",
            "auth",
            "JWT Implementation",
            "RS256 JWT with refresh tokens",
            "fn create_token() -> String { todo!() }",
            "rust",
        ).with_security_level(SecurityLevel::ReviewRequired)
    }

    #[test]
    fn test_assemble_basic() {
        let assembler = ContextAssembler::new().unwrap();
        let request = "Add JWT auth to my API";
        let templates = vec![sample_template()];
        
        let result = assembler.assemble(request, templates);
        
        assert!(result.contains("LOCUS"));
        assert!(result.contains("JWT"));
        assert!(result.contains("create_token"));
    }

    #[test]
    fn test_assemble_with_errors() {
        let assembler = ContextAssembler::new().unwrap();
        let request = "Fix the JWT validation";
        let templates = vec![sample_template()];
        let errors = vec![
            ErrorLog::new("test-agent", "Invalid signature")
                .with_context("token validation failed"),
        ];
        
        let result = assembler.assemble_with_errors(request, templates, errors);
        
        assert!(result.contains("Previous Errors"));
        assert!(result.contains("Invalid signature"));
    }

    #[test]
    fn test_token_estimation() {
        let assembler = ContextAssembler::new().unwrap();
        let short = "hello";
        let long = "a ".repeat(10000);
        
        assert!(assembler.estimate_tokens(short) < 10);
        assert!(assembler.estimate_tokens(&long) > 5000);
    }

    #[test]
    fn test_truncate() {
        let assembler = ContextAssembler::new().unwrap().with_max_tokens(100);
        let long = "word ".repeat(1000);
        
        let truncated = assembler.truncate_to_fit(&long);
        assert!(assembler.estimate_tokens(&truncated) <= 100);
    }

    #[test]
    fn test_hybrid_context_engine() {
        let engine = HybridContextEngine::new();
        let rs_code = r#"
            pub struct KeyVaultStore {
                pub keys: HashMap<String, String>,
            }

            pub fn encrypt_secret(secret: &str) -> Vec<u8> {
                secret.as_bytes().to_vec()
            }
        "#;
        engine.index_file(Path::new("src/keyring.rs"), rs_code);

        let syms = engine.query_symbol("KeyVaultStore");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Struct);

        let bm25_res = engine.bm25_search("encrypt secret", 5);
        assert!(!bm25_res.is_empty());

        let payload = engine.build_hybrid_context("encrypt secret", &[PathBuf::from("src/keyring.rs")], 4000);
        assert!(!payload.symbols.is_empty());
        assert!(!payload.dense_context.is_empty());
    }
}