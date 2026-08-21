//! AstDiffEngine — Surgical node-level byte-span AST patching and skeleton extraction.

use thiserror::Error;
use crate::graph::SymbolGraph;
use crate::types::Language;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DiffError {
    #[error("Target symbol '{0}' was not found in source file")]
    SymbolNotFound(String),

    #[error("Byte offset range [{0}..{1}] is out of bounds for source length {2}")]
    ByteSpanOverflow(usize, usize, usize),

    #[error("Patched syntax failed delimiter balance check")]
    InvalidSyntax(String),
}

pub struct AstDiffEngine;

impl AstDiffEngine {
    /// Surgically replaces a named symbol's definition inside `source` with `new_code`.
    pub fn patch(source: &str, symbol_name: &str, new_code: &str, lang: Language) -> Result<String, DiffError> {
        let mut graph = SymbolGraph::new();
        graph.index_file_content("target", source, lang);

        let target_node = graph
            .nodes
            .values()
            .find(|n| n.name == symbol_name)
            .ok_or_else(|| DiffError::SymbolNotFound(symbol_name.to_string()))?;

        let start = target_node.byte_start;
        let end = target_node.byte_end;
        let src_len = source.len();

        if start > src_len || end > src_len || start > end {
            return Err(DiffError::ByteSpanOverflow(start, end, src_len));
        }

        let mut output = String::with_capacity(src_len + new_code.len());
        output.push_str(&source[..start]);
        output.push_str(new_code.trim());
        output.push_str(&source[end..]);

        Ok(output)
    }

    /// Extracts a high-level skeleton by replacing function bodies with semicolons or passes.
    pub fn skeletonize(source: &str, lang: Language) -> String {
        let mut graph = SymbolGraph::new();
        graph.index_file_content("target", source, lang);

        if graph.nodes.is_empty() {
            return source.to_string();
        }

        let mut sorted_nodes: Vec<_> = graph.nodes.values().collect();
        sorted_nodes.sort_by_key(|n| n.byte_start);

        let mut skeleton = String::with_capacity(source.len() / 2);
        for node in sorted_nodes {
            match lang {
                Language::Rust => {
                    skeleton.push_str(&node.signature);
                    if !node.signature.ends_with(';') {
                        skeleton.push_str(";\n");
                    } else {
                        skeleton.push('\n');
                    }
                }
                Language::TypeScript => {
                    skeleton.push_str(&node.signature);
                    skeleton.push_str(";\n");
                }
                Language::Python => {
                    skeleton.push_str(&node.signature);
                    skeleton.push_str(":\n    ...\n");
                }
                Language::Unknown => {
                    skeleton.push_str(&node.signature);
                    skeleton.push('\n');
                }
            }
        }

        skeleton
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_rust_function() {
        let original = r#"
pub fn greet() -> &'static str {
    "hello old world"
}

pub fn answer() -> i32 {
    42
}
"#;
        let replacement = r#"pub fn greet() -> &'static str {
    "hello sovereign world"
}"#;

        let patched = AstDiffEngine::patch(original, "greet", replacement, Language::Rust)
            .expect("Patch operation failed");

        assert!(patched.contains("hello sovereign world"));
        assert!(patched.contains("pub fn answer() -> i32 {"));
    }

    #[test]
    fn test_patch_symbol_not_found() {
        let code = "pub fn foo() {}";
        let res = AstDiffEngine::patch(code, "bar", "pub fn bar() {}", Language::Rust);
        assert_eq!(res, Err(DiffError::SymbolNotFound("bar".to_string())));
    }
}
