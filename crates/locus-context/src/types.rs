pub use locus_core::types::SecurityLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub security_level: SecurityLevel,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub version: String,
}

impl Template {
    pub fn new(
        id: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        code: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            category: category.into(),
            name: name.into(),
            description: description.into(),
            code: code.into(),
            language: language.into(),
            security_level: SecurityLevel::Safe,
            tags: vec![],
            dependencies: vec![],
            version: "1.0.0".to_string(),
        }
    }

    pub fn with_security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLog {
    pub agent_id: String,
    pub error_message: String,
    pub timestamp: String,
    pub context: Option<String>,
    pub stack_trace: Option<String>,
}

impl ErrorLog {
    pub fn new(agent_id: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            error_message: error_message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: None,
            stack_trace: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    pub system_prompt: String,
    pub user_prompt: String,
    pub template_context: String,
    pub error_context: String,
    pub full_prompt: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct PromptSection {
    pub name: String,
    pub content: String,
    pub priority: i32,
    pub token_estimate: usize,
}

impl PromptSection {
    pub fn new(name: impl Into<String>, content: impl Into<String>, priority: i32) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            name: name.into(),
            content,
            priority,
            token_estimate,
        }
    }
}

fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 3.5).ceil() as usize
}

// ============================================================================
// Symbol Graph & Hybrid Context Retrieval Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Struct,
    Function,
    Trait,
    TypeAlias,
    Class,
    Interface,
    Enum,
    Constant,
    Variable,
    Module,
}

impl SymbolKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Function => "function",
            Self::Trait => "trait",
            Self::TypeAlias => "type",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Enum => "enum",
            Self::Constant => "const",
            Self::Variable => "var",
            Self::Module => "module",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolNode {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line_number: usize,
    pub signature: String,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolEdge {
    pub source_file: String,
    pub target_symbol: String,
    pub target_module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Document {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub content: String,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25SearchResult {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub score: f32,
    pub matched_terms: Vec<String>,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridContextPayload {
    pub query: String,
    pub symbols: Vec<SymbolNode>,
    pub bm25_results: Vec<Bm25SearchResult>,
    pub dense_context: String,
    pub token_estimate: usize,
    pub latency_ms: u64,
}