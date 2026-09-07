//! Flow-Sensitive Path Narrowing & Lexical Fact Environment.
//!
//! Implements a lexical scope fact table using a cactus stack / push-pop fact environment:
//! - Recognizes truthiness guards: `if (<id>) { ... }`, `if (<id> !== null && <id> !== undefined) { ... }`
//! - Handles early-exit guards: `if (!<id>) return;`
//! - Handles logical short-circuit guards: `id && id.prop`, `id && id.run()`
//! - Handles optional chaining: `id?.prop`
//! - Suppresses NullDereference violations when target identifier has an active NonNull fact in scope.

#![forbid(unsafe_code)]

use regex::Regex;
use std::sync::LazyLock;

/// A verified fact known to hold within a specific byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeFact {
    pub ident: String,
    pub span_start: usize,
    pub span_end: usize,
}

// ---------------------------------------------------------------------------
// Compiled Pattern Regexes for Guard Recognition
// ---------------------------------------------------------------------------

/// Matches `if (ident) {` or `if (ident)` without block.
static RE_IF_TRUTHINESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\bif\s*\(\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\)").unwrap()
});

/// Matches `if (ident !== null && ident !== undefined)` or `ident != null` or `ident != undefined`.
static RE_IF_NON_NULL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\bif\s*\(\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\s*(?:!==|!=)\s*(?:null|undefined)[^)]*\)").unwrap()
});

/// Matches early exit: `if (!ident) return;` or `if (ident == null) return;`.
static RE_IF_EARLY_RETURN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bif\s*\(\s*!(?:!|(?P<id>[a-zA-Z_$][a-zA-Z0-9_$]*))\s*\)\s*(?:return\b|throw\b)").unwrap()
});

/// Matches `const x = getNullable();` or functions known to return nullable values.
static RE_NULLABLE_DECL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*(?::\s*[^=]+)?=\s*(?:[a-zA-Z0-9_$]*Nullable\w*\s*\(|querySelector\w*\s*\(|\bfind\s*\(|null\b|undefined\b)").unwrap()
});

/// Matches method or property call on identifier: `ident.method(...)` or `ident.field`.
static RE_PROP_OR_METHOD_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\.\s*([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap()
});

/// Matches deep property access without optional chaining: `a.b.c`.
static RE_DEEP_NULL_DEREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z_$][a-zA-Z0-9_$]*\.[a-zA-Z_$][a-zA-Z0-9_$]*\.[a-zA-Z_$][a-zA-Z0-9_$]*").unwrap()
});

/// Flow-sensitive fact environment tracker.
#[derive(Debug, Default, Clone)]
pub struct FactStack {
    facts: Vec<ScopeFact>,
    nullable_vars: Vec<String>,
}

impl FactStack {
    /// Ingests source code and extracts lexical scopes with narrowed facts.
    pub fn analyze(source: &str) -> Self {
        let mut facts = Vec::new();
        let mut nullable_vars = Vec::new();
        let bytes = source.as_bytes();
        let len = bytes.len();

        // 1. Detect nullable variable declarations: `const x = getNullable();`
        for cap in RE_NULLABLE_DECL.captures_iter(source) {
            if let Some(m) = cap.get(1) {
                nullable_vars.push(m.as_str().to_string());
            }
        }

        // 2. Scan for truthiness guards: `if (x) { ... }`
        for cap in RE_IF_TRUTHINESS.captures_iter(source) {
            if let (Some(m_full), Some(id_m)) = (cap.get(0), cap.get(1)) {
                let ident = id_m.as_str().to_string();
                let (span_start, span_end) = Self::find_consequent_span(source, m_full.end(), len);
                facts.push(ScopeFact {
                    ident,
                    span_start,
                    span_end,
                });
            }
        }

        // 3. Scan for non-null guards: `if (x !== null) { ... }`
        for cap in RE_IF_NON_NULL.captures_iter(source) {
            if let (Some(m_full), Some(id_m)) = (cap.get(0), cap.get(1)) {
                let ident = id_m.as_str().to_string();
                let (span_start, span_end) = Self::find_consequent_span(source, m_full.end(), len);
                facts.push(ScopeFact {
                    ident,
                    span_start,
                    span_end,
                });
            }
        }

        // 4. Scan for early return guards: `if (!x) return;`
        for cap in RE_IF_EARLY_RETURN.captures_iter(source) {
            if let (Some(m_full), Some(id_m)) = (cap.get(0), cap.name("id")) {
                let ident = id_m.as_str().to_string();
                let span_start = m_full.end();
                // Extends to the end of the enclosing block or function
                let span_end = Self::find_enclosing_block_end(source, span_start, len);
                facts.push(ScopeFact {
                    ident,
                    span_start,
                    span_end,
                });
            }
        }

        Self { facts, nullable_vars }
    }

