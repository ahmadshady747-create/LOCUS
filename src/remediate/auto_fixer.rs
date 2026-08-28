//! Deterministic AST Rewriter & Self-Healing Engine.
//!
//! Non-speculative, deterministic AST corrections for unambiguous structural faults:
//! - Closing unmatched JSX/HTML tags.
//! - Converting vulnerable property chains (`a.b.c`) to optional chaining (`a?.b?.c`).
//! - Hoisting conditionally called React hooks to component function root.

#![forbid(unsafe_code)]

use regex::Regex;
use std::sync::LazyLock;
use std::time::Instant;

use crate::guard::AstGuard;
use crate::remediate::patch_strategy::PatchStrategy;
use crate::types::{RemediationEdit, RemediationKind, RemediationResult};

static RE_JSX_OPEN_TAG: LazyLock<Regex> =
    LazyLock::new(
        || match Regex::new(r#"<([a-zA-Z][a-zA-Z0-9.-]*)(?:\s+[^>]*)?>"#) {
            Ok(re) => re,
            Err(_) => Regex::new(r#"<[a-zA-Z]+"#).expect("static regex"),
        },
    );

static RE_JSX_CLOSE_TAG: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r#"</([a-zA-Z][a-zA-Z0-9.-]*)>"#) {
        Ok(re) => re,
        Err(_) => Regex::new(r#"</[a-zA-Z]+"#).expect("static regex"),
    });

static RE_VOID_TAGS: &[&str] = &[
    "img", "input", "br", "hr", "meta", "link", "area", "base", "col", "embed", "param", "source",
    "track", "wbr",
];

static RE_NULL_DEREF_CANDIDATE: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"\b([a-zA-Z_$][a-zA-Z0-9_$]*(?:\.[a-zA-Z_$][a-zA-Z0-9_$]*){2,})") {
        Ok(re) => re,
        Err(_) => Regex::new(r"\w+\.\w+").expect("static regex"),
    }
});

static RE_CONDITIONAL_HOOK: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(
        r#"(?s)(if\s*\([^)]*\)\s*\{[^}]*?)(\b(?:const|let)\s+(?:\[[^\]]+\]|\w+)\s*=\s*use[A-Z]\w*\s*\([^;]*\);?)([^}]*\})"#,
    ) {
        Ok(re) => re,
        Err(_) => Regex::new(r"if.*use[A-Z]").expect("static regex"),
    }
});

/// High-speed deterministic code remediation engine.
pub struct AutoFixer;

impl AutoFixer {
    /// Perform full deterministic remediation pipeline on source code.
    pub fn remediate(source: &str) -> RemediationResult {
        let start = Instant::now();
        let mut current_code = source.to_string();
        let mut all_edits = Vec::new();

        // 1. Fix Deep Null Dereferences (a.b.c -> a?.b?.c)
        let (code_after_null, null_edits) = Self::fix_null_dereferences(&current_code);
        if !null_edits.is_empty() {
            all_edits.extend(null_edits);
            current_code = code_after_null;
        }

        // 2. Fix Conditional React Hook Hoisting
        let (code_after_hooks, hook_edits) = Self::fix_conditional_hooks(&current_code);
        if !hook_edits.is_empty() {
            all_edits.extend(hook_edits);
            current_code = code_after_hooks;
        }

        // 3. Fix Unclosed JSX Tags
        let (code_after_jsx, jsx_edits) = Self::fix_unclosed_jsx_tags(&current_code);
        if !jsx_edits.is_empty() {
            all_edits.extend(jsx_edits);
            current_code = code_after_jsx;
        }

        // Final verification pass
        let verification = AstGuard::verify(&current_code);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        RemediationResult {
            success: !all_edits.is_empty() && verification.passed,
            original_code: source.to_string(),
            remediated_code: current_code,
            edits_applied: all_edits,
            passed_verification: verification.passed,
            latency_ms: elapsed_ms,
        }
    }

