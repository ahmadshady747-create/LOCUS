//! Core Types for the Swappable Slots & Plugin Architecture.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    Context,
    Sandbox,
}

impl SlotType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Context => "Context Retrieval Slot",
            Self::Sandbox => "Sandbox Execution Slot",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlotDescriptor {
    pub id: String,
    pub name: String,
    pub slot_type: SlotType,
    pub description: String,
    pub is_active: bool,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotsConfig {
    pub active_context_driver: String,
    pub active_sandbox_driver: String,
    pub descriptors: Vec<SlotDescriptor>,
}

impl Default for SlotsConfig {
    fn default() -> Self {
        Self {
            active_context_driver: "bm25".to_string(),
            active_sandbox_driver: "native".to_string(),
            descriptors: vec![
                SlotDescriptor {
                    id: "bm25".to_string(),
                    name: "In-Memory BM25 (Default)".to_string(),
                    slot_type: SlotType::Context,
                    description: "Zero-latency pure in-memory lexical retrieval engine (<5ms)".to_string(),
                    is_active: true,
                    is_builtin: true,
                },
                SlotDescriptor {
                    id: "ripgrep".to_string(),
                    name: "Ripgrep Direct Grep".to_string(),
                    slot_type: SlotType::Context,
                    description: "High-performance disk-level direct regex search driver".to_string(),
                    is_active: false,
                    is_builtin: true,
                },
                SlotDescriptor {
                    id: "native".to_string(),
                    name: "Native Process Execution".to_string(),
                    slot_type: SlotType::Sandbox,
                    description: "Direct cross-platform async process execution driver".to_string(),
                    is_active: true,
                    is_builtin: true,
                },
                SlotDescriptor {
                    id: "mock".to_string(),
                    name: "Mock Sandbox Isolation".to_string(),
                    slot_type: SlotType::Sandbox,
                    description: "Safe isolated dry-run driver for sandbox testing and diagnostics".to_string(),
                    is_active: false,
                    is_builtin: true,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSearchResult {
    pub file_path: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[derive(Debug, Error)]
pub enum SlotError {
    #[error("Slot driver '{0}' not found for type '{1:?}'")]
    DriverNotFound(String, SlotType),

    #[error("Slot execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Slot configuration error: {0}")]
    ConfigError(String),

    #[error("Internal slot error: {0}")]
    Other(String),
}
