//! SymbolGraph — Cross-file semantic dependency and symbol resolver (Rust, TS/JS, Python).

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use regex::Regex;

use crate::types::{fnv1a_64, EdgeKind, Language, SymbolEdge, SymbolKind, SymbolNode};

// ---------------------------------------------------------------------------
// Polyglot Extraction Regexes (LazyLock)
// ---------------------------------------------------------------------------

// Rust patterns
static RE_RS_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]+\))?\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]+\))?\s+)?enum\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]+\))?\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]+\))?\s+)?type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

// TypeScript / JavaScript / Frontend patterns
static RE_TS_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});
static RE_TS_CONST_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*(?::\s*[^=]+)?\s*=\s*(?:async\s*)?(?:\([^)]*\)|[a-zA-Z_$][a-zA-Z0-9_$]*)\s*=>").unwrap()
});
static RE_TS_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+(?:default\s+)?)?class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});
static RE_TS_INTERFACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+)?interface\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});
static RE_TS_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+)?type\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

// Python patterns
static RE_PY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_PY_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});

// ---------------------------------------------------------------------------
// SymbolGraph Implementation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct SymbolGraph {
    pub nodes: HashMap<u64, SymbolNode>,
    pub edges: Vec<SymbolEdge>,
    pub file_to_symbols: HashMap<String, Vec<u64>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Indexes an entire directory tree recursively.
    pub fn index_directory<P: AsRef<Path>>(root: P) -> Self {
        let mut graph = Self::new();
        let root_path = root.as_ref();
        graph.scan_recursive(root_path, root_path);
        graph.resolve_edges();
        graph
    }

    fn scan_recursive(&mut self, current: &Path, root: &Path) {
        let entries = match fs::read_dir(current) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip hidden, target, and node_modules
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                self.scan_recursive(&path, root);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let lang = Language::from_extension(ext);
                if lang != Language::Unknown {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                        self.index_file_content(&rel_path, &content, lang);
                    }
                }
            }
        }
    }

    /// Indexes symbols from a single file's content.
    pub fn index_file_content(&mut self, rel_path: &str, content: &str, lang: Language) {
        let extracted = match lang {
            Language::Rust => Self::extract_rust_symbols(rel_path, content),
            l if l.is_frontend() => Self::extract_ts_symbols(rel_path, content),
            Language::Python => Self::extract_py_symbols(rel_path, content),
            _ => Vec::new(),
        };

        let mut node_ids = Vec::with_capacity(extracted.len());
        for node in extracted {
            let id = node.id;
            self.nodes.insert(id, node);
            node_ids.push(id);
        }

        self.file_to_symbols.insert(rel_path.to_string(), node_ids);
    }

    fn extract_rust_symbols(file: &str, content: &str) -> Vec<SymbolNode> {
        let mut symbols = Vec::new();

        let patterns = [
            (&*RE_RS_FN, SymbolKind::Function),
            (&*RE_RS_STRUCT, SymbolKind::Struct),
            (&*RE_RS_ENUM, SymbolKind::Enum),
            (&*RE_RS_TRAIT, SymbolKind::Trait),
            (&*RE_RS_TYPE, SymbolKind::TypeAlias),
        ];

        for (re, kind) in patterns {
            for cap in re.captures_iter(content) {
                if let (Some(full_match), Some(name_match)) = (cap.get(0), cap.get(1)) {
                    let name = name_match.as_str().to_string();
                    let start = full_match.start();
                    let end = Self::find_closing_boundary(content, start);
                    let sig = if let Some(brace_idx) = content[start..end].find('{') {
                        content[start..start + brace_idx].trim().to_string()
                    } else if let Some(semi_idx) = content[start..end].find(';') {
                        content[start..start + semi_idx].trim().to_string()
                    } else {
                        full_match.as_str().to_string()
                    };
                    let id = fnv1a_64(format!("{}:{}:{:?}", file, name, kind).as_bytes());

                    symbols.push(SymbolNode {
                        id,
                        name,
                        kind: kind.clone(),
                        file: file.to_string(),
                        byte_start: start,
                        byte_end: end,
                        signature: sig,
                    });
                }
            }
        }

        symbols
    }

    fn extract_ts_symbols(file: &str, content: &str) -> Vec<SymbolNode> {
        let mut symbols = Vec::new();

        let patterns = [
            (&*RE_TS_FN, SymbolKind::Function),
            (&*RE_TS_CONST_FN, SymbolKind::Function),
            (&*RE_TS_CLASS, SymbolKind::Struct),
            (&*RE_TS_INTERFACE, SymbolKind::Trait),
            (&*RE_TS_TYPE, SymbolKind::TypeAlias),
        ];

        for (re, kind) in patterns {
            for cap in re.captures_iter(content) {
                if let (Some(full_match), Some(name_match)) = (cap.get(0), cap.get(1)) {
                    let name = name_match.as_str().to_string();
                    let start = full_match.start();
                    let end = Self::find_closing_boundary(content, start);
                    let sig_end = Self::find_signature_end(&content[start..end]);
                    let sig = content[start..start + sig_end].trim().to_string();
                    let id = fnv1a_64(format!("{}:{}:{:?}", file, name, kind).as_bytes());

                    symbols.push(SymbolNode {
                        id,
                        name,
                        kind: kind.clone(),
                        file: file.to_string(),
                        byte_start: start,
                        byte_end: end,
                        signature: sig,
                    });
                }
            }
        }

        symbols
    }

    fn extract_py_symbols(file: &str, content: &str) -> Vec<SymbolNode> {
        let mut symbols = Vec::new();

        let patterns = [
            (&*RE_PY_DEF, SymbolKind::Function),
            (&*RE_PY_CLASS, SymbolKind::Struct),
        ];

        for (re, kind) in patterns {
            for cap in re.captures_iter(content) {
                if let (Some(full_match), Some(name_match)) = (cap.get(0), cap.get(1)) {
                    let name = name_match.as_str().to_string();
                    let start = full_match.start();
                    let end = Self::find_py_closing_boundary(content, start);
                    let sig = if let Some(colon_idx) = content[start..end].find(':') {
                        content[start..start + colon_idx].trim().to_string()
                    } else {
                        full_match.as_str().to_string()
                    };
                    let id = fnv1a_64(format!("{}:{}:{:?}", file, name, kind).as_bytes());

                    symbols.push(SymbolNode {
                        id,
                        name,
                        kind: kind.clone(),
                        file: file.to_string(),
                        byte_start: start,
                        byte_end: end,
                        signature: sig,
                    });
                }
            }
        }

        symbols
    }

    fn find_signature_end(slice: &str) -> usize {
        let mut paren_depth = 0;
        let mut in_str = false;
        let mut quote = '"';
        let mut prev = '\0';

        for (i, ch) in slice.char_indices() {
            if ch == '\n' && in_str && (quote == '\'' || quote == '"') {
                in_str = false;
            }

            if (ch == '"' || ch == '\'' || ch == '`') && prev != '\\' {
                if ch == '\'' && !in_str {
                    let rest = &slice[i + ch.len_utf8()..];
                    let is_char_lit = (rest.chars().nth(1) == Some('\'') && rest.chars().next() != Some('\\'))
                        || (rest.starts_with('\\') && rest.chars().nth(2) == Some('\''));
                    if !is_char_lit && (prev == '&' || prev == '<' || prev == ',' || prev == ' ' || prev == '(') {
                        // Rust lifetime like &'a or <'a> — ignore
                        prev = ch;
                        continue;
                    }
                }

                if in_str && ch == quote {
                    in_str = false;
                } else if !in_str {
                    in_str = true;
                    quote = ch;
                }
                prev = ch;
                continue;
            }
            if in_str {
                prev = ch;
                continue;
            }

            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                if paren_depth > 0 { paren_depth -= 1; }
            } else if paren_depth == 0 {
                if ch == '{' || ch == ';' {
                    return i;
                }
            }
            prev = ch;
        }
        slice.len()
    }

    fn find_closing_boundary(content: &str, start: usize) -> usize {
        let slice = &content[start..];
        let mut depth = 0;
        let mut paren_depth = 0;
        let mut found_open = false;
        let mut in_str = false;
        let mut quote = '"';
        let mut prev = '\0';

        for (i, ch) in slice.char_indices() {
            if ch == '\n' && in_str && (quote == '\'' || quote == '"') {
                in_str = false;
            }

            if (ch == '"' || ch == '\'' || ch == '`') && prev != '\\' {
                if ch == '\'' && !in_str {
                    let rest = &slice[i + ch.len_utf8()..];
                    let is_char_lit = (rest.chars().nth(1) == Some('\'') && rest.chars().next() != Some('\\'))
                        || (rest.starts_with('\\') && rest.chars().nth(2) == Some('\''));
                    if !is_char_lit && (prev == '&' || prev == '<' || prev == ',' || prev == ' ' || prev == '(') {
                        // Rust lifetime like &'a or <'a> — ignore
                        prev = ch;
                        continue;
                    }
                }

                if in_str && ch == quote {
                    in_str = false;
                } else if !in_str {
                    in_str = true;
                    quote = ch;
                }
                prev = ch;
                continue;
            }
            if in_str {
                prev = ch;
                continue;
            }

            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            } else if ch == '{' {
                if paren_depth == 0 {
                    depth += 1;
                    found_open = true;
                }
            } else if ch == '}' {
                if paren_depth == 0 && found_open {
                    depth -= 1;
                    if depth == 0 {
                        return start + i + 1;
                    }
                }
            } else if ch == ';' && !found_open && paren_depth == 0 {
                return start + i + 1;
            }
            prev = ch;
        }

        content.len()
    }

    fn find_py_closing_boundary(content: &str, start: usize) -> usize {
        let slice = &content[start..];
        let mut lines = slice.lines();
        let first_line = lines.next().unwrap_or("");
        let indent = first_line.chars().take_while(|c| c.is_whitespace()).count();

        let mut current_offset = first_line.len();
        for line in lines {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                current_offset += line.len() + 1;
                continue;
            }
            let line_indent = line.chars().take_while(|c| c.is_whitespace()).count();
            if line_indent <= indent {
                return start + current_offset;
            }
            current_offset += line.len() + 1;
        }

        content.len()
    }

    fn resolve_edges(&mut self) {
        let mut name_to_id: HashMap<String, u64> = HashMap::new();
        for (id, node) in &self.nodes {
            name_to_id.insert(node.name.clone(), *id);
        }

        let mut edges = Vec::new();
        for (id, node) in &self.nodes {
            for (target_name, target_id) in &name_to_id {
                if target_id != id && node.signature.contains(target_name) {
                    edges.push(SymbolEdge {
                        from_id: *id,
                        to_id: *target_id,
                        edge_type: EdgeKind::Uses,
                    });
                }
            }
        }

        self.edges = edges;
    }

    /// Calculates token / byte reduction efficiency between original code and skeleton.
    pub fn calculate_token_savings(original: &str, skeleton: &str) -> f64 {
        let orig_len = original.trim().len();
        if orig_len == 0 {
            return 0.0;
        }
        let skel_len = skeleton.trim().len();
        if skel_len >= orig_len {
            0.0
        } else {
            ((orig_len - skel_len) as f64 / orig_len as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_symbols() {
        let code = r#"
pub struct User {
    pub id: u64,
    pub name: String,
}

pub async fn fetch_user(id: u64) -> Option<User> {
    None
}
"#;
        let mut graph = SymbolGraph::new();
        graph.index_file_content("models.rs", code, Language::Rust);

        assert_eq!(graph.nodes.len(), 2);
        let names: Vec<String> = graph.nodes.values().map(|n| n.name.clone()).collect();
        assert!(names.contains(&"User".to_string()));
        assert!(names.contains(&"fetch_user".to_string()));
    }

    #[test]
    fn test_token_savings_calculation() {
        let full = "pub fn heavy_computation() -> u64 {\n    let mut x = 0;\n    for i in 0..1000 { x += i; }\n    x\n}";
        let skel = "pub fn heavy_computation() -> u64;";
        let savings = SymbolGraph::calculate_token_savings(full, skel);
        assert!(savings > 50.0, "Expected >50% savings, got {}%", savings);
    }
}
