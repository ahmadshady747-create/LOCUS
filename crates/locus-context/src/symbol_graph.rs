//! AST Symbol Graph and Cross-File Dependency Resolver.
//!
//! Parses and indexes symbols (structs, functions, interfaces, classes, traits)
//! and tracks import/export edges across Rust, TypeScript, and Python codebases.

use crate::types::{SymbolEdge, SymbolKind, SymbolNode};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct SymbolGraph {
    /// Maps symbol name -> list of definition nodes across files
    symbols_by_name: HashMap<String, Vec<SymbolNode>>,
    /// Maps file path -> list of symbols defined in that file
    symbols_by_file: HashMap<String, Vec<SymbolNode>>,
    /// Import edges connecting files to referenced symbols
    import_edges: Vec<SymbolEdge>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Indexes code content of a file based on its file extension.
    pub fn index_file(&mut self, file_path: &Path, content: &str) {
        let path_str = file_path.to_string_lossy().replace('\\', "/");
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Clear previous entries for this file
        if let Some(old_syms) = self.symbols_by_file.remove(&path_str) {
            for sym in old_syms {
                if let Some(vec) = self.symbols_by_name.get_mut(&sym.name) {
                    vec.retain(|s| s.file_path != path_str);
                }
            }
        }
        self.import_edges.retain(|e| e.source_file != path_str);

        let (syms, edges) = match ext.as_str() {
            "rs" => self.parse_rust(&path_str, content),
            "ts" | "tsx" | "js" | "jsx" => self.parse_typescript(&path_str, content),
            "py" | "pyw" => self.parse_python(&path_str, content),
            _ => (vec![], vec![]),
        };

        for sym in &syms {
            self.symbols_by_name
                .entry(sym.name.clone())
                .or_default()
                .push(sym.clone());
        }

        self.symbols_by_file.insert(path_str, syms);
        self.import_edges.extend(edges);
    }

    /// Resolves definitions for a symbol name.
    pub fn find_symbol(&self, symbol_name: &str) -> Vec<SymbolNode> {
        self.symbols_by_name.get(symbol_name).cloned().unwrap_or_default()
    }

    /// Resolves relevant symbol signatures related to a target symbol and file context.
    pub fn resolve_symbol_context(&self, symbol_name: &str, file_path: &Path) -> Vec<SymbolNode> {
        let path_str = file_path.to_string_lossy().replace('\\', "/");
        let mut results = Vec::new();

        // 1. Direct matches for symbol name
        if let Some(direct) = self.symbols_by_name.get(symbol_name) {
            results.extend(direct.clone());
        }

        // 2. Symbols in the same file
        if let Some(file_syms) = self.symbols_by_file.get(&path_str) {
            for s in file_syms {
                if !results.iter().any(|r| r.name == s.name && r.file_path == s.file_path) {
                    results.push(s.clone());
                }
            }
        }

        // 3. Follow import edges from this file
        for edge in self.import_edges.iter().filter(|e| e.source_file == path_str) {
            if let Some(imported_syms) = self.symbols_by_name.get(&edge.target_symbol) {
                for s in imported_syms {
                    if !results.iter().any(|r| r.name == s.name && r.file_path == s.file_path) {
                        results.push(s.clone());
                    }
                }
            }
        }

        results
    }

    /// Returns a compact string of all signatures in a given file.
    pub fn get_file_signatures(&self, file_path: &Path) -> String {
        let path_str = file_path.to_string_lossy().replace('\\', "/");
        if let Some(syms) = self.symbols_by_file.get(&path_str) {
            let sigs: Vec<String> = syms.iter().map(|s| s.signature.clone()).collect();
            sigs.join("\n")
        } else {
            String::new()
        }
    }

    pub fn total_symbols(&self) -> usize {
        self.symbols_by_file.values().map(|v| v.len()).sum()
    }

    pub fn total_edges(&self) -> usize {
        self.import_edges.len()
    }

    // -----------------------------------------------------------------------
    // Rust Parser
    // -----------------------------------------------------------------------
    fn parse_rust(&self, path: &str, content: &str) -> (Vec<SymbolNode>, Vec<SymbolEdge>) {
        let mut symbols = Vec::new();
        let mut edges = Vec::new();

        let re_struct = Regex::new(r"pub(?:\(crate\))?\s+struct\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_enum = Regex::new(r"pub(?:\(crate\))?\s+enum\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_trait = Regex::new(r"pub(?:\(crate\))?\s+trait\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_type = Regex::new(r"pub(?:\(crate\))?\s+type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_fn = Regex::new(r"pub(?:\(crate\))?\s+(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Check imports: use a::b::{c, d}; or use a::b::c;
            if trimmed.starts_with("use ") && trimmed.ends_with(';') {
                let use_body = trimmed.trim_start_matches("use ").trim_end_matches(';').trim();
                if let Some(idx) = use_body.find("::{") {
                    let module = &use_body[..idx];
                    let inside = &use_body[idx + 3..use_body.len().saturating_sub(1)];
                    for item in inside.split(',') {
                        let sym = item.trim().split_whitespace().next().unwrap_or("");
                        if !sym.is_empty() && sym != "*" {
                            edges.push(SymbolEdge {
                                source_file: path.to_string(),
                                target_symbol: sym.to_string(),
                                target_module: module.to_string(),
                            });
                        }
                    }
                } else {
                    let parts: Vec<&str> = use_body.split("::").collect();
                    if let Some(last) = parts.last() {
                        let sym = last.trim();
                        let module = if parts.len() > 1 {
                            parts[..parts.len() - 1].join("::")
                        } else {
                            sym.to_string()
                        };
                        if !sym.is_empty() && sym != "*" {
                            edges.push(SymbolEdge {
                                source_file: path.to_string(),
                                target_symbol: sym.to_string(),
                                target_module: module,
                            });
                        }
                    }
                }
            }

            // Check struct
            if let Some(caps) = re_struct.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Struct,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Check enum
            if let Some(caps) = re_enum.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Enum,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Check trait
            if let Some(caps) = re_trait.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Trait,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Check type alias
            if let Some(caps) = re_type.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::TypeAlias,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches(';').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Check function
            if let Some(caps) = re_fn.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Function,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }
        }

        (symbols, edges)
    }

    // -----------------------------------------------------------------------
    // TypeScript / JavaScript Parser
    // -----------------------------------------------------------------------
    fn parse_typescript(&self, path: &str, content: &str) -> (Vec<SymbolNode>, Vec<SymbolEdge>) {
        let mut symbols = Vec::new();
        let mut edges = Vec::new();

        let re_import = Regex::new(r#"import\s+(?:\{([^}]+)\}|([a-zA-Z_][a-zA-Z0-9_]*))\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        let re_interface = Regex::new(r"export\s+interface\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_type = Regex::new(r"export\s+type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_class = Regex::new(r"export\s+class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_fn = Regex::new(r"export\s+(?:async\s+)?function\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_const_fn = Regex::new(r"export\s+const\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=").unwrap();

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Imports
            if let Some(caps) = re_import.captures(trimmed) {
                let module = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                if let Some(group) = caps.get(1) {
                    for item in group.as_str().split(',') {
                        let sym = item.trim().split_whitespace().next().unwrap_or("");
                        if !sym.is_empty() && sym != "type" {
                            edges.push(SymbolEdge {
                                source_file: path.to_string(),
                                target_symbol: sym.to_string(),
                                target_module: module.to_string(),
                            });
                        }
                    }
                } else if let Some(def) = caps.get(2) {
                    edges.push(SymbolEdge {
                        source_file: path.to_string(),
                        target_symbol: def.as_str().to_string(),
                        target_module: module.to_string(),
                    });
                }
            }

            // Interface
            if let Some(caps) = re_interface.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Interface,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Type
            if let Some(caps) = re_type.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::TypeAlias,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches(';').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Class
            if let Some(caps) = re_class.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Class,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Function
            if let Some(caps) = re_fn.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Function,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches('{').trim().to_string(),
                    doc_comment: None,
                });
            }

            // Export const
            if let Some(caps) = re_const_fn.captures(trimmed) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Constant,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: trimmed.trim_end_matches(';').trim().to_string(),
                    doc_comment: None,
                });
            }
        }

        (symbols, edges)
    }

    // -----------------------------------------------------------------------
    // Python Parser
    // -----------------------------------------------------------------------
    fn parse_python(&self, path: &str, content: &str) -> (Vec<SymbolNode>, Vec<SymbolEdge>) {
        let mut symbols = Vec::new();
        let mut edges = Vec::new();

        let re_from_import = Regex::new(r"from\s+([a-zA-Z0-9_.]+)\s+import\s+([a-zA-Z0-9_,\s]+)").unwrap();
        let re_import = Regex::new(r"import\s+([a-zA-Z0-9_]+)").unwrap();
        let re_class = Regex::new(r"^class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let re_def = Regex::new(r"^(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // from x import a, b
            if let Some(caps) = re_from_import.captures(trimmed) {
                let module = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if let Some(items) = caps.get(2) {
                    for item in items.as_str().split(',') {
                        let sym = item.trim().split_whitespace().next().unwrap_or("");
                        if !sym.is_empty() {
                            edges.push(SymbolEdge {
                                source_file: path.to_string(),
                                target_symbol: sym.to_string(),
                                target_module: module.to_string(),
                            });
                        }
                    }
                }
            } else if let Some(caps) = re_import.captures(trimmed) {
                if let Some(mod_name) = caps.get(1) {
                    edges.push(SymbolEdge {
                        source_file: path.to_string(),
                        target_symbol: mod_name.as_str().to_string(),
                        target_module: mod_name.as_str().to_string(),
                    });
                }
            }

            // Top level Class
            if let Some(caps) = re_class.captures(line) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Class,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: line.trim_end().to_string(),
                    doc_comment: None,
                });
            }

            // Top level Def
            if let Some(caps) = re_def.captures(line) {
                let name = caps.get(1).unwrap().as_str().to_string();
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Function,
                    file_path: path.to_string(),
                    line_number: line_idx + 1,
                    signature: line.trim_end().to_string(),
                    doc_comment: None,
                });
            }
        }

        (symbols, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_symbol_and_import_extraction() {
        let mut graph = SymbolGraph::new();
        let code = r#"
            use std::collections::{HashMap, HashSet};
            use crate::types::SymbolNode;

            pub struct CognitiveRouter {
                pub id: String,
            }

            pub enum RouterStrategy {
                Speed,
                Power,
            }

            pub async fn route_task(task: &str) -> String {
                "done".to_string()
            }
        "#;

        graph.index_file(Path::new("src/router.rs"), code);

        assert_eq!(graph.total_symbols(), 3);
        let router = graph.find_symbol("CognitiveRouter");
        assert_eq!(router.len(), 1);
        assert_eq!(router[0].kind, SymbolKind::Struct);

        let edges = &graph.import_edges;
        assert!(edges.iter().any(|e| e.target_symbol == "HashMap"));
        assert!(edges.iter().any(|e| e.target_symbol == "SymbolNode"));
    }

    #[test]
    fn test_typescript_symbol_extraction() {
        let mut graph = SymbolGraph::new();
        let code = r#"
            import { useState, useEffect } from "react";
            import type { AppState } from "../types";

            export interface TaskProps {
                id: string;
            }

            export const computeScore = (val: number) => val * 2;
        "#;

        graph.index_file(Path::new("src/components/Task.tsx"), code);

        let props = graph.find_symbol("TaskProps");
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].kind, SymbolKind::Interface);
    }
}
