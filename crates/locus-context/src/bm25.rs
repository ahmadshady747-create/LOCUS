//! Pure in-memory Okapi BM25 Lexical Retrieval Engine.
//!
//! Sub-5ms keyword and identifier matching across codebases without vector databases or embeddings.

use crate::types::{Bm25Document, Bm25SearchResult};
use regex::Regex;
use std::collections::{HashMap, HashSet};

pub struct Bm25Engine {
    documents: HashMap<String, Bm25Document>,
    /// Maps indexed term -> list of (doc_id, term_frequency)
    inverted_index: HashMap<String, Vec<(String, usize)>>,
    /// Document lengths (total token count per doc)
    doc_lengths: HashMap<String, usize>,
    /// Total number of indexed documents
    total_docs: usize,
    /// Average document length across the index
    avg_doc_length: f32,
    /// BM25 term frequency saturation parameter (default: 1.2)
    k1: f32,
    /// BM25 document length normalization parameter (default: 0.75)
    b: f32,
}

impl Default for Bm25Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Engine {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            total_docs: 0,
            avg_doc_length: 0.0,
            k1: 1.2,
            b: 0.75,
        }
    }

    /// Indexes or updates a document in the lexical inverted index.
    pub fn add_document(&mut self, doc: Bm25Document) {
        let tokens = Self::tokenize_code(&format!("{} {}", doc.title, doc.content));
        let token_count = tokens.len().max(1);

        // Count term frequencies in this doc
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        for t in tokens {
            *term_freqs.entry(t).or_insert(0) += 1;
        }

        let doc_id = doc.id.clone();

        // Update inverted index
        for (term, tf) in term_freqs {
            self.inverted_index
                .entry(term)
                .or_default()
                .push((doc_id.clone(), tf));
        }

        self.doc_lengths.insert(doc_id.clone(), token_count);
        self.documents.insert(doc_id, doc);
        self.total_docs = self.documents.len();

        // Recalculate average document length
        let total_tokens: usize = self.doc_lengths.values().sum();
        self.avg_doc_length = if self.total_docs > 0 {
            total_tokens as f32 / self.total_docs as f32
        } else {
            0.0
        };
    }

    /// Searches the indexed corpus using the Okapi BM25 ranking algorithm.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Bm25SearchResult> {
        if self.total_docs == 0 {
            return vec![];
        }

        let query_tokens = Self::tokenize_query(query);
        if query_tokens.is_empty() {
            return vec![];
        }

        let mut doc_scores: HashMap<String, f32> = HashMap::new();
        let mut doc_matched_terms: HashMap<String, HashSet<String>> = HashMap::new();

        let n = self.total_docs as f32;
        let avgdl = self.avg_doc_length.max(1.0);

        for token in &query_tokens {
            if let Some(postings) = self.inverted_index.get(token) {
                let doc_freq = postings.len() as f32;
                // IDF with smoothing: ln(1 + (N - n + 0.5) / (n + 0.5))
                let idf = ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();

                for (doc_id, tf) in postings {
                    let doc_len = *self.doc_lengths.get(doc_id).unwrap_or(&1) as f32;
                    let tf_val = *tf as f32;

                    // Okapi BM25 formula
                    let numerator = tf_val * (self.k1 + 1.0);
                    let denominator = tf_val + self.k1 * (1.0 - self.b + self.b * (doc_len / avgdl));
                    let term_score = idf * (numerator / denominator);

                    *doc_scores.entry(doc_id.clone()).or_insert(0.0) += term_score;
                    doc_matched_terms
                        .entry(doc_id.clone())
                        .or_default()
                        .insert(token.clone());
                }
            }
        }

        let mut ranked: Vec<(String, f32)> = doc_scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        ranked
            .into_iter()
            .take(limit)
            .filter_map(|(doc_id, score)| {
                let doc = self.documents.get(&doc_id)?;
                let matched = doc_matched_terms.remove(&doc_id).unwrap_or_default();
                let snippet = Self::create_snippet(&doc.content, &matched);

                Some(Bm25SearchResult {
                    id: doc.id.clone(),
                    file_path: doc.file_path.clone(),
                    title: doc.title.clone(),
                    score,
                    matched_terms: matched.into_iter().collect(),
                    snippet,
                })
            })
            .collect()
    }

    /// Tokenizes code content into subwords, identifiers, and terms.
    pub fn tokenize_code(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let re_ident = Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]*").unwrap();

        for m in re_ident.find_iter(text) {
            let word = m.as_str();
            let lower = word.to_lowercase();
            tokens.push(lower.clone());

            // Subword identifier splitting (e.g. CognitiveRouter -> cognitive, router)
            let subwords = Self::split_identifier(word);
            for sw in subwords {
                if sw != lower {
                    tokens.push(sw);
                }
            }
        }

        tokens
    }

    /// Tokenizes query string.
    pub fn tokenize_query(query: &str) -> Vec<String> {
        Self::tokenize_code(query)
    }

    /// Splits camelCase, PascalCase, or snake_case identifiers into individual terms.
    fn split_identifier(ident: &str) -> Vec<String> {
        let mut parts = Vec::new();
        // Snake case split
        for part in ident.split('_') {
            if part.is_empty() {
                continue;
            }
            // Camel/Pascal case split
            let mut current = String::new();
            let chars: Vec<char> = part.chars().collect();
            for i in 0..chars.len() {
                let c = chars[i];
                if c.is_uppercase() && !current.is_empty() {
                    let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
                    if next_is_lower || current.chars().all(|ch| ch.is_lowercase()) {
                        parts.push(current.to_lowercase());
                        current = String::new();
                    }
                }
                current.push(c);
            }
            if !current.is_empty() {
                parts.push(current.to_lowercase());
            }
        }
        parts
    }

    fn create_snippet(content: &str, matched_terms: &HashSet<String>) -> String {
        let lines: Vec<&str> = content.lines().collect();
        for line in &lines {
            let lower = line.to_lowercase();
            if matched_terms.iter().any(|t| lower.contains(t)) {
                return line.trim().to_string();
            }
        }
        lines.first().unwrap_or(&"").trim().to_string()
    }

    pub fn total_documents(&self) -> usize {
        self.total_docs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_subword_tokenization() {
        let ident = "CognitiveRouterEngine_v2";
        let subwords = Bm25Engine::split_identifier(ident);
        assert!(subwords.contains(&"cognitive".to_string()));
        assert!(subwords.contains(&"router".to_string()));
        assert!(subwords.contains(&"engine".to_string()));
    }

    #[test]
    fn test_bm25_indexing_and_ranking() {
        let mut engine = Bm25Engine::new();

        engine.add_document(Bm25Document {
            id: "doc1".to_string(),
            file_path: "src/router.rs".to_string(),
            title: "Cognitive Router Engine".to_string(),
            content: "Classifies micro-tasks and standard architectural tasks using cost matrices.".to_string(),
            token_count: 10,
        });

        engine.add_document(Bm25Document {
            id: "doc2".to_string(),
            file_path: "src/radar.rs".to_string(),
            title: "Free Provider Radar".to_string(),
            content: "Discovers free tier API keys for Gemini Flash and Groq cloud.".to_string(),
            token_count: 10,
        });

        let results = engine.search("cognitive task router", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].score > 0.0);
    }
}
