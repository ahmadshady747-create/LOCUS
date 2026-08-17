//! Abstract Trait Interfaces for Core Slots.

use crate::types::{ContextSearchResult, ExecutionResult, SlotError};
use async_trait::async_trait;

#[async_trait]
pub trait ContextSlot: Send + Sync {
    /// Unique identifier for this driver (e.g. "bm25", "ripgrep").
    fn driver_id(&self) -> &'static str;

    /// Human readable name for this driver.
    fn driver_name(&self) -> &'static str;

    /// Executes a contextual lexical or semantic search.
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<ContextSearchResult>, SlotError>;
}

#[async_trait]
pub trait SandboxSlot: Send + Sync {
    /// Unique identifier for this driver (e.g. "native", "mock").
    fn driver_id(&self) -> &'static str;

    /// Human readable name for this driver.
    fn driver_name(&self) -> &'static str;

    /// Executes a command within the sandbox environment.
    async fn execute(&self, command: &str, working_dir: &str) -> Result<ExecutionResult, SlotError>;
}
