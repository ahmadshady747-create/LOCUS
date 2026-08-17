use anyhow::Result;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

use crate::ast_cache::{AstContextCache, CachedSymbol, FileAnalysisEntry};
use crate::embeddings::{create_embedder, EmbeddingEngine};
use crate::ignore_engine::IgnoreEngine;
use crate::types::Template;
use crate::vector_index::VectorIndex;
use crate::vector_types::{EmbeddingConfig, SemanticSearchResult, VectorDocument};

/// Summary report returned after indexing a directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDirectoryReport {
    pub total_files_scanned: usize,
    pub files_indexed: usize,
    pub files_ignored: usize,
    pub files_from_cache: usize,
    pub total_symbols_indexed: usize,
    pub duration_ms: u64,
    pub cache_hit_rate_percent: f64,
}

/// Semantic Indexer for Source Code Symbols, Templates, and Project Knowledge
/// Equipped with intelligent ignore filtering and in-memory AST SHA-256 caching.
pub struct SemanticIndexer {
    index: Arc<VectorIndex>,
    embedder: Arc<dyn EmbeddingEngine>,
    config: EmbeddingConfig,
    cache: Arc<AstContextCache>,
    ignore_engine: Arc<RwLock<IgnoreEngine>>,
}

impl SemanticIndexer {
    /// Creates a SemanticIndexer with standard default ignore filters and AST cache
    pub fn new(config: EmbeddingConfig) -> Self {
        let embedder = Arc::from(create_embedder(&config));
        Self {
            index: Arc::new(VectorIndex::new()),
            embedder,
            config,
            cache: Arc::new(AstContextCache::new()),
            ignore_engine: Arc::new(RwLock::new(IgnoreEngine::new())),
        }
    }

    /// Creates a SemanticIndexer configured for a specific workspace root,
    /// automatically discovering and loading `.gitignore` rules from that root.
    pub fn from_workspace_root<P: AsRef<Path>>(root: P, config: EmbeddingConfig) -> Self {
        let embedder = Arc::from(create_embedder(&config));
        Self {
            index: Arc::new(VectorIndex::new()),
            embedder,
            config,
            cache: Arc::new(AstContextCache::new()),
            ignore_engine: Arc::new(RwLock::new(IgnoreEngine::from_workspace_root(root))),
        }
    }

    pub fn with_shared_index(index: Arc<VectorIndex>, config: EmbeddingConfig) -> Self {
        let embedder = Arc::from(create_embedder(&config));
        Self {
            index,
            embedder,
            config,
            cache: Arc::new(AstContextCache::new()),
            ignore_engine: Arc::new(RwLock::new(IgnoreEngine::new())),
        }
    }

    pub fn with_shared_index_and_cache(
        index: Arc<VectorIndex>,
        config: EmbeddingConfig,
        cache: Arc<AstContextCache>,
        ignore_engine: Arc<RwLock<IgnoreEngine>>,
    ) -> Self {
        let embedder = Arc::from(create_embedder(&config));
        Self {
            index,
            embedder,
            config,
            cache,
            ignore_engine,
        }
    }

    /// Access the underlying vector index
    pub fn index(&self) -> Arc<VectorIndex> {
        self.index.clone()
    }

    /// Access the AST context cache
    pub fn cache(&self) -> Arc<AstContextCache> {
        self.cache.clone()
    }

    /// Access the ignore engine
    pub fn ignore_engine(&self) -> Arc<RwLock<IgnoreEngine>> {
        self.ignore_engine.clone()
    }

    /// Semantic search across all indexed code symbols, files, and templates
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SemanticSearchResult>> {
        let query_vec = self.embedder.embed(query).await?;
        let results = self.index.search(&query_vec, top_k, 0.05);
        Ok(results)
    }

