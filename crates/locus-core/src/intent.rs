//! Zero-Lag Intent Classifier for OmniBar & Ambient HUD.
//!
//! Classifies raw input into actionable intents in sub-millisecond (<0.5ms) time,
//! supporting prefix-based dispatch, natural language action verbs (Arabic & English),
//! and clipboard integration.

use serde::{Deserialize, Serialize};

/// High-level classified intent from omnibar input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OmniIntent {
    /// Local workspace search in file paths and code contents.
    LocalSearch { query: String },
    /// Web search through private/duckduckgo or configured engine.
    WebSearch { query: String },
    /// Direct terminal / shell command execution.
    TerminalCommand { command: String },
    /// Chat context and past conversation memory retrieval.
    ChatMemory { description: String },
    /// Formal verification and invariant checking.
    FormalVerify { target: String },
    /// Autonomous Agent Action / code transformation.
    AgentAction {
        prompt: String,
        target_code: Option<String>,
    },
}

impl OmniIntent {
    /// Classifies an omnibar input query into an OmniIntent in <0.5ms with zero panics.
    pub fn parse(input: &str, selected_clipboard: Option<String>) -> Self {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Self::LocalSearch {
                query: String::new(),
            };
        }

        // 1. Explicit Prefix Matches
        if let Some(rest) = trimmed.strip_prefix('?') {
            return Self::WebSearch {
                query: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("/w ") {
            return Self::WebSearch {
                query: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix('>') {
            return Self::TerminalCommand {
                command: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("@chat ") {
            return Self::ChatMemory {
                description: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("@memory ") {
            return Self::ChatMemory {
                description: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("#verify") {
            return Self::FormalVerify {
                target: rest.trim().to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix('!') {
            return Self::AgentAction {
                prompt: rest.trim().to_string(),
                target_code: selected_clipboard,
            };
        }

        // 2. Natural Language Action Verb Detection (English & Arabic)
        let lower = trimmed.to_lowercase();
        let first_word = lower
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| !c.is_alphanumeric());

        let is_action_verb = matches!(
            first_word,
            "fix"
                | "refactor"
                | "convert"
                | "generate"
                | "create"
                | "write"
                | "build"
                | "explain"
                | "check"
                | "patch"
                | "أصلح"
                | "اصلح"
                | "حوّل"
                | "حول"
                | "ترقيع"
                | "اكتب"
                | "أنشئ"
                | "انشئ"
                | "ابن"
                | "ابني"
                | "اشرح"
                | "فحص"
                | "افحص"
        );

        if is_action_verb {
            return Self::AgentAction {
                prompt: trimmed.to_string(),
                target_code: selected_clipboard,
            };
        }

        // 3. Default fallback: LocalSearch
        Self::LocalSearch {
            query: trimmed.to_string(),
        }
    }
}