    /// Finds the consequent span (either `{ ... }` or single statement) starting after `after_if`.
    fn find_consequent_span(source: &str, after_if: usize, len: usize) -> (usize, usize) {
        let bytes = source.as_bytes();
        let mut i = after_if;
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i >= len {
            return (after_if, len);
        }

        if bytes[i] == b'{' {
            let start = i;
            let mut depth = 0;
            while i < len {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return (start, i + 1);
                        }
                    }
                    b'"' | b'\'' | b'`' => {
                        let q = bytes[i];
                        i += 1;
                        while i < len && bytes[i] != q {
                            if bytes[i] == b'\\' {
                                i += 1;
                            }
                            i += 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            return (start, len);
        }

        // Single statement: until ';' or '\n'
        let start = i;
        while i < len && bytes[i] != b';' && bytes[i] != b'\n' {
            i += 1;
        }
        if i < len && bytes[i] == b';' {
            i += 1;
        }
        (start, i)
    }

    /// Finds the end of the enclosing block `{ ... }`.
    fn find_enclosing_block_end(source: &str, from: usize, len: usize) -> usize {
        let bytes = source.as_bytes();
        let mut i = from;
        let mut depth = 0;
        while i < len {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    if depth == 0 {
                        return i;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            i += 1;
        }
        len
    }

    /// Checks whether `ident` has an active NonNull fact at `byte_offset`.
    pub fn is_provably_non_null(&self, ident: &str, byte_offset: usize) -> bool {
        for fact in &self.facts {
            if fact.ident == ident && byte_offset >= fact.span_start && byte_offset <= fact.span_end {
                return true;
            }
        }
        false
    }

    /// Full flow-sensitive check for null dereference violations.
    pub fn check_null_deref(source: &str) -> Option<String> {
        let is_rust = source.contains("fn ")
            || source.contains("impl ")
            || source.contains("pub struct ")
            || source.contains("pub enum ");
        if is_rust {
            return None;
        }

        let is_ts_or_js = source.contains(": string")
            || source.contains("interface ")
            || source.contains("export ")
            || source.contains("const ")
            || source.contains("let ")
            || source.contains("function ");
        if !is_ts_or_js {
            return None;
        }

        let fact_stack = Self::analyze(source);

        // 1. Check known nullable variables: `const x = getNullable(); x.run();`
        for var in &fact_stack.nullable_vars {
            for m in RE_PROP_OR_METHOD_CALL.find_iter(source) {
                let call_str = m.as_str();
                let prefix = match call_str.split('.').next() {
                    Some(p) => p.trim(),
                    None => continue,
                };
                if prefix == var {
                    // Check if inside optional chaining
                    let line_start = source[..m.start()].rfind('\n').unwrap_or(0);
                    let line_end = source[m.end()..].find('\n').map_or(source.len(), |p| m.end() + p);
                    let full_line = &source[line_start..line_end];
                    if full_line.contains(&format!("{}?.", var)) || full_line.contains("?.") {
                        continue;
                    }

                    // Check if protected by logical AND: `var && var.run()`
                    if full_line.contains(&format!("{} &&", var)) {
                        continue;
                    }

                    // Check if variable is proven non-null by active fact stack
                    if fact_stack.is_provably_non_null(var, m.start()) {
                        continue;
                    }

                    // Skip variable declaration itself
                    if m.start() < source.find(var).unwrap_or(0) + var.len() + 15 {
                        let decl_ctx = &source[..m.start()];
                        if decl_ctx.trim_end().ends_with('=') || decl_ctx.trim_end().ends_with(':') {
                            continue;
                        }
                    }

                    return Some(format!(
                        "Nullable variable `{}` dereferenced without guard at byte {}: `{}`",
                        var,
                        m.start(),
                        m.as_str()
                    ));
                }
            }
        }

        // 2. Check deep property access without optional chaining: `a.b.c`
        for m in RE_DEEP_NULL_DEREF.find_iter(source) {
            let s = m.as_str();
            if s.starts_with("process.env") || s.starts_with("import.meta") {
                continue;
            }

            // Skip comments
            let before = &source[..m.start()];
            let last_newline = before.rfind('\n').unwrap_or(0);
            let line_prefix = before[last_newline..].trim_start();
            if line_prefix.starts_with("//") || line_prefix.starts_with('*') {
                continue;
            }

            // Optional chaining check on the line
            let line_end = source[m.end()..].find('\n').map_or(source.len(), |p| m.end() + p);
            let full_line = &source[last_newline..line_end];
            if full_line.contains("?.") {
                continue;
            }

            // Extract root identifier
            let root_ident = s.split('.').next().unwrap_or("").trim();
            if !root_ident.is_empty() && fact_stack.is_provably_non_null(root_ident, m.start()) {
                continue;
            }

            // Logical AND guard check: `root && root.b.c`
            if !root_ident.is_empty() && full_line.contains(&format!("{} &&", root_ident)) {
                continue;
            }

            return Some(format!(
                "Deep property access without optional chaining at byte {}: `{}` — use `?.`",
                m.start(),
                s
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guarded_nullable_passes() {
        let code = r#"
        const x = getNullable();
        if (x) {
            x.run();
        }
        "#;
        assert_eq!(FactStack::check_null_deref(code), None);
    }

    #[test]
    fn test_unguarded_nullable_fails() {
        let code = r#"
        const x = getNullable();
        x.run();
        "#;
        let violation = FactStack::check_null_deref(code);
        assert!(violation.is_some());
    }

    #[test]
    fn test_non_null_comparison_passes() {
        let code = r#"
        const x = getNullable();
        if (x !== null && x !== undefined) {
            x.run();
        }
        "#;
        assert_eq!(FactStack::check_null_deref(code), None);
    }

    #[test]
    fn test_deep_access_with_optional_chaining_passes() {
        let code = r#"
        export function getUser(res: any) {
            return res?.data?.user?.profile;
        }
        "#;
        assert_eq!(FactStack::check_null_deref(code), None);
    }

    #[test]
    fn test_deep_access_unguarded_fails() {
        let code = r#"
        export function getUser(res: any) {
            return res.data.user.profile;
        }
        "#;
        let violation = FactStack::check_null_deref(code);
        assert!(violation.is_some());
    }
}