    /// Index a source code file by extracting and embedding its functions, structs, and declarations.
    /// Employs ignore filtering, binary detection, and SHA-256 in-memory caching.
    pub async fn index_file(&self, file_path: &str, content: &str) -> Result<usize> {
        // Step 1: Check Ignore Engine (default ignored folders + .gitignore)
        if self.ignore_engine.read().should_ignore_str(file_path) {
            debug!("Skipping ignored path: {}", file_path);
            return Ok(0);
        }

        // Step 2: Skip binary / asset files
        if IgnoreEngine::is_binary_content(content.as_bytes()) {
            debug!("Skipping binary content: {}", file_path);
            return Ok(0);
        }

        // Step 3: Check In-Memory AST / Context Cache using SHA-256 content hash
        if let Some(cached) = self.cache.get_valid_entry(file_path, content) {
            debug!("AST Cache HIT for {}: reusing {} cached symbols", file_path, cached.symbols.len());

            // If index is empty for this file (e.g. after index reset), re-populate from cached vectors
            if self.index.document_count() == 0 || !self.is_file_in_index(file_path) {
                let mut docs = Vec::with_capacity(cached.symbols.len());
                for sym in &cached.symbols {
                    let doc = VectorDocument::new(&sym.doc_id, file_path, &sym.kind, &sym.snippet)
                        .with_symbol(&sym.name, sym.start_line, sym.end_line)
                        .with_vector(sym.vector.clone());
                    docs.push(doc);
                }
                if !docs.is_empty() {
                    self.index.upsert_batch(docs);
                }
            }

            return Ok(cached.symbols.len().max(1));
        }

        debug!("AST Cache MISS for {}: parsing AST and generating embeddings", file_path);

        // Remove prior indexed documents for this file
        self.index.remove_file(file_path);

        let chunks = self.extract_code_symbols(file_path, content);
        let content_hash = AstContextCache::compute_content_hash(content);

        if chunks.is_empty() {
            // Index the entire file as a single summary document if small
            let doc_id = format!("{}:file_summary", file_path);
            let vec = self.embedder.embed(content).await?;
            let doc = VectorDocument::new(doc_id.clone(), file_path, "FileSummary", content)
                .with_vector(vec.clone());
            self.index.upsert(doc);

            let cached_symbol = CachedSymbol {
                name: "FileSummary".to_string(),
                kind: "FileSummary".to_string(),
                start_line: 1,
                end_line: content.lines().count().max(1),
                snippet: content.to_string(),
                doc_id,
                vector: vec,
            };

            let entry = FileAnalysisEntry::new(
                file_path,
                content_hash,
                vec![cached_symbol],
                content.len() / 4,
                content.len(),
            );
            self.cache.put(file_path, entry);
            return Ok(1);
        }

        let mut docs = Vec::with_capacity(chunks.len());
        let mut cached_symbols = Vec::with_capacity(chunks.len());

        for (symbol_name, kind, start_line, end_line, chunk_text) in chunks {
            let doc_id = format!("{}:{}:{}", file_path, kind, symbol_name);
            let embedding_text = format!("{} {} in {}:\n{}", kind, symbol_name, file_path, chunk_text);
            let vec = self.embedder.embed(&embedding_text).await?;

            let doc = VectorDocument::new(doc_id.clone(), file_path, &kind, &chunk_text)
                .with_symbol(&symbol_name, start_line, end_line)
                .with_vector(vec.clone());

            cached_symbols.push(CachedSymbol {
                name: symbol_name,
                kind,
                start_line,
                end_line,
                snippet: chunk_text,
                doc_id,
                vector: vec,
            });

            docs.push(doc);
        }

        let count = docs.len();
        self.index.upsert_batch(docs);

        let entry = FileAnalysisEntry::new(
            file_path,
            content_hash,
            cached_symbols,
            content.len() / 4,
            content.len(),
        );
        self.cache.put(file_path, entry);

        debug!("Indexed {} symbols from {}", count, file_path);
        Ok(count)
    }

    /// Recursively scans and indexes all source files in a directory,
    /// respecting ignore filters, binary rules, and using AST caching.
    pub async fn index_directory(&self, root: &Path) -> Result<IndexDirectoryReport> {
        let start_time = Instant::now();
        let mut total_scanned = 0;
        let mut files_indexed = 0;
        let mut files_ignored = 0;
        let mut files_from_cache = 0;
        let mut total_symbols = 0;

        let initial_cache_hits = self.cache.stats().hits;

        // Use ignore walker to respect default ignores and .gitignore automatically
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .ignore(true)
            .build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            total_scanned += 1;

            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                if self.ignore_engine.read().should_ignore(path, true) {
                    files_ignored += 1;
                }
                continue;
            }

            if self.ignore_engine.read().should_ignore(path, false) {
                files_ignored += 1;
                continue;
            }

