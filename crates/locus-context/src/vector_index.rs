use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::RwLock;

use crate::vector_types::{SemanticSearchResult, VectorDocument};

/// Compute cosine similarity between two normalized or unnormalized float slices
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a <= 0.0 || norm_b <= 0.0 {
        return 0.0;
    }

    let denom = (norm_a * norm_b).sqrt();
    let score = dot / denom;
    // Clamp to [-1.0, 1.0] to guard against IEEE float rounding
    score.clamp(-1.0, 1.0)
}

/// Thread-safe in-memory vector index for code symbols and documentation
pub struct VectorIndex {
    documents: RwLock<Vec<VectorDocument>>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            documents: RwLock::new(Vec::new()),
        }
    }

    /// Insert or replace a document in the index
    pub fn upsert(&self, doc: VectorDocument) {
        let mut docs = self.documents.write().unwrap();
        if let Some(pos) = docs.iter().position(|d| d.id == doc.id) {
            docs[pos] = doc;
        } else {
            docs.push(doc);
        }
    }

    /// Insert multiple documents in batch
    pub fn upsert_batch(&self, new_docs: Vec<VectorDocument>) {
        let mut docs = self.documents.write().unwrap();
        for doc in new_docs {
            if let Some(pos) = docs.iter().position(|d| d.id == doc.id) {
                docs[pos] = doc;
            } else {
                docs.push(doc);
            }
        }
    }

    /// Remove all documents associated with a file path (e.g. before re-indexing)
    pub fn remove_file(&self, file_path: &str) -> usize {
        let mut docs = self.documents.write().unwrap();
        let initial_len = docs.len();
        docs.retain(|d| d.file_path != file_path);
        initial_len - docs.len()
    }

    /// Total number of indexed vector documents
    pub fn len(&self) -> usize {
        self.documents.read().unwrap().len()
    }

    /// Total number of indexed vector documents (alias)
    pub fn document_count(&self) -> usize {
        self.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.documents.read().unwrap().is_empty()
    }

    /// Checks if a file path is present in the index
    pub fn contains_file(&self, file_path: &str) -> bool {
        self.documents.read().unwrap().iter().any(|d| d.file_path == file_path)
    }

    /// Returns a clone of all indexed documents
    pub fn get_all_documents(&self) -> Vec<VectorDocument> {
        self.documents.read().unwrap().clone()
    }

    /// Clear all documents from index
    pub fn clear(&self) {
        self.documents.write().unwrap().clear();
    }

    /// Search for most semantically similar documents
    pub fn search(
        &self,
        query_vector: &[f32],
        top_k: usize,
        min_similarity: f32,
    ) -> Vec<SemanticSearchResult> {
        let docs = self.documents.read().unwrap();
        let mut scored: Vec<(f32, &VectorDocument)> = Vec::with_capacity(docs.len());

        for doc in docs.iter() {
            if doc.vector.is_empty() {
                continue;
            }
            let sim = cosine_similarity(query_vector, &doc.vector);
            if sim >= min_similarity {
                scored.push((sim, doc));
            }
        }

        // Sort descending by similarity score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .map(|(similarity, doc)| {
                // Truncate snippet to 300 chars for preview
                let snippet = if doc.content.len() > 300 {
                    format!("{}...", &doc.content[..300])
                } else {
                    doc.content.clone()
                };

                SemanticSearchResult {
                    document_id: doc.id.clone(),
                    file_path: doc.file_path.clone(),
                    symbol_name: doc.symbol_name.clone(),
                    symbol_kind: doc.symbol_kind.clone(),
                    snippet,
                    line_start: doc.line_start,
                    line_end: doc.line_end,
                    similarity,
                    language: doc.language.clone(),
                    tags: doc.tags.clone(),
                }
            })
            .collect()
    }

    /// Save index to a persistent JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let docs = self.documents.read().unwrap();
        let file = File::create(path.as_ref())
            .with_context(|| format!("Failed to create vector index file at {:?}", path.as_ref()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &*docs)
            .with_context(|| "Failed to serialize vector index")?;
        Ok(())
    }

    /// Load index from a persistent JSON file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<usize> {
        if !path.as_ref().exists() {
            return Ok(0);
        }
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open vector index file at {:?}", path.as_ref()))?;
        let reader = BufReader::new(file);
        let loaded_docs: Vec<VectorDocument> = serde_json::from_reader(reader)
            .with_context(|| "Failed to deserialize vector index")?;

        let count = loaded_docs.len();
        let mut docs = self.documents.write().unwrap();
        *docs = loaded_docs;
        Ok(count)
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        let v3 = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-5);
        assert!((cosine_similarity(&v1, &v3) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_vector_index_search() {
        let index = VectorIndex::new();

        let doc1 = VectorDocument::new("doc1", "src/auth.rs", "Function", "fn verify_jwt_token()")
            .with_vector(vec![0.9, 0.1, 0.0]);
        let doc2 = VectorDocument::new("doc2", "src/db.rs", "Function", "fn query_database_pool()")
            .with_vector(vec![0.0, 0.9, 0.1]);

        index.upsert_batch(vec![doc1, doc2]);
        assert_eq!(index.len(), 2);

        let query = vec![0.85, 0.15, 0.0];
        let results = index.search(&query, 2, 0.1);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document_id, "doc1");
        assert!(results[0].similarity > 0.9);
    }
}
