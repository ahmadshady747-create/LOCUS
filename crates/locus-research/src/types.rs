//! Core types for semantic research, registry lookups, and compiler error resolution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Rust,
    TypeScript,
    Python,
    General,
}

impl Default for Ecosystem {
    fn default() -> Self {
        Self::General
    }
}

impl Ecosystem {
    pub fn from_str_lenient(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "rust" | "rs" | "cargo" | "crates.io" | "docs.rs" => Self::Rust,
            "typescript" | "ts" | "javascript" | "js" | "npm" | "node" => Self::TypeScript,
            "python" | "py" | "pypi" | "pip" => Self::Python,
            _ => Self::General,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust (crates.io / docs.rs)",
            Self::TypeScript => "TypeScript / JavaScript (npm)",
            Self::Python => "Python (PyPI)",
            Self::General => "General / Multi-ecosystem",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocQuery {
    pub query: String,
    pub ecosystem: Ecosystem,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub license: Option<String>,
    pub downloads: Option<u64>,
    pub ecosystem: Ecosystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSection {
    pub title: String,
    pub content: String,
    pub code_snippets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSearchResult {
    pub package: PackageMetadata,
    pub summary_markdown: String,
    pub signatures: Vec<String>,
    pub cached: bool,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerErrorDiagnostic {
    pub code: Option<String>,
    pub language: Ecosystem,
    pub raw_message: String,
    pub file_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedErrorSolution {
    pub error_code: String,
    pub error_title: String,
    pub language: String,
    pub explanation: String,
    pub recommended_fix_markdown: String,
    pub negative_memory_pattern: String,
    pub references: Vec<String>,
}
