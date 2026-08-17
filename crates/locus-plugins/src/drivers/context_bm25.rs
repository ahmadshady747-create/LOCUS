//! In-Memory BM25 Context Retrieval Driver.

use crate::traits::ContextSlot;
use crate::types::{ContextSearchResult, SlotError};
use async_trait::async_trait;
use tracing::debug;

pub struct InMemoryBM25Driver;

impl InMemoryBM25Driver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InMemoryBM25Driver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextSlot for InMemoryBM25Driver {
    fn driver_id(&self) -> &'static str {
        "bm25"
    }

    fn driver_name(&self) -> &'static str {
        "In-Memory BM25 (Default)"
    }

    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<ContextSearchResult>, SlotError> {
        debug!("Executing InMemoryBM25Driver search for: '{}'", query);

        // Fast in-memory mock result demonstration / hook
        let results = vec![
            ContextSearchResult {
                file_path: "src/main.rs".to_string(),
                snippet: format!("// Matched in-memory symbols for '{}'", query),
                score: 0.95,
            },
        ];

        Ok(results.into_iter().take(top_k).collect())
    }
}
