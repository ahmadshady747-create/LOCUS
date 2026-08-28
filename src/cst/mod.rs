//! Lossless Concrete Syntax Tree (CST) Module.
//!
//! Implements a 100% safe, pure-Rust Green/Red Tree architecture for full trivia preservation,
//! sub-microsecond incremental node navigation, and byte-span accuracy.

#![forbid(unsafe_code)]

pub mod green;
pub mod lexer;
pub mod red;

pub use green::{
    GreenElement, GreenNode, GreenNodeBuilder, GreenToken, SyntaxKind, TextRange, TextSize,
};
pub use lexer::tokenize;
pub use red::{SyntaxElement, SyntaxNode, SyntaxToken};

/// Parses source text into a structured, lossless `GreenNode` preserving all whitespace and comments.
pub fn parse_to_green(source: &str) -> GreenNode {
    let tokens = tokenize(source);
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::Root);

    let mut in_decl = false;
    let mut brace_depth = 0;

    let mut i = 0;
    while i < tokens.len() {
        let (kind, text) = tokens[i];

        if !in_decl {
            let is_fn_start = (kind == SyntaxKind::FnKw)
                || (kind == SyntaxKind::AsyncKw
                    && tokens[i + 1..]
                        .iter()
                        .take(5)
                        .any(|(k, _)| *k == SyntaxKind::FnKw))
                || (kind == SyntaxKind::PubKw
                    && tokens[i + 1..]
                        .iter()
                        .take(5)
                        .any(|(k, _)| *k == SyntaxKind::FnKw || *k == SyntaxKind::AsyncKw));

            let is_struct_start = (kind == SyntaxKind::StructKw
                || kind == SyntaxKind::ClassKw
                || kind == SyntaxKind::InterfaceKw
                || kind == SyntaxKind::EnumKw
                || kind == SyntaxKind::TraitKw)
                || (kind == SyntaxKind::PubKw
                    && tokens[i + 1..].iter().take(5).any(|(k, _)| {
                        *k == SyntaxKind::StructKw
                            || *k == SyntaxKind::ClassKw
                            || *k == SyntaxKind::InterfaceKw
                            || *k == SyntaxKind::EnumKw
                            || *k == SyntaxKind::TraitKw
                    }));

            if is_fn_start {
                builder.start_node(SyntaxKind::FunctionDecl);
                in_decl = true;
            } else if is_struct_start {
                builder.start_node(SyntaxKind::StructDecl);
                in_decl = true;
            }
        }

        builder.token(kind, text);

        if kind == SyntaxKind::OpenBrace {
            brace_depth += 1;
        } else if kind == SyntaxKind::CloseBrace {
            if brace_depth > 0 {
                brace_depth -= 1;
            }
            if brace_depth == 0 && in_decl {
                builder.finish_node();
                in_decl = false;
            }
        } else if kind == SyntaxKind::Semicolon && brace_depth == 0 && in_decl {
            builder.finish_node();
            in_decl = false;
        }

        i += 1;
    }

    if in_decl {
        builder.finish_node();
    }

    builder.finish_node(); // Close Root
    builder.finish()
}

/// Parses source text into a navigable `SyntaxNode` root.
pub fn parse_to_cst(source: &str) -> SyntaxNode {
    let green = parse_to_green(source);
    SyntaxNode::new_root(green)
}

/// Losslessly reconstructs the exact original text from a syntax node.
pub fn to_lossless_text(node: &SyntaxNode) -> String {
    node.text()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossless_trivia_rust_roundtrip() {
        let original = r#"
/// Documentation for greet
// Single line comment
pub async fn greet(name: &str) -> String {
    /* Multi-line
       block comment */
    let formatted = format!("Hello, {}!", name);
    formatted
}
"#;
        let cst = parse_to_cst(original);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            reconstructed, original,
            "Reconstructed Rust CST text must match original 100% byte-for-byte"
        );
    }

    #[test]
    fn test_lossless_trivia_tsx_roundtrip() {
        let original = r#"
"use client";
import React, { useState } from 'react';

// Main interactive card component
export function UserProfileCard({ title }: { title: string }) {
    const [open, setOpen] = useState(false);
    return (
        <div className="card p-4">
            {/* Header section */}
            <h1>{title}</h1>
            <button onClick={() => setOpen(!open)}>Toggle</button>
        </div>
    );
}
"#;
        let cst = parse_to_cst(original);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            reconstructed, original,
            "Reconstructed TSX CST text must match original 100% byte-for-byte"
        );
    }

    #[test]
    fn test_lossless_trivia_python_roundtrip() {
        let original = r#"
# Python worker script
def calculate_metrics(items: list[int]) -> int:
    """Computes aggregate sum."""
    total = 0
    for x in items:
        total += x
    return total
"#;
        let cst = parse_to_cst(original);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            reconstructed, original,
            "Reconstructed Python CST text must match original 100% byte-for-byte"
        );
    }

    #[test]
    fn test_cst_hierarchical_navigation() {
        let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub() {}";
        let root = parse_to_cst(code);

        assert_eq!(root.kind(), SyntaxKind::Root);
        let children = root.children();
        assert_eq!(children.len(), 2);

        let first_fn = &children[0];
        assert_eq!(first_fn.kind(), SyntaxKind::FunctionDecl);
        assert_eq!(first_fn.offset(), 0);

        let second_fn = &children[1];
        assert_eq!(second_fn.kind(), SyntaxKind::FunctionDecl);
        assert!(second_fn.offset() > 0);

        // Sibling navigation
        assert_eq!(first_fn.next_sibling(), Some(second_fn.clone()));
        assert_eq!(second_fn.prev_sibling(), Some(first_fn.clone()));

        // Parent navigation
        assert_eq!(first_fn.parent(), Some(root.clone()));
        assert_eq!(second_fn.parent(), Some(root.clone()));
    }

    #[test]
    fn test_cst_token_at_offset() {
        let code = "fn compute() { 42 }";
        let root = parse_to_cst(code);

        let token_fn = root.token_at_offset(1).expect("Token at offset 1 expected");
        assert_eq!(token_fn.kind(), SyntaxKind::FnKw);
        assert_eq!(token_fn.text(), "fn");

        let token_lit = root.token_at_offset(15).expect("Token at offset 15 expected");
        assert_eq!(token_lit.kind(), SyntaxKind::IntLiteral);
        assert_eq!(token_lit.text(), "42");
    }

    #[test]
    fn test_green_node_content_addressed_immutability() {
        let token1 = GreenToken::new(SyntaxKind::Ident, "foo");
        let token2 = GreenToken::new(SyntaxKind::Ident, "foo");
        let node1 = GreenNode::new(SyntaxKind::Param, vec![GreenElement::Token(token1)]);
        let node2 = GreenNode::new(SyntaxKind::Param, vec![GreenElement::Token(token2)]);

        assert_eq!(node1.content_hash(), node2.content_hash());
        assert_eq!(node1, node2);
    }
}
