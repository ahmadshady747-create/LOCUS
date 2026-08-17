use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    /// Zero-overhead, embedded deterministic subword & n-gram vectorizer (no memory footprint or model download)
    LocalFast,
    /// Local Ollama embedding models (e.g., nomic-embed-text, all-minilm:l6-v2, bge-small)
    Ollama,
    /// Custom HTTP OpenAI-compatible embeddings endpoint
    CustomEndpoint,
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        Self::LocalFast
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub dimensions: usize,
    pub model_name: String,
    pub endpoint_url: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::LocalFast,
            dimensions: 384,
            model_name: "nomic-embed-text".to_string(),
            endpoint_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: String, // "Function" | "Struct" | "Class" | "Interface" | "FileSummary" | "Template"
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub language: Option<String>,
    pub vector: Vec<f32>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl VectorDocument {
    pub fn new(
        id: impl Into<String>,
        file_path: impl Into<String>,
        symbol_kind: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            file_path: file_path.into(),
            symbol_name: None,
            symbol_kind: symbol_kind.into(),
            content: content.into(),
            line_start: 1,
            line_end: 1,
            language: None,
            vector: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_symbol(mut self, name: impl Into<String>, start: usize, end: usize) -> Self {
        self.symbol_name = Some(name.into());
        self.line_start = start;
        self.line_end = end;
        self
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn with_vector(mut self, vector: Vec<f32>) -> Self {
        self.vector = vector;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub document_id: String,
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub symbol_kind: String,
    pub snippet: String,
    pub line_start: usize,
    pub line_end: usize,
    pub similarity: f32, // Cosine similarity in range [0.0, 1.0]
    pub language: Option<String>,
    pub tags: Vec<String>,
}