            // Read file content
            if let Ok(content) = std::fs::read_to_string(path) {
                let relative_path = path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let pre_hits = self.cache.stats().hits;
                let symbols = self.index_file(&relative_path, &content).await?;

                if symbols > 0 {
                    files_indexed += 1;
                    total_symbols += symbols;
                    if self.cache.stats().hits > pre_hits {
                        files_from_cache += 1;
                    }
                }
            } else {
                files_ignored += 1;
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let stats = self.cache.stats();
        let delta_hits = stats.hits.saturating_sub(initial_cache_hits);
        let total_reqs = files_indexed;
        let cache_hit_rate = if total_reqs > 0 {
            (delta_hits as f64 / total_reqs as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "Directory indexed in {}ms: {} scanned, {} indexed ({} from cache), {} symbols",
            duration_ms, total_scanned, files_indexed, files_from_cache, total_symbols
        );

        Ok(IndexDirectoryReport {
            total_files_scanned: total_scanned,
            files_indexed,
            files_ignored,
            files_from_cache,
            total_symbols_indexed: total_symbols,
            duration_ms,
            cache_hit_rate_percent: cache_hit_rate,
        })
    }

    /// Check if a file already has indexed documents in the vector index
    fn is_file_in_index(&self, file_path: &str) -> bool {
        self.index.contains_file(file_path)
    }

    /// Index a project template for smart semantic recommendation
    pub async fn index_template(&self, template: &Template) -> Result<()> {
        let doc_id = format!("template:{}", template.id);
        let text_to_embed = format!(
            "Template: {} ({}) [{}]\n{}\n{}",
            template.name,
            template.category,
            template.language,
            template.description,
            template.code
        );

        let vec = self.embedder.embed(&text_to_embed).await?;
        let doc = VectorDocument::new(doc_id, format!("template://{}", template.id), "Template", &template.code)
            .with_symbol(&template.name, 1, template.code.lines().count().max(1))
            .with_language(&template.language)
            .with_tags(template.tags.clone())
            .with_metadata("description", &template.description)
            .with_metadata("category", &template.category)
            .with_vector(vec);

        self.index.upsert(doc);
        Ok(())
    }

    /// Extract functions, methods, structs, classes, interfaces and declarations from source code
    fn extract_code_symbols(
        &self,
        file_path: &str,
        content: &str,
    ) -> Vec<(String, String, usize, usize, String)> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Multi-language signature regex
        let fn_re = Regex::new(r"(?:pub\s+)?(?:async\s+)?(?:fn|def|function)\s+([a-zA-Z0-9_]+)").unwrap();
        let struct_re = Regex::new(r"(?:pub\s+)?(?:struct|enum|class|interface|type)\s+([a-zA-Z0-9_]+)").unwrap();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            if let Some(caps) = fn_re.captures(line) {
                if let Some(name) = caps.get(1) {
                    let start_line = i + 1;
                    let end_line = (start_line + 10).min(lines.len());
                    let snippet = lines[i..end_line].join("\n");
                    symbols.push((
                        name.as_str().to_string(),
                        "Function".to_string(),
                        start_line,
                        end_line,
                        snippet,
                    ));
                }
            } else if let Some(caps) = struct_re.captures(line) {
                if let Some(name) = caps.get(1) {
                    let start_line = i + 1;
                    let end_line = (start_line + 12).min(lines.len());
                    let snippet = lines[i..end_line].join("\n");
                    symbols.push((
                        name.as_str().to_string(),
                        "TypeDefinition".to_string(),
                        start_line,
                        end_line,
                        snippet,
                    ));
                }
            }
            i += 1;
        }

        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[tokio::test]
    async fn test_semantic_indexing_and_search() {
        let config = EmbeddingConfig::default();
        let indexer = SemanticIndexer::new(config);

        let rust_code = r#"
pub async fn fetch_user_profile(user_id: Uuid) -> Result<UserProfile> {
    let query = "SELECT * FROM users WHERE id = $1";
    let profile = db.query_one(query, &[&user_id]).await?;
    Ok(profile)
}

pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub is_verified: bool,
}

pub fn render_shader_pipeline() {
    gl::draw_arrays(gl::TRIANGLES, 0, 3);
}
"#;

        let count = indexer.index_file("src/users.rs", rust_code).await.unwrap();
        assert!(count >= 2);

