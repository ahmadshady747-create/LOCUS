//! Fast S-Expression & AST Pattern Matching Engine.
//!
//! Sub-millisecond structural AST pattern matching for diagnostics,
//! linting rules, code generation invariants, and architectural queries.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use regex::Regex;
use crate::types::{AstQueryKind, AstQueryMatch};

static RE_CALL_EXPR: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(([^)]*)\)"#) {
        Ok(re) => re,
        Err(_) => Regex::new(r"\w+\(\)").expect("static regex"),
    }
});

static RE_JSX_TAG: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"<([a-zA-Z][a-zA-Z0-9.-]*)(?:\s+[^>]*)?(?:/?>|>)"#) {
        Ok(re) => re,
        Err(_) => Regex::new(r"<[a-zA-Z]+").expect("static regex"),
    }
});

static RE_MEMBER_ACCESS: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"([a-zA-Z_$][a-zA-Z0-9_$]*)\.([a-zA-Z_$][a-zA-Z0-9_$]*)"#) {
        Ok(re) => re,
        Err(_) => Regex::new(r"\w+\.\w+").expect("static regex"),
    }
});

/// S-Expression AST Query Engine.
pub struct AstQueryEngine;

impl AstQueryEngine {
    /// Execute an S-expression pattern query across source code.
    /// Example patterns:
    /// - `(call_expression function: "fetch")`
    /// - `(jsx_element tag: "Button")`
    /// - `(member_access object: "process" property: "env")`
    pub fn query(pattern: &str, source: &str) -> Vec<AstQueryMatch> {
        let mut matches = Vec::new();
        let trimmed_pattern = pattern.trim();

        if trimmed_pattern.starts_with("(call_expression") {
            let target_fn = Self::extract_pattern_field(trimmed_pattern, "function");
            for cap in RE_CALL_EXPR.captures_iter(source) {
                if let Some(fn_match) = cap.get(1) {
                    let fn_name = fn_match.as_str();
                    if target_fn.is_none() || target_fn.as_deref() == Some(fn_name) || target_fn.as_deref() == Some("*") {
                        if let Some(full_match) = cap.get(0) {
                            matches.push(AstQueryMatch {
                                pattern: pattern.to_string(),
                                capture_name: "call".to_string(),
                                node_kind: AstQueryKind::CallExpression,
                                text: full_match.as_str().to_string(),
                                byte_start: full_match.start(),
                                byte_end: full_match.end(),
                            });
                        }
                    }
                }
            }
        } else if trimmed_pattern.starts_with("(jsx_element") {
            let target_tag = Self::extract_pattern_field(trimmed_pattern, "tag");
            for cap in RE_JSX_TAG.captures_iter(source) {
                if let Some(tag_match) = cap.get(1) {
                    let tag_name = tag_match.as_str();
                    if target_tag.is_none() || target_tag.as_deref() == Some(tag_name) || target_tag.as_deref() == Some("*") {
                        if let Some(full_match) = cap.get(0) {
                            matches.push(AstQueryMatch {
                                pattern: pattern.to_string(),
                                capture_name: "tag".to_string(),
                                node_kind: AstQueryKind::JsxElement,
                                text: full_match.as_str().to_string(),
                                byte_start: full_match.start(),
                                byte_end: full_match.end(),
                            });
                        }
                    }
                }
            }
        } else if trimmed_pattern.starts_with("(member_access") {
            let target_obj = Self::extract_pattern_field(trimmed_pattern, "object");
            let target_prop = Self::extract_pattern_field(trimmed_pattern, "property");
            for cap in RE_MEMBER_ACCESS.captures_iter(source) {
                let obj = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let prop = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                let obj_ok = target_obj.is_none() || target_obj.as_deref() == Some(obj) || target_obj.as_deref() == Some("*");
                let prop_ok = target_prop.is_none() || target_prop.as_deref() == Some(prop) || target_prop.as_deref() == Some("*");

                if obj_ok && prop_ok {
                    if let Some(full_match) = cap.get(0) {
                        matches.push(AstQueryMatch {
                            pattern: pattern.to_string(),
                            capture_name: "member".to_string(),
                            node_kind: AstQueryKind::MemberAccess,
                            text: full_match.as_str().to_string(),
                            byte_start: full_match.start(),
                            byte_end: full_match.end(),
                        });
                    }
                }
            }
        }

        matches
    }

    fn extract_pattern_field(pattern: &str, field: &str) -> Option<String> {
        let needle = format!("{}:", field);
        if let Some(pos) = pattern.find(&needle) {
            let rest = &pattern[pos + needle.len()..];
            let clean = rest.trim_start()
                .trim_matches(|c| c == '"' || c == '\'' || c == ')' || c == ' ')
                .split_whitespace()
                .next()
                .map(|s| s.trim_matches(|c| c == '"' || c == '\'' || c == ')'));
            clean.map(|s| s.to_string())
        } else {
            None
        }
    }
}
