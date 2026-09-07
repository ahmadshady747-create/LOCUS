//! BM25 & Lexical Inverted Index Matcher.
//!
//! Provides tokenization, term frequency (TF), inverse document frequency (IDF),
//! and AST-scoped lexical keyword search in 100% pure safe Rust.

#![forbid(unsafe_code)]

use crate::search::hybrid_mmap::hybrid::types::SearchFilter;
use crate::types::{Language, SymbolKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// An indexed symbol document ready for BM25 and lexical retrieval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolDocument {
    pub symbol_id: u64,
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_kind: SymbolKind,
    pub language: Language,
    pub signature: String,
    pub doc_comment: String,
    pub body_snippet: String,
}

impl SymbolDocument {
    pub fn new(
        symbol_id: u64,
        file_path: impl Into<String>,
        symbol_name: impl Into<String>,
        symbol_kind: SymbolKind,
        language: Language,
        signature: impl Into<String>,
        doc_comment: impl Into<String>,
        body_snippet: impl Into<String>,
    ) -> Self {
        Self {
            symbol_id,
            file_path: file_path.into(),
            symbol_name: symbol_name.into(),
            symbol_kind,
            language,
            signature: signature.into(),
            doc_comment: doc_comment.into(),
            body_snippet: body_snippet.into(),
        }
    }
}

/// A lexical candidate hit produced by `LexicalKeywordMatcher`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexicalCandidate {
    pub symbol_id: u64,
    pub file_path: String,
    pub symbol_name: String,
    pub symbol_kind: SymbolKind,
    pub language: Language,
    pub score: f32,
    pub snippet: String,
}

/// In-memory inverted index and BM25 ranker for AST symbols.
#[derive(Debug, Clone, Default)]
pub struct LexicalKeywordMatcher {
    documents: HashMap<u64, SymbolDocument>,
    /// Inverted index: term -> Map<symbol_id, term_frequency>
    inverted_index: HashMap<String, HashMap<u64, u32>>,
    /// Document lengths (total token count per symbol_id)
    doc_lengths: HashMap<u64, usize>,
    total_token_count: usize,
}

impl LexicalKeywordMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tokenizes input text into lowercase alphanumeric identifiers, splitting camelCase and snake_case.
    pub fn tokenize(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in text.chars() {
            if ch.is_alphanumeric() {
                if ch.is_uppercase() && !current_token.is_empty() {
                    tokens.push(current_token.to_lowercase());
                    current_token = String::new();
                }
                current_token.push(ch);
            } else {
                if !current_token.is_empty() {
                    tokens.push(current_token.to_lowercase());
                    current_token = String::new();
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token.to_lowercase());
        }

        tokens
    }

    /// Indexes a symbol document into the BM25 inverted index.
    pub fn index_document(&mut self, doc: SymbolDocument) {
        let mut combined_text = String::with_capacity(
            doc.symbol_name.len() * 3 + doc.signature.len() + doc.doc_comment.len() + doc.body_snippet.len()
        );

        // Weight symbol name heavily by repeating it
        combined_text.push_str(&doc.symbol_name);
        combined_text.push(' ');
        combined_text.push_str(&doc.symbol_name);
        combined_text.push(' ');
        combined_text.push_str(&doc.signature);
        combined_text.push(' ');
        combined_text.push_str(&doc.doc_comment);
        combined_text.push(' ');
        combined_text.push_str(&doc.body_snippet);

        let tokens = Self::tokenize(&combined_text);
        let doc_len = tokens.len().max(1);

        let symbol_id = doc.symbol_id;

        // If replacing an existing document, subtract old token count
        if let Some(old_len) = self.doc_lengths.insert(symbol_id, doc_len) {
            self.total_token_count = self.total_token_count.saturating_sub(old_len);
        }
        self.total_token_count += doc_len;

        // Populate inverted index
        let mut tf_map: HashMap<String, u32> = HashMap::new();
        for token in tokens {
            *tf_map.entry(token).or_insert(0) += 1;
        }

        for (token, count) in tf_map {
            self.inverted_index
                .entry(token)
                .or_default()
                .insert(symbol_id, count);
        }

        self.documents.insert(symbol_id, doc);
    }

    /// Performs BM25 scoring across matching candidate documents.
    pub fn search(&self, query_text: &str, filter: &SearchFilter, limit: usize) -> Vec<LexicalCandidate> {
        if self.documents.is_empty() {
            return Vec::new();
        }

        let query_tokens = Self::tokenize(query_text);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        let n = self.documents.len() as f32;
        let avg_dl = (self.total_token_count as f32 / n).max(1.0);
        let k1: f32 = 1.2;
        let b: f32 = 0.75;

        let query_term_set: HashSet<&String> = query_tokens.iter().collect();
        let mut candidate_scores: HashMap<u64, f32> = HashMap::new();

        for &token in &query_term_set {
            if let Some(postings) = self.inverted_index.get(token) {
                let df = postings.len() as f32;
                // Standard BM25 Robertson-Spärck Jones IDF
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.01);

                for (&symbol_id, &tf) in postings {
                    let doc = match self.documents.get(&symbol_id) {
                        Some(d) => d,
                        None => continue,
                    };

                    if !filter.matches(doc.language, &doc.symbol_kind, &doc.file_path) {
                        continue;
                    }

                    let doc_len = *self.doc_lengths.get(&symbol_id).unwrap_or(&1) as f32;
                    let tf_f32 = tf as f32;
                    let num = tf_f32 * (k1 + 1.0);
                    let den = tf_f32 + k1 * (1.0 - b + b * (doc_len / avg_dl));
                    let term_score = idf * (num / den);

                    *candidate_scores.entry(symbol_id).or_insert(0.0) += term_score;
                }
            }
        }

        // Exact name match bonus
        let query_lower = query_text.to_lowercase();
        for (&symbol_id, score) in candidate_scores.iter_mut() {
            if let Some(doc) = self.documents.get(&symbol_id) {
                let sym_lower = doc.symbol_name.to_lowercase();
                if sym_lower == query_lower {
                    *score += 10.0;
                } else if sym_lower.starts_with(&query_lower) || sym_lower.contains(&query_lower) {
                    *score += 5.0;
                }
            }
        }

        // Convert to sorted candidate list
        let mut candidates: Vec<LexicalCandidate> = candidate_scores
            .into_iter()
            .filter_map(|(id, score)| {
                let doc = self.documents.get(&id)?;
                Some(LexicalCandidate {
                    symbol_id: id,
                    file_path: doc.file_path.clone(),
                    symbol_name: doc.symbol_name.clone(),
                    symbol_kind: doc.symbol_kind.clone(),
                    language: doc.language,
                    score,
                    snippet: doc.signature.clone(),
                })
            })
            .collect();

        // Sort descending by score; break ties by symbol_id ascending
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.symbol_id.cmp(&b.symbol_id))
        });

        if limit > 0 && candidates.len() > limit {
            candidates.truncate(limit);
        }

        candidates
    }

    /// Fetches a document by its symbol ID.
    pub fn get_document(&self, symbol_id: u64) -> Option<&SymbolDocument> {
        self.documents.get(&symbol_id)
    }
}
