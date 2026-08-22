//! SymbolGraph — Cross-file semantic dependency resolver, cross-module reference linking,
//! blast-radius impact analysis, and architectural health engine (Rust, TS/JS, Python).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Instant;
use regex::Regex;

use crate::types::{
    fnv1a_64, ArchitecturalHealth, BlastRadiusReport, EdgeKind, Language, ResolvedSymbol, RiskScore,
    SymbolEdge, SymbolKind, SymbolNode, SymbolReference,
};

// ---------------------------------------------------------------------------
// Polyglot Extraction Regexes (LazyLock)
// ---------------------------------------------------------------------------

// Rust patterns
static RE_RS_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_ENUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?enum\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_RS_USE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?use\s+([^;]+);").unwrap()
});
static RE_RS_MOD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*);").unwrap()
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
static RE_TS_IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*import\s+[^;\n]+?from\s*['"]([^'"]+)['"]"#).unwrap()
});
static RE_TS_EXPORT_FROM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*export\s+(?:\*|\{[^}]*\})\s*from\s*['"]([^'"]+)['"]"#).unwrap()
});

// Python patterns
static RE_PY_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_PY_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static RE_PY_IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:from\s+([a-zA-Z0-9_.]+)\s+import\s+[^#\n]+|import\s+([a-zA-Z0-9_.]+))").unwrap()
});

