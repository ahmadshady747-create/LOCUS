use locus_core::{
    FileContent, FileEvent, FileEventKind, FileMetadata, ModificationOp, Result, SearchMatchType,
    SearchResult, StagedFileChange, SymbolInfo, SymbolKind, WorkspaceIndex,
};
use notify::{RecommendedWatcher, Watcher, Event, EventKind, RecursiveMode};
use ignore::WalkBuilder;
use globset::{Glob, GlobSetBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info, warn, error};
use regex::Regex;

pub struct FileSystemEngine {
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    event_tx: broadcast::Sender<FileEvent>,
    ignore_patterns: Vec<String>,
    glob_set: Option<globset::GlobSet>,
    index: Arc<RwLock<WorkspaceIndex>>,
    staged_changes: Arc<RwLock<HashMap<String, StagedFileChange>>>,
    snapshot_store: crate::SnapshotStore,
}

impl FileSystemEngine {
    pub fn new(root: PathBuf, ignore_patterns: Vec<String>) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(1024);
        let mut glob_builder = GlobSetBuilder::new();
        for pattern in &ignore_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                glob_builder.add(glob);
            }
        }
        let glob_set = glob_builder.build().ok();

        Ok(Self {
            root: root.clone(),
            watcher: None,
            event_tx,
            ignore_patterns,
            glob_set,
            index: Arc::new(RwLock::new(WorkspaceIndex::new(root))),
            staged_changes: Arc::new(RwLock::new(HashMap::new())),
            snapshot_store: crate::SnapshotStore::new(),
        })
    }

    pub async fn scan_workspace(&self) -> Result<WorkspaceIndex> {
        info!("Scanning workspace: {}", self.root.display());
        let mut index = WorkspaceIndex::new(self.root.clone());
        let walker = WalkBuilder::new(&self.root)
            .ignore(true)
            .git_ignore(true)
            .hidden(false)
            .build();

        let mut file_count = 0;
        let mut total_size = 0u64;

        for entry in walker {
            let entry = entry.map_err(|e| locus_core::LocusError::FileSystem(e.to_string()))?;
            let path = entry.path();

            if self.should_ignore(path) {
                continue;
            }

            if path.is_file() {
                if let Ok(metadata) = self.extract_metadata(path).await {
                    total_size += metadata.size;
                    index.files.insert(path.to_path_buf(), metadata);
                    file_count += 1;
                }
            }
        }

        index.total_files = file_count;
        index.total_size = total_size;
        index.updated_at = chrono::Utc::now();

        *self.index.write().await = index.clone();
        info!("Scanned {} files, {} bytes", file_count, total_size);
        Ok(index)
    }

    async fn extract_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = tokio::fs::metadata(path).await?;
        let size = metadata.len();
        let modified: chrono::DateTime<chrono::Utc> = metadata.modified()?.into();

        let content = tokio::fs::read(path).await?;
        let is_binary = content.iter().any(|&b| b == 0);
        let hash = blake3::hash(&content).to_hex().to_string();

        let language = self.detect_language(path);
        let symbols = if !is_binary {
            self.extract_symbols(path, &content).await.unwrap_or_default()
        } else {
            vec![]
        };

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size,
            modified,
            hash,
            language,
            symbols,
            is_binary,
        })
    }

    fn detect_language(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "py" => "python",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" | "hpp" | "h" => "cpp",
                "c" => "c",
                "cs" => "csharp",
                "rb" => "ruby",
                "php" => "php",
                "swift" => "swift",
                "kt" | "kts" => "kotlin",
                "scala" => "scala",
                "clj" => "clojure",
                "hs" => "haskell",
                "ml" => "ocaml",
                "fs" => "fsharp",
                "lua" => "lua",
                "pl" => "perl",
                "r" => "r",
                "jl" => "julia",
                "dart" => "dart",
                "zig" => "zig",
                "nim" => "nim",
                "v" => "vlang",
                "cr" => "crystal",
                "ex" | "exs" => "elixir",
                "erl" => "erlang",
                "elm" => "elm",
                "purs" => "purescript",
                "sql" => "sql",
                "sh" | "bash" | "zsh" => "shell",
                "ps1" => "powershell",
                "bat" | "cmd" => "batch",
                "dockerfile" => "dockerfile",
                "toml" => "toml",
                "yaml" | "yml" => "yaml",
                "json" => "json",
                "xml" => "xml",
                "html" => "html",
                "css" => "css",
                "scss" | "sass" => "scss",
                "less" => "less",
                "md" => "markdown",
                "txt" => "text",
                _ => "unknown",
            }.to_string())
    }

    async fn extract_symbols(&self, path: &Path, content: &[u8]) -> Result<Vec<SymbolInfo>> {
        let text = String::from_utf8_lossy(content);
        let language = self.detect_language(path).unwrap_or_default();
        let mut symbols = Vec::new();

        let patterns: Vec<(SymbolKind, &str)> = match language.as_str() {
            "rust" => vec![
                (SymbolKind::Function, r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)"),
                (SymbolKind::Struct, r"(?:pub\s+)?struct\s+(\w+)"),
                (SymbolKind::Enum, r"(?:pub\s+)?enum\s+(\w+)"),
                (SymbolKind::Trait, r"(?:pub\s+)?trait\s+(\w+)"),
                (SymbolKind::Type, r"(?:pub\s+)?type\s+(\w+)"),
                (SymbolKind::Const, r"(?:pub\s+)?const\s+(\w+)"),
                (SymbolKind::Module, r"(?:pub\s+)?mod\s+(\w+)"),
            ],
            "typescript" | "javascript" => vec![
                (SymbolKind::Function, r"(?:export\s+)?(?:async\s+)?function\s+(\w+)"),
                (SymbolKind::Function, r"(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s+)?\("),
                (SymbolKind::Class, r"(?:export\s+)?class\s+(\w+)"),
                (SymbolKind::Interface, r"(?:export\s+)?interface\s+(\w+)"),
                (SymbolKind::Type, r"(?:export\s+)?type\s+(\w+)"),
                (SymbolKind::Const, r"(?:export\s+)?const\s+(\w+)"),
            ],
            "python" => vec![
                (SymbolKind::Function, r"^\s*def\s+(\w+)"),
                (SymbolKind::Class, r"^\s*class\s+(\w+)"),
                (SymbolKind::Const, r"^\s*(\w+)\s*="),
            ],
            "go" => vec![
                (SymbolKind::Function, r"^\s*func\s+(?:\([^)]*\)\s+)?(\w+)"),
                (SymbolKind::Struct, r"^\s*type\s+(\w+)\s+struct"),
                (SymbolKind::Interface, r"^\s*type\s+(\w+)\s+interface"),
            ],
            _ => vec![],
        };

        for (line_num, line) in text.lines().enumerate() {
            for (kind, pattern) in &patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if let Some(caps) = re.captures(line) {
                        if let Some(name) = caps.get(1) {
                            symbols.push(SymbolInfo {
                                name: name.as_str().to_string(),
                                kind: kind.clone(),
                                line: line_num + 1,
                                column: name.start() + 1,
                                signature: Some(line.trim().to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(symbols)
    }

    fn should_ignore(&self, path: &Path) -> bool {
        if let Some(glob_set) = &self.glob_set {
            if glob_set.is_match(path) {
                return true;
            }
        }

        let path_str = path.to_string_lossy();
        for pattern in &self.ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    pub async fn read_file(&self, path: &Path) -> Result<FileContent> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        let content = tokio::fs::read_to_string(&full_path).await?;
        Ok(FileContent::new(full_path, content))
    }

    pub async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, content).await?;
        self.invalidate_cache(&full_path).await;
        Ok(())
    }

    pub async fn modify_file(&self, path: &Path, ops: &[ModificationOp]) -> Result<()> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        let content = tokio::fs::read_to_string(&full_path).await?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        for op in ops {
            match op {
                ModificationOp::Insert { line, column, text } => {
                    let line_idx = line.saturating_sub(1);
                    if line_idx < lines.len() {
                        let line_content = &mut lines[line_idx];
                        let col_idx = column.saturating_sub(1).min(line_content.len());
                        line_content.insert_str(col_idx, text);
                    } else if line_idx == lines.len() {
                        lines.push(text.to_string());
                    }
                }
                ModificationOp::Delete { start_line, start_column, end_line, end_column } => {
                    let start_idx = start_line.saturating_sub(1);
                    let end_idx = end_line.saturating_sub(1);
                    if start_idx < lines.len() && end_idx < lines.len() {
                        let start_col = start_column.saturating_sub(1);
                        let end_col = end_column.saturating_sub(1);
                        if start_idx == end_idx {
                            if let Some(line) = lines.get_mut(start_idx) {
                                let end = end_col.min(line.len());
                                line.drain(start_col..end);
                            }
                        } else {
                            if let Some(line) = lines.get_mut(start_idx) {
                                line.drain(start_col..);
                            }
                            for i in (start_idx + 1)..end_idx {
                                lines[i].clear();
                            }
                            if let Some(line) = lines.get_mut(end_idx) {
                                line.drain(..end_col.min(line.len()));
                            }
                        }
                    }
                }
                ModificationOp::Replace { start_line, start_column, end_line, end_column, text } => {
                    let start_idx = start_line.saturating_sub(1);
                    let end_idx = end_line.saturating_sub(1);
                    if start_idx < lines.len() && end_idx < lines.len() {
                        let start_col = start_column.saturating_sub(1);
                        let end_col = end_column.saturating_sub(1);
                        if start_idx == end_idx {
                            if let Some(line) = lines.get_mut(start_idx) {
                                let end = end_col.min(line.len());
                                line.replace_range(start_col..end, text);
                            }
                        } else {
                            if let Some(line) = lines.get_mut(start_idx) {
                                line.drain(start_col..);
                                line.push_str(text);
                            }
                            for i in (start_idx + 1)..end_idx {
                                lines[i].clear();
                            }
                            if let Some(line) = lines.get_mut(end_idx) {
                                line.drain(..end_col.min(line.len()));
                            }
                        }
                    }
                }
            }
        }

        let new_content = lines.join("\n");
        tokio::fs::write(&full_path, new_content).await?;
        self.invalidate_cache(&full_path).await;
        Ok(())
    }

    async fn invalidate_cache(&self, path: &Path) {
        let mut index = self.index.write().await;
        if let Some(meta) = index.files.get_mut(path) {
            if let Ok(new_meta) = self.extract_metadata(path).await {
                *meta = new_meta;
            }
        }
        index.updated_at = chrono::Utc::now();
    }

    pub async fn stage_change(&self, path: &Path, proposed_content: &str) -> Result<StagedFileChange> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        let original_content = if full_path.exists() {
            tokio::fs::read_to_string(&full_path).await.unwrap_or_default()
        } else {
            String::new()
        };

        let change_id = uuid::Uuid::new_v4().to_string();
        let change = StagedFileChange {
            change_id: change_id.clone(),
            file_path: full_path,
            original_content,
            proposed_content: proposed_content.to_string(),
            created_at: chrono::Utc::now(),
        };

        let mut staged = self.staged_changes.write().await;
        staged.insert(change_id, change.clone());
        Ok(change)
    }

    pub async fn accept_change(&self, change_id: &str) -> Result<()> {
        let change = {
            let mut staged = self.staged_changes.write().await;
            staged.remove(change_id)
        };

        if let Some(c) = change {
            // 1. Capture atomic snapshot before overwrite
            let _ = self
                .snapshot_store
                .capture_snapshot(
                    &self.root,
                    &c.file_path,
                    &c.original_content,
                    &format!("Before accepting full change {}", change_id),
                )
                .await;

            // 2. Apply proposed content
            self.write_file(&c.file_path, &c.proposed_content).await?;
            info!("Accepted and applied change {} to {}", change_id, c.file_path.display());
            Ok(())
        } else {
            Err(locus_core::LocusError::NotFound(format!("Staged change {} not found", change_id)))
        }
    }

    pub async fn accept_hunk(&self, change_id: &str, hunk_id: &str) -> Result<Option<StagedFileChange>> {
        let mut staged_guard = self.staged_changes.write().await;
        let change = staged_guard.get(change_id).cloned().ok_or_else(|| {
            locus_core::LocusError::NotFound(format!("Staged change {} not found", change_id))
        })?;

        // 1. Compute current hunks
        let hunks = crate::diff_engine::compute_hunks(&change.original_content, &change.proposed_content);
        let target_hunk = hunks.iter().find(|h| h.hunk_id == hunk_id).ok_or_else(|| {
            locus_core::LocusError::NotFound(format!("Hunk {} not found in change {}", hunk_id, change_id))
        })?;

        // 2. Capture snapshot of current file on disk
        let _ = self
            .snapshot_store
            .capture_snapshot(
                &self.root,
                &change.file_path,
                &change.original_content,
                &format!("Before applying hunk {} to {}", hunk_id, change.file_path.display()),
            )
            .await;

        // 3. Apply only the single hunk to the original content
        let new_original = crate::diff_engine::apply_single_hunk(&change.original_content, target_hunk);

        // 4. Write new baseline to disk
        self.write_file(&change.file_path, &new_original).await?;

        // 5. Recompute remaining hunks against proposed to prevent offset drift
        let remaining_hunks = crate::diff_engine::compute_hunks(&new_original, &change.proposed_content);

        if remaining_hunks.is_empty() || new_original.trim() == change.proposed_content.trim() {
            // All changes accepted! Remove staged entry
            staged_guard.remove(change_id);
            info!("All hunks accepted for {}. Change completed.", change_id);
            Ok(None)
        } else {
            // Update in-place with new baseline
            let updated = StagedFileChange {
                original_content: new_original,
                ..change
            };
            staged_guard.insert(change_id.to_string(), updated.clone());
            info!("Hunk {} accepted for {}. Remaining hunks: {}", hunk_id, change_id, remaining_hunks.len());
            Ok(Some(updated))
        }
    }

    pub async fn reject_hunk(&self, change_id: &str, hunk_id: &str) -> Result<Option<StagedFileChange>> {
        let mut staged_guard = self.staged_changes.write().await;
        let change = staged_guard.get(change_id).cloned().ok_or_else(|| {
            locus_core::LocusError::NotFound(format!("Staged change {} not found", change_id))
        })?;

        // 1. Compute current hunks
        let hunks = crate::diff_engine::compute_hunks(&change.original_content, &change.proposed_content);
        let target_hunk = hunks.iter().find(|h| h.hunk_id == hunk_id).ok_or_else(|| {
            locus_core::LocusError::NotFound(format!("Hunk {} not found in change {}", hunk_id, change_id))
        })?;

        // 2. Reject the single hunk by discarding its proposed change
        let new_proposed = crate::diff_engine::reject_single_hunk(
            &change.original_content,
            &change.proposed_content,
            target_hunk,
        );

        // 3. Recompute remaining hunks
        let remaining_hunks = crate::diff_engine::compute_hunks(&change.original_content, &new_proposed);

        if remaining_hunks.is_empty() || change.original_content.trim() == new_proposed.trim() {
            // All hunks rejected/discarded! Remove staged entry
            staged_guard.remove(change_id);
            info!("All hunks rejected for {}. Change discarded.", change_id);
            Ok(None)
        } else {
            let updated = StagedFileChange {
                proposed_content: new_proposed,
                ..change
            };
            staged_guard.insert(change_id.to_string(), updated.clone());
            info!("Hunk {} rejected for {}. Remaining hunks: {}", hunk_id, change_id, remaining_hunks.len());
            Ok(Some(updated))
        }
    }

    pub async fn reject_change(&self, change_id: &str) -> Result<()> {
        let change = {
            let mut staged = self.staged_changes.write().await;
            staged.remove(change_id)
        };

        if let Some(c) = change {
            // Restore original content if previous file existed
            if !c.original_content.is_empty() {
                self.write_file(&c.file_path, &c.original_content).await?;
            }
            info!("Rejected change {} and restored {}", change_id, c.file_path.display());
            Ok(())
        } else {
            Err(locus_core::LocusError::NotFound(format!("Staged change {} not found", change_id)))
        }
    }

    pub async fn rollback_last(&self) -> Result<locus_core::types::RollbackResult> {
        self.snapshot_store.rollback_last().await
    }

    pub async fn list_snapshots(&self) -> Vec<locus_core::types::FileSnapshot> {
        self.snapshot_store.list_snapshots().await
    }

    pub fn compute_hunks(&self, original: &str, proposed: &str) -> Vec<locus_core::types::DiffHunk> {
        crate::diff_engine::compute_hunks(original, proposed)
    }

    pub async fn list_staged_changes(&self) -> Vec<StagedFileChange> {
        let staged = self.staged_changes.read().await;
        staged.values().cloned().collect()
    }

    /// Parses search/replace blocks from input text and applies them to the specified file, capturing snapshot
    pub async fn apply_search_replace(
        &self,
        path: &Path,
        content_with_blocks: &str,
    ) -> Result<(String, usize)> {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        let original = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| locus_core::LocusError::FileSystem(format!("Failed to read file for search/replace: {}", e)))?;

        let blocks = crate::search_replace::parse_search_replace_blocks(content_with_blocks);
        if blocks.is_empty() {
            return Err(locus_core::LocusError::InvalidInput(
                "No valid <<<<<<< SEARCH ... ======= ... >>>>>>> REPLACE blocks found in text".to_string(),
            ));
        }

        let (new_content, applied_count) = crate::search_replace::apply_search_replace_blocks(&original, &blocks)
            .map_err(|e| locus_core::LocusError::InvalidInput(e))?;

        // Capture snapshot before write
        let _ = self.snapshot_store.capture_snapshot(&self.root, &full_path, &original, "Before applying search/replace").await;

        self.write_file(path, &new_content).await?;
        Ok((new_content, applied_count))
    }

    pub fn watch(&self, paths: &[PathBuf]) -> Result<BroadcastStream<FileEvent>> {
        let (tx, rx) = broadcast::channel(1024);
        let mut watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let kind = match event.kind {
                            EventKind::Create(_) => FileEventKind::Created,
                            EventKind::Modify(_) => FileEventKind::Modified,
                            EventKind::Remove(_) => FileEventKind::Deleted,
                            EventKind::Other => continue,
                            _ => continue,
                        };
                        let _ = tx.send(FileEvent {
                            path,
                            kind,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            },
            notify::Config::default(),
        )?;

        for path in paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        Ok(BroadcastStream::new(rx))
    }

    pub fn events(&self) -> BroadcastStream<FileEvent> {
        BroadcastStream::new(self.event_tx.subscribe())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let index = self.index.read().await;
        let mut results = Vec::new();
        let regex = Regex::new(&regex::escape(query)).ok();

        for (path, meta) in &index.files {
            if meta.is_binary {
                continue;
            }

            if let Ok(content) = tokio::fs::read_to_string(path).await {
                for (line_num, line) in content.lines().enumerate() {
                    let mut matched = false;
                    let mut match_type = SearchMatchType::Exact;

                    if line.contains(query) {
                        matched = true;
                    } else if let Some(re) = &regex {
                        if re.is_match(line) {
                            matched = true;
                            match_type = SearchMatchType::Regex;
                        }
                    } else {
                        let query_lower = query.to_lowercase();
                        let line_lower = line.to_lowercase();
                        if line_lower.contains(&query_lower) {
                            matched = true;
                            match_type = SearchMatchType::Fuzzy;
                        }
                    }

                    if matched {
                        for symbol in &meta.symbols {
                            if symbol.name.to_lowercase().contains(&query.to_lowercase()) {
                                match_type = SearchMatchType::Symbol;
                                break;
                            }
                        }

                        let start = line_num.saturating_sub(2);
                        let end = (line_num + 3).min(content.lines().count());
                        let context_lines: Vec<&str> = content.lines().skip(start).take(end - start).collect();
                        let context = context_lines.join("\n");

                        results.push(SearchResult {
                            path: path.clone(),
                            line: line_num + 1,
                            column: line.find(query).unwrap_or(0) + 1,
                            context,
                            match_type: match_type.clone(),
                        });
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            b.match_type.cmp(&a.match_type)
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line.cmp(&b.line))
        });

        Ok(results)
    }

    pub async fn get_index(&self) -> WorkspaceIndex {
        self.index.read().await.clone()
    }
}

impl Drop for FileSystemEngine {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
        }
    }
}