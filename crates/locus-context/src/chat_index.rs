//! Deep Chat Memory Indexer and Natural Language Semantic Retrieval.
//!
//! Indexes chat transcripts, session logs, decisions, and architectural rationales
//! using Okapi BM25 with sub-10ms query performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Individual chat memory entry or conversation block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMemoryEntry {
    pub id: String,
    pub session_id: String,
    pub role: String, // "user", "assistant", "system", "decision"
    pub content: String,
    pub timestamp: u64,
    pub tags: Vec<String>,
}

/// Match result returned from deep chat memory search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMemoryMatch {
    pub entry: ChatMemoryEntry,
    pub snippet: String,
    pub score: f64,
}

/// In-memory BM25 index for conversation transcripts and architectural memory.
#[derive(Debug, Clone, Default)]
pub struct ChatMemoryIndex {
    entries: HashMap<String, ChatMemoryEntry>,
    inverted_index: HashMap<String, Vec<(String, usize)>>,
    doc_lengths: HashMap<String, usize>,
    avg_doc_length: f64,
    k1: f64,
    b: f64,
}

impl ChatMemoryIndex {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            avg_doc_length: 0.0,
            k1: 1.2,
            b: 0.75,
        }
    }

    /// Indexes a complete chat session by splitting it into semantic turn chunks.
    pub fn index_session(&mut self, session_id: &str, raw_text: &str) -> usize {
        let mut count = 0;
        let blocks = raw_text.split("\n\n");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for (idx, block) in blocks.enumerate() {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            let role = if trimmed.starts_with("User:") || trimmed.starts_with("المستخدم:") {
                "user"
            } else if trimmed.starts_with("Assistant:") || trimmed.starts_with("لوكيوس:") || trimmed.starts_with("LOCUS:") {
                "assistant"
            } else if trimmed.contains("DECISION:") || trimmed.contains("قرار:") {
                "decision"
            } else {
                "assistant"
            };

            let entry = ChatMemoryEntry {
                id: format!("{}:{}", session_id, idx),
                session_id: session_id.to_string(),
                role: role.to_string(),
                content: trimmed.to_string(),
                timestamp: now,
                tags: vec!["chat".to_string(), session_id.to_string()],
            };

            self.add_entry(entry);
            count += 1;
        }

        count
    }

    /// Adds a single memory entry and updates the inverted index.
    pub fn add_entry(&mut self, entry: ChatMemoryEntry) {
        let tokens = Self::tokenize(&entry.content);
        let len = tokens.len().max(1);

        let mut tf_map: HashMap<String, usize> = HashMap::new();
        for t in &tokens {
            *tf_map.entry(t.clone()).or_insert(0) += 1;
        }

        let id = entry.id.clone();
        for (term, count) in tf_map {
            self.inverted_index
                .entry(term)
                .or_insert_with(Vec::new)
                .push((id.clone(), count));
        }

        self.doc_lengths.insert(id.clone(), len);
        self.entries.insert(id, entry);

        // Update average document length
        let total_len: usize = self.doc_lengths.values().sum();
        self.avg_doc_length = total_len as f64 / self.entries.len().max(1) as f64;
    }

    /// Searches memory for queries using Okapi BM25 scoring in <10ms.
    pub fn search(&self, query: &str, limit: usize) -> Vec<ChatMemoryMatch> {
        let q_tokens = Self::tokenize(query);
        if q_tokens.is_empty() || self.entries.is_empty() {
            return Vec::new();
        }

        let n = self.entries.len() as f64;
        let mut scores: HashMap<String, f64> = HashMap::new();

        for q_term in &q_tokens {
            if let Some(postings) = self.inverted_index.get(q_term) {
                let df = postings.len() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.01);

                for (doc_id, tf) in postings {
                    let doc_len = self.doc_lengths.get(doc_id).copied().unwrap_or(1) as f64;
                    let tf_val = *tf as f64;
                    let num = tf_val * (self.k1 + 1.0);
                    let denom = tf_val + self.k1 * (1.0 - self.b + self.b * (doc_len / self.avg_doc_length.max(1.0)));
                    let score = idf * (num / denom);

                    *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                }
            }
        }

        let mut matches: Vec<ChatMemoryMatch> = scores
            .into_iter()
            .filter_map(|(id, score)| {
                let entry = self.entries.get(&id)?.clone();
                let snippet = Self::extract_snippet(&entry.content, &q_tokens);
                Some(ChatMemoryMatch {
                    entry,
                    snippet,
                    score,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(if limit == 0 { 10 } else { limit });
        matches
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() >= 2)
            .collect()
    }

    fn extract_snippet(text: &str, query_tokens: &[String]) -> String {
        for line in text.lines() {
            let line_lower = line.to_lowercase();
            if query_tokens.iter().any(|t| line_lower.contains(t)) {
                return line.trim().to_string();
            }
        }
        text.lines().next().unwrap_or(text).trim().to_string()
    }
}
