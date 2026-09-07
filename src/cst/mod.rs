//! Lossless Concrete Syntax Tree (CST) Module.
//!
//! Implements a 100% safe, pure-Rust Green/Red Tree architecture for full trivia preservation,
//! sub-microsecond incremental node navigation, and byte-span accuracy.

#![forbid(unsafe_code)]

pub mod green;
pub mod lexer;
pub mod polyglot;
pub mod red;

pub use green::{
    GreenElement, GreenNode, GreenNodeBuilder, GreenToken, SyntaxKind, TextRange, TextSize,
};
pub use lexer::tokenize;
pub use polyglot::{
    CCppCstDriver, CfgEdge, CfgEdgeKind, CfgNode, CfgNodeKind, ControlFlowGraph, CstParseError,
    DartCstDriver, GenericPolyglotDriver, GoCstDriver, GreenTreeRoot, InvariantBitset, JavaCstDriver,
    LanguageId, PhpCstDriver, PythonCstDriver, RustCstDriver, TreeStats, TypeScriptCstDriver,
    UniversalCstDriver, UniversalCstDriverExt, UniversalDriverRegistry, GLOBAL_REGISTRY,
};
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

        // Track declarations to group them into GreenNode parents
        if !in_decl {
            if kind == SyntaxKind::FnKw || kind == SyntaxKind::DefKw {
                builder.start_node(SyntaxKind::FunctionDecl);
                in_decl = true;
            } else if kind == SyntaxKind::StructKw || kind == SyntaxKind::ClassKw {
                builder.start_node(SyntaxKind::StructDecl);
                in_decl = true;
            } else if kind == SyntaxKind::EnumKw {
                builder.start_node(SyntaxKind::EnumDecl);
                in_decl = true;
            } else if kind == SyntaxKind::TraitKw || kind == SyntaxKind::InterfaceKw {
                builder.start_node(SyntaxKind::TraitDecl);
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

    builder.finish_node(); // Root
    builder.finish()
}

/// Full end-to-end parser producing a navigable, red `SyntaxNode`.
pub fn parse_to_cst(source: &str) -> SyntaxNode {
    let green = parse_to_green(source);
    SyntaxNode::new_root(green)
}

/// Reconstructs the complete, byte-for-byte lossless source string from a `SyntaxNode`.
pub fn to_lossless_text(node: &SyntaxNode) -> String {
    node.text()
}

// ---------------------------------------------------------------------------
// CST Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossless_trivia_rust_roundtrip() {
        let source = r#"
/// Universal matrix accelerator
#[inline(always)]
pub fn compute<T: Real>(matrix: &[T], dim: usize) -> Result<Vec<T>, LocusError> {
    // Check dimensions
    if dim == 0 {
        return Err(LocusError::EmptyMatrix);
    }
    let mut acc = Vec::with_capacity(dim);
    for row in matrix.chunks(dim) {
        acc.push(row.iter().sum());
    }
    Ok(acc)
}
"#;
        let cst = parse_to_cst(source);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            source, reconstructed,
            "CST must produce a 100% byte-for-byte lossless round-trip."
        );
    }

    #[test]
    fn test_lossless_trivia_tsx_roundtrip() {
        let source = r#"
import React, { useState, useEffect } from 'react';

/** Header component */
export const HeaderWidget: React.FC<Props> = ({ title, count }) => {
    const [active, setActive] = useState(false);

    return (
        <header className="flex flex-row items-center">
            <h1>{title}</h1>
            <span>{count}</span>
        </header>
    );
};
"#;
        let cst = parse_to_cst(source);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            source, reconstructed,
            "TSX CST must maintain exact trivia roundtrip."
        );
    }

    #[test]
    fn test_lossless_trivia_python_roundtrip() {
        let source = r#"
def calculate_metrics(data: list[float]) -> dict:
    # Compute mean and variance
    total = sum(data)
    n = len(data)
    mean = total / n if n > 0 else 0.0
    return {"mean": mean, "count": n}
"#;
        let cst = parse_to_cst(source);
        let reconstructed = to_lossless_text(&cst);
        assert_eq!(
            source, reconstructed,
            "Python CST must maintain exact whitespace and comments."
        );
    }

    #[test]
    fn test_green_node_content_addressed_immutability() {
        let source_a = "pub fn add(x: i32) -> i32 { x + 1 }";
        let source_b = "pub fn add(x: i32) -> i32 { x + 1 }";
        let source_c = "pub fn sub(x: i32) -> i32 { x - 1 }";

        let green_a = parse_to_green(source_a);
        let green_b = parse_to_green(source_b);
        let green_c = parse_to_green(source_c);

        assert_eq!(green_a.content_hash(), green_b.content_hash());
        assert_ne!(green_a.content_hash(), green_c.content_hash());
    }

    #[test]
    fn test_cst_hierarchical_navigation() {
        let source = "fn foo() { let x = 1; } fn bar() { let y = 2; }";
        let root = parse_to_cst(source);

        let functions: Vec<SyntaxNode> = root
            .children()
            .into_iter()
            .filter(|n| n.kind() == SyntaxKind::FunctionDecl)
            .collect();

        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].text_range().start(), 0);
    }

    #[test]
    fn test_cst_token_at_offset() {
        let source = "let alpha = 42;";
        let root = parse_to_cst(source);

        let token_at_alpha = root.token_at_offset(6);
        assert!(token_at_alpha.is_some());
        let tok = token_at_alpha.unwrap();
        assert_eq!(tok.text(), "alpha");
        assert_eq!(tok.kind(), SyntaxKind::Ident);
    }
}
