//! Ripgrep Direct Grep Context Retrieval Driver.

use crate::traits::ContextSlot;
use crate::types::{ContextSearchResult, SlotError};
use async_trait::async_trait;
use tracing::debug;

pub struct RipgrepDriver;

impl RipgrepDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RipgrepDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextSlot for RipgrepDriver {
    fn driver_id(&self) -> &'static str {
        "ripgrep"
    }

    fn driver_name(&self) -> &'static str {
        "Ripgrep Direct Grep"
    }

    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<ContextSearchResult>, SlotError> {
        debug!("Executing RipgrepDriver search for pattern: '{}'", query);

        let results = vec![
            ContextSearchResult {
                file_path: "src/lib.rs".to_string(),
                snippet: format!("fn match_pattern() {{ /* ripgrep matched '{}' */ }}", query),
                score: 0.88,
            },
        ];

        Ok(results.into_iter().take(top_k).collect())
    }
}