        // Search for user auth / profile
        let results = indexer.search("how to query user profile from db", 2).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].file_path, "src/users.rs");
        assert!(results[0].symbol_name.as_deref() == Some("fetch_user_profile") || results[0].symbol_name.as_deref() == Some("UserProfile"));
    }

    #[tokio::test]
    async fn test_ignore_engine_skips_blacklisted_directories() {
        let indexer = SemanticIndexer::new(EmbeddingConfig::default());
        let dummy_code = "pub fn dummy() {}";

        // Should return Ok(0) for ignored paths
        assert_eq!(indexer.index_file("node_modules/lodash/index.js", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file("target/debug/build.rs", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file("dist/bundle.js", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file(".git/config", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file(".venv/lib/site.py", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file("__pycache__/app.pyc", dummy_code).await.unwrap(), 0);
        assert_eq!(indexer.index_file(".locus/snapshots/01.bak", dummy_code).await.unwrap(), 0);

        // Should index valid source file
        assert!(indexer.index_file("src/lib.rs", dummy_code).await.unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_binary_files_automatically_skipped() {
        let indexer = SemanticIndexer::new(EmbeddingConfig::default());

        // Binary extension
        assert_eq!(indexer.index_file("assets/logo.png", "dummy-png-data").await.unwrap(), 0);
        assert_eq!(indexer.index_file("bin/app.exe", "dummy-exe-data").await.unwrap(), 0);

        // Binary content with null bytes
        let binary_content = String::from_utf8_lossy(b"ELF\x00\x00\x00\x00BinaryPayload");
        assert_eq!(indexer.index_file("mystery.dat", &binary_content).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_ast_cache_hit_prevents_re_embedding() {
        let indexer = SemanticIndexer::new(EmbeddingConfig::default());
        let code = "pub fn calculate_hash() -> u64 { 42 }";
        let file_path = "src/hash.rs";

        // Cold run: Miss -> Embeddings generated
        let count1 = indexer.index_file(file_path, code).await.unwrap();
        assert_eq!(count1, 1);
        assert_eq!(indexer.cache().stats().hits, 0);
        assert_eq!(indexer.cache().stats().misses, 1);

        // Warm run: Hit -> 0 re-embeddings
        let count2 = indexer.index_file(file_path, code).await.unwrap();
        assert_eq!(count2, 1);
        assert_eq!(indexer.cache().stats().hits, 1);
        assert_eq!(indexer.cache().stats().misses, 1);

        // Modified file: Miss -> Re-embedded
        let modified_code = "pub fn calculate_hash() -> u64 { 84 }";
        let count3 = indexer.index_file(file_path, modified_code).await.unwrap();
        assert_eq!(count3, 1);
        assert_eq!(indexer.cache().stats().hits, 1);
        assert_eq!(indexer.cache().stats().misses, 2);
    }

    #[tokio::test]
    async fn test_indexing_benchmark_cold_vs_warm() {
        let temp_dir = std::env::temp_dir().join(format!("locus_benchmark_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::create_dir_all(temp_dir.join("node_modules").join("fake_pkg")).unwrap();
        fs::create_dir_all(temp_dir.join("target").join("debug")).unwrap();

        // Create 20 source files in src/
        for i in 0..20 {
            let file_path = temp_dir.join("src").join(format!("module_{}.rs", i));
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "pub fn compute_value_{}() -> usize {{ {} * 10 }}", i, i).unwrap();
            writeln!(f, "pub struct Config_{} {{ pub enabled: bool }}", i).unwrap();
        }

        // Create 20 ignored files in node_modules/ and target/
        for i in 0..20 {
            let ignored_file = temp_dir.join("node_modules").join("fake_pkg").join(format!("index_{}.js", i));
            let mut f = File::create(&ignored_file).unwrap();
            writeln!(f, "module.exports = {{ id: {} }};", i).unwrap();
        }

        let indexer = SemanticIndexer::from_workspace_root(&temp_dir, EmbeddingConfig::default());

        // 1. Cold indexing run
        let cold_report = indexer.index_directory(&temp_dir).await.unwrap();
        assert_eq!(cold_report.files_indexed, 20);
        assert!(cold_report.files_ignored >= 20);
        assert_eq!(cold_report.files_from_cache, 0);

        // 2. Warm indexing run (100% cache hit)
        let warm_report = indexer.index_directory(&temp_dir).await.unwrap();
        assert_eq!(warm_report.files_indexed, 20);
        assert_eq!(warm_report.files_from_cache, 20);
        assert_eq!(warm_report.cache_hit_rate_percent, 100.0);

        // Warm indexing should be significantly faster than cold
        println!(
            "BENCHMARK RESULTS: Cold run = {}ms vs Warm run = {}ms (Speedup: {:.1}x)",
            cold_report.duration_ms.max(1),
            warm_report.duration_ms.max(1),
            cold_report.duration_ms.max(1) as f64 / warm_report.duration_ms.max(1) as f64
        );

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
