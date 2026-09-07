//! Ambiguity Entropy Calculator.
//!
//! Evaluates developer intent into a normalized entropy score H(intent) in [0.0, 1.0]
//! using 4 deterministic dimensions: Symbols, Types, Scope, and Specifications.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use std::time::Instant;
use regex::Regex;

use crate::graph::SymbolGraph;
use crate::intent::socratic::edge_case_detector::EdgeCaseDetector;
use crate::intent::socratic::types::{
    AmbiguityScore, AmbiguousTarget, EntropyBreakdown, EntropyThresholdConfig,
    IntentEntropyReport,
};

// ---------------------------------------------------------------------------
// Lexical Patterns for Intent Component Extraction (LazyLock)
// ---------------------------------------------------------------------------

// Detects file paths (e.g. `src/auth.rs`, `lib/index.ts`, `core/engine.py`)
static RE_FILE_PATHS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:[a-zA-Z0-9_.-]+[/\\])+[a-zA-Z0-9_.-]+\.[a-zA-Z0-9]+"#).unwrap()
});

// Detects standard file extensions (e.g. `.rs`, `.tsx`, `.ts`, `.py`, `.go`, `.cpp`, `.java`)
static RE_FILE_EXTENSIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\.(rs|tsx|jsx|ts|js|py|go|cpp|c|dart|java|php|svelte|astro|vue)\b"#).unwrap()
});

// Detects explicit code identifiers (snake_case, PascalCase, camelCase, double colon paths)
static RE_CODE_IDENTIFIERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)+|[A-Z][a-zA-Z0-9]+|[a-z]+(?:_[a-z0-9]+)+)\b"#).unwrap()
});

// Detects explicit type annotations (e.g. `i32`, `String`, `Result<...>`, `Option<...>`, `Promise<...>`, `bool`, `Vec<...>`)
static RE_TYPE_ANNOTATIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|f32|f64|bool|str|String|Result|Option|Vec|HashMap|Arc|Rc|Box|Promise|void|int|float|double|boolean|list|dict|Record)\b"#).unwrap()
});

// Detects vague / abstract intent verbs and adjectives lacking technical grounding
static RE_VAGUE_TERMS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(fix it|make it work|do something|clean up|improve|better|refactor everything|help me|bug|broken|optimize it|fast|nice|good)\b").unwrap()
});

// ---------------------------------------------------------------------------
// AmbiguityEntropyCalculator Implementation
// ---------------------------------------------------------------------------

pub struct AmbiguityEntropyCalculator;

impl AmbiguityEntropyCalculator {
    /// Mathematical weight constants: w1 + w2 + w3 + w4 = 1.00
    pub const WEIGHT_SYMBOLS: f32 = 0.35;
    pub const WEIGHT_TYPES: f32 = 0.25;
    pub const WEIGHT_SCOPE: f32 = 0.20;
    pub const WEIGHT_SPEC: f32 = 0.20;

