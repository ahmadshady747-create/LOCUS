//! AstDiffEngine — Surgical node-level byte-span AST patching and skeleton extraction.

use crate::graph::SymbolGraph;
use crate::types::Language;
use thiserror::Error;

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
    pub fn patch(
        source: &str,
        symbol_name: &str,
        new_code: &str,
        lang: Language,
    ) -> Result<String, DiffError> {
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
        if lang.is_frontend() {
            return Self::skeletonize_frontend(source);
        }

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
                Language::Python => {
                    skeleton.push_str(&node.signature);
                    skeleton.push_str(":\n    ...\n");
                }
                _ => {
                    skeleton.push_str(&node.signature);
                    skeleton.push_str(";\n");
                }
            }
        }

        skeleton
    }

    /// Specialized frontend skeletonizer preserving imports, interfaces, types, and collapsing JSX render trees.
    fn skeletonize_frontend(source: &str) -> String {
        let mut skeleton = String::with_capacity(source.len() / 4);

        // 1. Preserve all import statements
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ")
                || trimmed.starts_with("\"use client\"")
                || trimmed.starts_with("'use client'")
            {
                skeleton.push_str(trimmed);
                skeleton.push('\n');
            }
        }

        if !skeleton.is_empty() {
            skeleton.push('\n');
        }

        // 2. Extract types and interfaces
        let mut graph = SymbolGraph::new();
        graph.index_file_content("target", source, Language::Tsx);

        let mut sorted_nodes: Vec<_> = graph.nodes.values().collect();
        sorted_nodes.sort_by_key(|n| n.byte_start);

        for node in sorted_nodes {
            match node.kind {
                crate::types::SymbolKind::Trait | crate::types::SymbolKind::TypeAlias => {
                    // Extract full interface/type definition from source
                    let decl = &source[node.byte_start..node.byte_end];
                    skeleton.push_str(decl.trim());
                    skeleton.push_str("\n\n");
                }
                crate::types::SymbolKind::Function | crate::types::SymbolKind::Struct => {
                    // Check if signature contains JSX / Component render
                    let sig = &node.signature;
                    let is_component = node
                        .name
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false);

                    if is_component
                        || sig.contains("JSX.Element")
                        || sig.contains("ReactNode")
                        || sig.contains("React.FC")
                    {
                        // Count rough JSX tags inside the component body
                        let body = &source[node.byte_start..node.byte_end];
                        let tag_count = body.matches('<').count().max(1);

                        skeleton.push_str(sig);
                        skeleton.push_str(" {\n    // [JSX: ~");
                        skeleton.push_str(&tag_count.to_string());
                        skeleton.push_str(" render nodes collapsed for token optimization]\n}\n\n");
                    } else {
                        skeleton.push_str(sig);
                        if !sig.ends_with(';') {
                            skeleton.push_str(";\n");
                        } else {
                            skeleton.push('\n');
                        }
                    }
                }
                _ => {
                    skeleton.push_str(&node.signature);
                    skeleton.push_str(";\n");
                }
            }
        }

        skeleton.trim_end().to_string()
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

    #[test]
    fn test_skeletonize_react_tsx() {
        let component = r#"
"use client";
import React, { useState } from 'react';
import { Button } from '@/components/ui/button';

export interface UserTableProps {
    users: Array<{ id: string; name: string }>;
    onSelect: (id: string) => void;
}

export function UserTable({ users, onSelect }: UserTableProps) {
    const [selected, setSelected] = useState<string | null>(null);

    const handleRowClick = (id: string) => {
        setSelected(id);
        onSelect(id);
    };

    return (
        <div className="overflow-x-auto">
            <table className="min-w-full">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Name</th>
                    </tr>
                </thead>
                <tbody>
                    {users.map(u => (
                        <tr key={u.id} onClick={() => handleRowClick(u.id)}>
                            <td>{u.id}</td>
                            <td>{u.name}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
"#;
        let skeleton = AstDiffEngine::skeletonize(component, Language::Tsx);
        assert!(skeleton.contains("import React, { useState } from 'react';"));
        assert!(skeleton.contains("export interface UserTableProps"));
        assert!(
            skeleton.contains("export function UserTable({ users, onSelect }: UserTableProps) {")
        );
        assert!(skeleton.contains("// [JSX: ~"));
        assert!(!skeleton.contains("<thead>"));
        assert!(skeleton.len() < component.len() / 2);
    }

    #[test]
    fn test_patch_frontend_event_handler() {
        let source = r#"
export function Form() {
    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        console.log("old submit");
    };

    return <form onSubmit={handleSubmit}><button>Submit</button></form>;
}
"#;
        let new_handler = "const handleSubmit = async (e: React.FormEvent) => {\n        e.preventDefault();\n        await api.post('/data');\n    };";
        let patched = AstDiffEngine::patch(source, "handleSubmit", new_handler, Language::Tsx)
            .expect("Frontend patch failed");

        assert!(patched.contains("await api.post('/data');"));
        assert!(patched.contains("<form onSubmit={handleSubmit}>"));
    }
}