// ---------------------------------------------------------------------------
// SymbolGraph Implementation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct SymbolGraph {
    pub nodes: HashMap<u64, SymbolNode>,
    pub edges: Vec<SymbolEdge>,
    pub file_to_symbols: HashMap<String, Vec<u64>>,
    pub symbol_references: HashMap<String, Vec<SymbolReference>>,
    pub file_imports: HashMap<String, Vec<String>>,
    pub file_contents: HashMap<String, String>,
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
        graph.index_references();
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
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" || file_name == "dist" || file_name == "build" {
                continue;
            }

            if path.is_dir() {
                self.scan_recursive(&path, root);
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let lang = Language::from_extension(ext);
                if lang != Language::Unknown {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                        self.index_file_content(&rel_path, &content, lang);
                    }
                }
            }
        }
    }

    /// Indexes symbols and module dependencies from a single file's content.
    pub fn index_file_content(&mut self, rel_path: &str, content: &str, lang: Language) {
        let normalized_path = rel_path.replace('\\', "/");
        self.file_contents.insert(normalized_path.clone(), content.to_string());

        let extracted = match lang {
            Language::Rust => Self::extract_rust_symbols(&normalized_path, content),
            l if l.is_frontend() => Self::extract_ts_symbols(&normalized_path, content),
            Language::Python => Self::extract_py_symbols(&normalized_path, content),
            _ => Vec::new(),
        };

        let mut node_ids = Vec::with_capacity(extracted.len());
        for node in extracted {
            let id = node.id;
            self.nodes.insert(id, node);
            node_ids.push(id);
        }
        self.file_to_symbols.insert(normalized_path.clone(), node_ids);

        // Extract module imports
        let imports = Self::extract_file_imports(&normalized_path, content, lang);
        self.file_imports.insert(normalized_path, imports);
    }

    // --- Import / Module Resolution ---

    fn extract_file_imports(file: &str, content: &str, lang: Language) -> Vec<String> {
        let mut imports = Vec::new();
        match lang {
            Language::Rust => {
                for cap in RE_RS_USE.captures_iter(content) {
                    if let Some(m) = cap.get(1) {
                        let path = m.as_str().trim().replace("crate::", "").replace("super::", "");
                        imports.push(path);
                    }
                }
                for cap in RE_RS_MOD.captures_iter(content) {
                    if let Some(m) = cap.get(1) {
                        imports.push(format!("{}.rs", m.as_str()));
                    }
                }
            }
            l if l.is_frontend() => {
                for cap in RE_TS_IMPORT.captures_iter(content) {
                    if let Some(m) = cap.get(1) {
                        let target = Self::normalize_relative_import(file, m.as_str());
                        imports.push(target);
                    }
                }
                for cap in RE_TS_EXPORT_FROM.captures_iter(content) {
                    if let Some(m) = cap.get(1) {
                        let target = Self::normalize_relative_import(file, m.as_str());
                        imports.push(target);
                    }
                }
            }
            Language::Python => {
                for cap in RE_PY_IMPORT.captures_iter(content) {
                    if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
                        imports.push(m.as_str().to_string());
                    }
                }
            }
            _ => {}
        }
        imports
    }

    fn normalize_relative_import(current_file: &str, import_spec: &str) -> String {
        if import_spec.starts_with('.') {
            let parent = Path::new(current_file).parent().unwrap_or(Path::new(""));
            let resolved = parent.join(import_spec);
            let s = resolved.to_string_lossy().replace('\\', "/");
            if s.starts_with("./") {
                s[2..].to_string()
            } else {
                s
            }
        } else {
            import_spec.to_string()
        }
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

    /// Resolves cross-symbol dependency edges across all indexed nodes.
    pub fn resolve_edges(&mut self) {
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

    /// Indexes word-boundary references across all indexed files.
    pub fn index_references(&mut self) {
        let mut refs: HashMap<String, Vec<SymbolReference>> = HashMap::new();

        let symbol_names: HashSet<String> = self.nodes.values().map(|n| n.name.clone()).collect();

        for (file, content) in &self.file_contents {
            let mut byte_offset = 0;
            for (line_idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if !trimmed.starts_with("//") && !trimmed.starts_with('#') && !trimmed.starts_with('*') {
                    for sym in &symbol_names {
                        if line.contains(sym) {
                            let is_word = Self::is_word_match(line, sym);
                            if is_word {
                                let r = SymbolReference {
                                    file: file.clone(),
                                    line: line_idx + 1,
                                    byte_offset,
                                    context_snippet: trimmed.to_string(),
                                };
                                refs.entry(sym.clone()).or_default().push(r);
                            }
                        }
                    }
                }
                byte_offset += line.len() + 1;
            }
        }

        self.symbol_references = refs;
    }

    fn is_word_match(line: &str, word: &str) -> bool {
        let mut start = 0;
        while let Some(idx) = line[start..].find(word) {
            let actual_idx = start + idx;
            let before_ok = if actual_idx == 0 {
                true
            } else {
                let c = line[..actual_idx].chars().last().unwrap_or(' ');
                !c.is_alphanumeric() && c != '_'
            };
            let after_idx = actual_idx + word.len();
            let after_ok = if after_idx >= line.len() {
                true
            } else {
                let c = line[after_idx..].chars().next().unwrap_or(' ');
                !c.is_alphanumeric() && c != '_'
            };

            if before_ok && after_ok {
                return true;
            }
            start = actual_idx + word.len().max(1);
        }
        false
    }

    // ---------------------------------------------------------------------------
    // Public Query Interfaces: Cross-Module Resolution & Blast Radius
    // ---------------------------------------------------------------------------

    /// Fully resolves symbol origin, byte-span, signature, and doc-comments across module paths.
    pub fn resolve_symbol(&self, symbol: &str, from_file: Option<&str>) -> Option<ResolvedSymbol> {
        // 1. If from_file is given, prioritize local definitions or explicit imports
        if let Some(file) = from_file {
            let normalized = file.replace('\\', "/");
            if let Some(node_ids) = self.file_to_symbols.get(&normalized) {
                for id in node_ids {
                    if let Some(node) = self.nodes.get(id) {
                        if node.name == symbol {
                            return Some(self.build_resolved_symbol(node));
                        }
                    }
                }
            }
        }

        // 2. Global search across symbol graph
        if let Some(node) = self.nodes.values().find(|n| n.name == symbol) {
            return Some(self.build_resolved_symbol(node));
        }

        None
    }

    fn build_resolved_symbol(&self, node: &SymbolNode) -> ResolvedSymbol {
        let doc_comment = if let Some(content) = self.file_contents.get(&node.file) {
            let before = &content[..node.byte_start];
            let mut docs = Vec::new();
            for line in before.lines().rev() {
                let trimmed = line.trim();
                if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with('*') {
                    docs.push(trimmed.to_string());
                } else if trimmed.is_empty() {
                    continue;
                } else {
                    break;
                }
            }
            if docs.is_empty() {
                None
            } else {
                docs.reverse();
                Some(docs.join("\n"))
            }
        } else {
            None
        };

        let is_exported = node.signature.starts_with("pub ")
            || node.signature.starts_with("pub(")
            || node.signature.starts_with("export ")
            || node.signature.contains("export function")
            || node.signature.contains("export const");

        ResolvedSymbol {
            name: node.name.clone(),
            kind: node.kind.clone(),
            file: node.file.clone(),
            byte_start: node.byte_start,
            byte_end: node.byte_end,
            signature: node.signature.clone(),
            doc_comment,
            is_exported,
        }
    }

    /// Finds all references and call sites of a symbol across the entire indexed codebase.
    pub fn find_references(&self, symbol: &str) -> Vec<SymbolReference> {
        self.symbol_references.get(symbol).cloned().unwrap_or_default()
    }

    /// Computes direct and transitive blast radius impact when modifying a symbol.
    pub fn calculate_blast_radius(&self, symbol: &str, file: Option<&str>, depth: usize) -> BlastRadiusReport {
        let start = Instant::now();

        let resolved = self.resolve_symbol(symbol, file);
        let origin_file = resolved.as_ref().map(|r| r.file.clone()).unwrap_or_else(|| file.unwrap_or("unknown").to_string());

        let target_node = self.nodes.values().find(|n| n.name == symbol && (file.is_none() || n.file == origin_file));
        let target_id = target_node.map(|n| n.id);

        let mut direct_dependents = Vec::new();
        let mut transitive_dependents = Vec::new();
        let mut affected_files_set = HashSet::new();

        // 1. Check graph edges (reverse dependencies)
        if let Some(tid) = target_id {
            let mut visited: HashSet<u64> = HashSet::new();
            let mut queue: VecDeque<(u64, usize)> = VecDeque::new();

            visited.insert(tid);
            queue.push_back((tid, 0));

            while let Some((curr_id, curr_depth)) = queue.pop_front() {
                if curr_depth >= depth {
                    continue;
                }

                for edge in &self.edges {
                    if edge.to_id == curr_id && !visited.contains(&edge.from_id) {
                        visited.insert(edge.from_id);
                        if let Some(caller) = self.nodes.get(&edge.from_id) {
                            affected_files_set.insert(caller.file.clone());
                            if curr_depth == 0 {
                                direct_dependents.push(format!("{} ({})", caller.name, caller.file));
                            } else {
                                transitive_dependents.push(format!("{} ({})", caller.name, caller.file));
                            }
                        }
                        queue.push_back((edge.from_id, curr_depth + 1));
                    }
                }
            }
        }

        // 2. Check reference sites from other files
        if let Some(refs) = self.symbol_references.get(symbol) {
            for r in refs {
                if r.file != origin_file {
                    affected_files_set.insert(r.file.clone());
                    let desc = format!("{}:L{}", r.file, r.line);
                    if !direct_dependents.contains(&desc) {
                        direct_dependents.push(desc);
                    }
                }
            }
        }

        let reference_count = self.symbol_references.get(symbol).map(|v| v.len()).unwrap_or(0);
        let mut affected_files: Vec<String> = affected_files_set.into_iter().collect();
        affected_files.sort();

        // 3. Determine Risk Score
        let risk_score = if reference_count > 15 || affected_files.len() > 5 {
            RiskScore::Critical
        } else if reference_count >= 6 || affected_files.len() >= 3 {
            RiskScore::High
        } else if reference_count >= 2 || !affected_files.is_empty() {
            RiskScore::Medium
        } else {
            RiskScore::Low
        };

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        BlastRadiusReport {
            symbol: symbol.to_string(),
            origin_file,
            direct_dependents,
            transitive_dependents,
            affected_files,
            risk_score,
            reference_count,
            latency_ms,
        }
    }

    /// Detects circular import dependencies across workspace modules.
    pub fn detect_import_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for file in self.file_imports.keys() {
            if !visited.contains(file) {
                Self::dfs_cycle_check(
                    file,
                    &self.file_imports,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_cycle_check(
        current: &str,
        imports: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(current.to_string());
        rec_stack.insert(current.to_string());
        path.push(current.to_string());

        if let Some(deps) = imports.get(current) {
            for dep in deps {
                let dep_file = if imports.contains_key(dep) {
                    dep.clone()
                } else {
                    imports.keys().find(|k| k.ends_with(dep) || dep.ends_with(*k)).cloned().unwrap_or_default()
                };

                if !dep_file.is_empty() {
                    if !visited.contains(&dep_file) {
                        Self::dfs_cycle_check(&dep_file, imports, visited, rec_stack, path, cycles);
                    } else if rec_stack.contains(&dep_file) {
                        let mut cycle = Vec::new();
                        let start_pos = path.iter().position(|p| p == &dep_file).unwrap_or(0);
                        for node in &path[start_pos..] {
                            cycle.push(node.clone());
                        }
                        cycle.push(dep_file.clone());
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(current);
    }

    /// Identifies exported public symbols with zero inbound references across other workspace files.
    pub fn find_orphan_exports(&self) -> Vec<String> {
        let mut orphans = Vec::new();

        for node in self.nodes.values() {
            let is_exported = node.signature.starts_with("pub ")
                || node.signature.starts_with("pub(")
                || node.signature.starts_with("export ")
                || node.signature.contains("export function")
                || node.signature.contains("export const");

            if is_exported {
                let refs = self.symbol_references.get(&node.name);
                let external_refs = refs.map(|v| v.iter().filter(|r| r.file != node.file).count()).unwrap_or(0);

                if external_refs == 0 && node.name != "main" && node.name != "default" {
                    orphans.push(format!("{} ({}:{})", node.name, node.file, node.signature));
                }
            }
        }

        orphans.sort();
        orphans
    }

    /// Audits total architectural health, circular references, and orphan exports.
    pub fn analyze_architectural_health(&self) -> ArchitecturalHealth {
        let start = Instant::now();

        let circular_dependencies = self.detect_import_cycles();
        let orphan_exports = self.find_orphan_exports();
        let total_files = self.file_to_symbols.len();
        let total_symbols = self.nodes.len();
        let total_edges = self.edges.len();
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        ArchitecturalHealth {
            circular_dependencies,
            orphan_exports,
            total_files,
            total_symbols,
            total_edges,
            latency_ms,
        }
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
    fn test_resolve_symbol_and_blast_radius() {
        let mut graph = SymbolGraph::new();
        let file_a = "pub struct Config { pub timeout: u64 }\npub fn load_config() -> Config { Config { timeout: 10 } }";
        let file_b = "use crate::Config;\npub fn run_server() {\n    let c = load_config();\n    let _ = c.timeout;\n}";

        graph.index_file_content("src/config.rs", file_a, Language::Rust);
        graph.index_file_content("src/server.rs", file_b, Language::Rust);
        graph.index_references();
        graph.resolve_edges();

        let resolved = graph.resolve_symbol("Config", Some("src/config.rs"));
        assert!(resolved.is_some());
        let res = resolved.unwrap();
        assert_eq!(res.file, "src/config.rs");
        assert!(res.is_exported);

        let report = graph.calculate_blast_radius("Config", Some("src/config.rs"), 2);
        assert_eq!(report.symbol, "Config");
        assert!(report.affected_files.contains(&"src/server.rs".to_string()));
        assert!(report.risk_score == RiskScore::Medium || report.risk_score == RiskScore::High);
    }

    #[test]
    fn test_detect_import_cycles() {
        let mut graph = SymbolGraph::new();
        graph.file_imports.insert("src/a.ts".to_string(), vec!["src/b.ts".to_string()]);
        graph.file_imports.insert("src/b.ts".to_string(), vec!["src/c.ts".to_string()]);
        graph.file_imports.insert("src/c.ts".to_string(), vec!["src/a.ts".to_string()]);

        let cycles = graph.detect_import_cycles();
        assert!(!cycles.is_empty(), "Should detect circular import between a, b, c");
        assert_eq!(cycles[0].len(), 4);
    }

    #[test]
    fn test_token_savings_calculation() {
        let full = "pub fn heavy_computation() -> u64 {\n    let mut x = 0;\n    for i in 0..1000 { x += i; }\n    x\n}";
        let skel = "pub fn heavy_computation() -> u64;";
        let savings = SymbolGraph::calculate_token_savings(full, skel);
        assert!(savings > 50.0, "Expected >50% savings, got {}%", savings);
    }
}
