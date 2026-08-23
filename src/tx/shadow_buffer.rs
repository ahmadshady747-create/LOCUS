//! In-Memory Staging Layer with Rollback Journal.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use crate::types::{Language, TxStagedFile};

/// In-memory shadow buffer storing staged file modifications prior to disk commit.
pub struct ShadowBuffer {
    staged: HashMap<String, TxStagedFile>,
    journal: Vec<String>, // ordered list of staged file paths
}

impl Default for ShadowBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadowBuffer {
    pub fn new() -> Self {
        Self {
            staged: HashMap::new(),
            journal: Vec::new(),
        }
    }

    /// Stage a file edit in memory, capturing original disk content if available.
    pub fn stage(&mut self, path: &str, staged_content: &str, language: Language) {
        let original_content = if std::path::Path::new(path).exists() {
            std::fs::read_to_string(path).ok()
        } else {
            None
        };

        let file = TxStagedFile {
            path: path.to_string(),
            original_content,
            staged_content: staged_content.to_string(),
            language,
        };

        if !self.staged.contains_key(path) {
            self.journal.push(path.to_string());
        }
        self.staged.insert(path.to_string(), file);
    }

    /// Get reference to a staged file.
    pub fn get_staged(&self, path: &str) -> Option<&TxStagedFile> {
        self.staged.get(path)
    }

    /// Get all staged files.
    pub fn all_staged(&self) -> Vec<&TxStagedFile> {
        self.journal.iter().filter_map(|p| self.staged.get(p)).collect()
    }

    /// Number of staged files.
    pub fn len(&self) -> usize {
        self.staged.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.staged.is_empty()
    }

    /// Clear all staged edits and journal.
    pub fn clear(&mut self) {
        self.staged.clear();
        self.journal.clear();
    }
}