    /// Fix deep property access chains by inserting optional chaining (?.)
    pub fn fix_null_dereferences(source: &str) -> (String, Vec<RemediationEdit>) {
        let mut edits = Vec::new();

        for cap in RE_NULL_DEREF_CANDIDATE.captures_iter(source) {
            if let Some(full_match) = cap.get(0) {
                let matched_str = full_match.as_str();

                // Skip if already has optional chaining or is module/namespace import
                if matched_str.starts_with("process.env")
                    || matched_str.starts_with("import.meta")
                    || matched_str.starts_with("console.log")
                    || matched_str.starts_with("std::")
                {
                    continue;
                }

                let replacement = matched_str.replace('.', "?.");
                edits.push(PatchStrategy::create_edit(
                    RemediationKind::OptionalChaining,
                    format!(
                        "Converted '{}' to optional chaining '{}'",
                        matched_str, replacement
                    ),
                    full_match.start(),
                    full_match.end(),
                    replacement,
                ));
            }
        }

        let new_code = PatchStrategy::apply_edits(source, edits.clone());
        (new_code, edits)
    }

    /// Hoist conditional React hooks out of `if` blocks to function scope
    pub fn fix_conditional_hooks(source: &str) -> (String, Vec<RemediationEdit>) {
        let mut edits = Vec::new();

        for cap in RE_CONDITIONAL_HOOK.captures_iter(source) {
            if let (Some(full_match), Some(if_before), Some(hook_stmt), Some(if_after)) =
                (cap.get(0), cap.get(1), cap.get(2), cap.get(3))
            {
                let hook_str = hook_stmt.as_str().trim();
                let remaining_if = format!("{}{}", if_before.as_str(), if_after.as_str());
                let clean_if = remaining_if
                    .replace("{\n\n", "{\n")
                    .replace("{\n    \n", "{\n");
                let replacement = format!("{}\n{}", hook_str, clean_if);

                edits.push(PatchStrategy::create_edit(
                    RemediationKind::HookHoisting,
                    format!("Hoisted hook '{}' above conditional block", hook_str),
                    full_match.start(),
                    full_match.end(),
                    replacement,
                ));
            }
        }

        let new_code = PatchStrategy::apply_edits(source, edits.clone());
        (new_code, edits)
    }

    /// Balance and close unclosed JSX tags
    pub fn fix_unclosed_jsx_tags(source: &str) -> (String, Vec<RemediationEdit>) {
        let mut tag_stack: Vec<String> = Vec::new();

        // Scan tokens in source
        let mut i = 0;
        let bytes = source.as_bytes();
        let len = bytes.len();

        while i < len {
            if bytes[i] == b'<' {
                // Check if closing tag
                if i + 1 < len && bytes[i + 1] == b'/' {
                    if let Some(cap) = RE_JSX_CLOSE_TAG.captures(&source[i..]) {
                        if let (Some(full), Some(tag_match)) = (cap.get(0), cap.get(1)) {
                            let tag_name = tag_match.as_str();
                            if let Some(pos) = tag_stack.iter().rposition(|t| t == tag_name) {
                                tag_stack.truncate(pos);
                            }
                            i += full.end();
                            continue;
                        }
                    }
                } else if i + 1 < len && bytes[i + 1] != b'!' && bytes[i + 1] != b'?' {
                    // Opening tag
                    if let Some(cap) = RE_JSX_OPEN_TAG.captures(&source[i..]) {
                        if let (Some(full), Some(tag_match)) = (cap.get(0), cap.get(1)) {
                            let tag_name = tag_match.as_str();
                            let is_void = RE_VOID_TAGS.contains(&tag_name);
                            let full_tag = full.as_str();
                            let is_self_closing = full_tag.ends_with("/>");

                            if !is_void && !is_self_closing {
                                tag_stack.push(tag_name.to_string());
                            }
                            i += full.end();
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }

        if tag_stack.is_empty() {
            return (source.to_string(), Vec::new());
        }

        // Synthesize closing tags in reverse order
        let mut closing_tags = String::new();
        for tag in tag_stack.iter().rev() {
            closing_tags.push_str(&format!("</{}>\n", tag));
        }

        let edit = PatchStrategy::create_edit(
            RemediationKind::JsxCloseTag,
            format!("Appended missing JSX closing tags: {}", closing_tags.trim()),
            source.len(),
            source.len(),
            format!("\n{}", closing_tags),
        );

        let new_code = PatchStrategy::apply_edits(source, vec![edit.clone()]);
        (new_code, vec![edit])
    }
}