    /// Evaluates the developer intent string against the workspace SymbolGraph.
    /// Computes deterministic entropy in sub-0.05ms.
    pub fn evaluate(
        intent: &str,
        target_path: Option<&str>,
        symbol_graph: Option<&SymbolGraph>,
        config: Option<&EntropyThresholdConfig>,
    ) -> IntentEntropyReport {
        let start = Instant::now();
        let active_config = config.copied().unwrap_or_default();

        let trimmed = intent.trim();
        let word_count = trimmed.split_whitespace().count();

        // -------------------------------------------------------------------
        // 1. Calculate Scope Entropy (E_scope) - Weight 0.20
        // -------------------------------------------------------------------
        let scope_entropy: f32;
        let mut has_explicit_path = false;

        if target_path.is_some() {
            scope_entropy = 0.0;
            has_explicit_path = true;
        } else if let Some(m) = RE_FILE_PATHS.find(trimmed) {
            let _ = m.as_str();
            scope_entropy = 0.05;
            has_explicit_path = true;
        } else if RE_FILE_EXTENSIONS.is_match(trimmed) {
            scope_entropy = 0.30;
        } else if trimmed.contains("src/") || trimmed.contains("src\\") || trimmed.contains("module") {
            scope_entropy = 0.40;
        } else {
            scope_entropy = 0.90;
        }

        // -------------------------------------------------------------------
        // 2. Calculate Symbols Entropy (E_symbols) - Weight 0.35
        // -------------------------------------------------------------------
        let mut extracted_identifiers: Vec<String> = Vec::new();
        for cap in RE_CODE_IDENTIFIERS.captures_iter(trimmed) {
            if let Some(ident) = cap.get(1) {
                let s = ident.as_str();
                // Filter out common english words matching PascalCase
                if !matches!(s, "I" | "The" | "A" | "An" | "In" | "On" | "For" | "To" | "Of" | "And" | "Or" | "Is") {
                    extracted_identifiers.push(s.to_string());
                }
            }
        }

        let mut missing_symbols = Vec::new();
        let mut ambiguous_targets = Vec::new();
        let symbol_entropy: f32;

        if extracted_identifiers.is_empty() {
            symbol_entropy = 0.95;
        } else if let Some(graph) = symbol_graph {
            let mut resolved_count = 0;
            let mut ambiguous_count = 0;

            for ident in &extracted_identifiers {
                let matches: Vec<&crate::types::SymbolNode> = graph
                    .nodes
                    .values()
                    .filter(|n| n.name == *ident)
                    .collect();

                if matches.is_empty() {
                    missing_symbols.push(ident.clone());
                } else if matches.len() == 1 {
                    resolved_count += 1;
                } else {
                    ambiguous_count += 1;
                    let candidate_paths: Vec<String> = matches.iter().map(|s| s.file.clone()).collect();
                    let candidate_kinds: Vec<String> = matches.iter().map(|s| s.kind.to_string()).collect();
                    ambiguous_targets.push(AmbiguousTarget {
                        identifier: ident.clone(),
                        candidate_paths,
                        candidate_kinds,
                        confidence: 1.0 / (matches.len() as f32),
                        disambiguation_hint: format!(
                            "Identifier `{}` matches {} symbols across different files. Please specify the target module.",
                            ident,
                            matches.len()
                        ),
                    });
                }
            }

            let total_idents = extracted_identifiers.len() as f32;
            let missing_ratio = (missing_symbols.len() as f32) / total_idents;
            let ambig_ratio = (ambiguous_count as f32) / total_idents;
            symbol_entropy = (0.20 * (1.0 - (resolved_count as f32 / total_idents)) + 0.40 * missing_ratio + 0.40 * ambig_ratio).clamp(0.0, 1.0);
        } else {
            // No graph provided: evaluate syntactic density of identifiers
            let id_count = extracted_identifiers.len();
            if id_count >= 3 {
                symbol_entropy = 0.05;
            } else if id_count >= 1 {
                symbol_entropy = 0.20;
            } else {
                symbol_entropy = 0.80;
            }
        }

        // -------------------------------------------------------------------
        // 3. Calculate Types Entropy (E_types) - Weight 0.25
        // -------------------------------------------------------------------
        let type_match_count = RE_TYPE_ANNOTATIONS.find_iter(trimmed).count();
        let type_entropy = if type_match_count >= 3 {
            0.0
        } else if type_match_count >= 1 {
            0.20
        } else if trimmed.contains("->") || trimmed.contains(':') || trimmed.contains("returns") {
            0.40
        } else {
            0.85
        };

        // -------------------------------------------------------------------
        // 4. Calculate Specification & Edge-Case Entropy (E_spec) - Weight 0.20
        // -------------------------------------------------------------------
        let underspecified_edge_cases = EdgeCaseDetector::detect(trimmed);
        let spec_violation_count = underspecified_edge_cases.len();

        let base_spec_entropy: f32 = match spec_violation_count {
            0 => 0.05,
            1 => 0.35,
            2 => 0.65,
            _ => 0.95,
        };

        // Extra penalty for extremely short or abstract vague prompts
        let vague_penalty: f32 = if RE_VAGUE_TERMS.is_match(trimmed) || word_count < 4 {
            0.40
        } else {
            0.0
        };

        let spec_entropy: f32 = (base_spec_entropy + vague_penalty).clamp(0.0f32, 1.0f32);

        // -------------------------------------------------------------------
        // 5. Compound Entropy Calculation
        // -------------------------------------------------------------------
        let mut total_raw = Self::WEIGHT_SYMBOLS * symbol_entropy
            + Self::WEIGHT_TYPES * type_entropy
            + Self::WEIGHT_SCOPE * scope_entropy
            + Self::WEIGHT_SPEC * spec_entropy;

        if word_count < 3 {
            total_raw = (total_raw + 0.30).min(1.0);
        }

        let total_entropy = AmbiguityScore::new(total_raw);
        let confidence_level = (1.0 - total_entropy.value()).clamp(0.0, 1.0);
        let requires_socratic_dialogue = total_entropy.requires_dialogue(&active_config);

        // -------------------------------------------------------------------
        // 6. Generate Socratic Clarification Questions
        // -------------------------------------------------------------------
        let mut socratic_clarification_questions = Vec::new();

        if !has_explicit_path && scope_entropy > 0.35 {
            socratic_clarification_questions.push(
                "Which specific source file path or module should be created or modified?".to_string(),
            );
        }

        if symbol_entropy > 0.40 && extracted_identifiers.is_empty() {
            socratic_clarification_questions.push(
                "What are the exact names of the functions, structs, or components to implement?".to_string(),
            );
        }

        for ambig in &ambiguous_targets {
            socratic_clarification_questions.push(ambig.disambiguation_hint.clone());
        }

        for edge_case in &underspecified_edge_cases {
            socratic_clarification_questions.push(format!(
                "Edge Case Notice: {}. How should this be handled?",
                edge_case.description
            ));
        }

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        IntentEntropyReport {
            intent: trimmed.to_string(),
            total_entropy,
            breakdown: EntropyBreakdown {
                symbol_entropy,
                type_entropy,
                scope_entropy,
                spec_entropy,
            },
            missing_symbols,
            ambiguous_targets,
            underspecified_edge_cases,
            confidence_level,
            requires_socratic_dialogue,
            socratic_clarification_questions,
            latency_ms,
        }
    }
}
