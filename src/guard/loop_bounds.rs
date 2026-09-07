//! Canonical Loop Bounds & Invariant Prover.
//!
//! Implements AST pattern-matching for induction variables in iteration statements:
//! - Canonical loops: `for (let <id> = <init>; <id> < <target>.length; <id>++)`, `while (<id> < <target>.length)`
//! - Rust loops: `for <id> in 0..<target>.len()`, `while <id> < <target>.len()`
//! - Proves 0 <= init <= id < target.length in loop bodies to eliminate ARRAY_OOB false positives.

#![forbid(unsafe_code)]

use regex::Regex;
use std::sync::LazyLock;

/// Active induction variable invariant within a lexical loop scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopInvariant {
    pub target: String,
    pub induction_var: String,
    pub body_start: usize,
    pub body_end: usize,
}

// ---------------------------------------------------------------------------
// Compiled Pattern Regexes for Canonical Loops
// ---------------------------------------------------------------------------

/// Matches `for (let/var/const i = 0; i < arr.length; i++)` or `.byteLength` / `.size` / `.len()`.
static RE_FOR_CANONICAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\bfor\s*\(\s*(?:let|var|const)\s+([a-zA-Z0-9_$]+)\s*=\s*\d+\s*;\s*([a-zA-Z0-9_$]+)\s*<\s*([a-zA-Z0-9_$]+)\s*\.\s*(?:length|byteLength|size|len\s*\(\s*\))\s*;[^)]*\)").unwrap()
});

/// Matches `while (i < arr.length)` or `while (i < arr.len())`.
static RE_WHILE_CANONICAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\bwhile\s*\(\s*([a-zA-Z0-9_$]+)\s*<\s*([a-zA-Z0-9_$]+)\s*\.\s*(?:length|byteLength|size|len\s*\(\s*\))\s*\)").unwrap()
});

/// Matches Rust `for i in 0..arr.len()` or `for i in 0..arr.length`.
static RE_RUST_FOR_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bfor\s+([a-zA-Z0-9_]+)\s+in\s+\d+\s*\.\.\s*([a-zA-Z0-9_]+)\s*\.\s*(?:len\s*\(\s*\)|length)").unwrap()
});

/// Matches array indexing: `arr[i]`.
static RE_ARRAY_INDEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\[\s*([a-zA-Z_$][a-zA-Z0-9_$]*|\d+)\s*\]").unwrap()
});

/// Tracker for canonical loop induction variables and bounds.
#[derive(Debug, Default, Clone)]
pub struct LoopBoundTracker {
    invariants: Vec<LoopInvariant>,
}

impl LoopBoundTracker {
    /// Constructs tracker and extracts all loop invariants from `source`.
    pub fn analyze(source: &str) -> Self {
        let mut invariants = Vec::new();
        let bytes = source.as_bytes();
        let len = bytes.len();

        // 1. Scan for JS/TS for-loops
        for cap in RE_FOR_CANONICAL.captures_iter(source) {
            if let (Some(m_full), Some(id1_m), Some(id2_m), Some(target_m)) =
                (cap.get(0), cap.get(1), cap.get(2), cap.get(3))
            {
                if id1_m.as_str() == id2_m.as_str() {
                    let id = id1_m.as_str().to_string();
                    let target = target_m.as_str().to_string();
                    let loop_header_end = m_full.end();

                    let (body_start, body_end) = Self::find_body_span(source, loop_header_end, len);
                    invariants.push(LoopInvariant {
                        target,
                        induction_var: id,
                        body_start,
                        body_end,
                    });
                }
            }
        }

        // 2. Scan for while-loops
        for cap in RE_WHILE_CANONICAL.captures_iter(source) {
            if let (Some(m_full), Some(id_m), Some(target_m)) = (cap.get(0), cap.get(1), cap.get(2)) {
                let id = id_m.as_str().to_string();
                let target = target_m.as_str().to_string();
                let loop_header_end = m_full.end();

                let (body_start, body_end) = Self::find_body_span(source, loop_header_end, len);
                invariants.push(LoopInvariant {
                    target,
                    induction_var: id,
                    body_start,
                    body_end,
                });
            }
        }

        // 3. Scan for Rust for-in loops
        for cap in RE_RUST_FOR_RANGE.captures_iter(source) {
            if let (Some(m_full), Some(id_m), Some(target_m)) = (cap.get(0), cap.get(1), cap.get(2)) {
                let id = id_m.as_str().to_string();
                let target = target_m.as_str().to_string();
                let loop_header_end = m_full.end();

                let (body_start, body_end) = Self::find_body_span(source, loop_header_end, len);
                invariants.push(LoopInvariant {
                    target,
                    induction_var: id,
                    body_start,
                    body_end,
                });
            }
        }

        Self { invariants }
    }

    /// Finds the start and end byte offsets of a loop body starting after the loop header.
    fn find_body_span(source: &str, after_header: usize, len: usize) -> (usize, usize) {
        let bytes = source.as_bytes();
        let mut i = after_header;

        // Skip whitespace
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i >= len {
            return (after_header, len);
        }

        // If block body with '{'
        if bytes[i] == b'{' {
            let body_start = i;
            let mut depth = 0;
            while i < len {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return (body_start, i + 1);
                        }
                    }
                    b'"' | b'\'' | b'`' => {
                        let quote = bytes[i];
                        i += 1;
                        while i < len && bytes[i] != quote {
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
            return (body_start, len);
        }

        // Single statement body: scan until ';' or newline
        let body_start = i;
        while i < len && bytes[i] != b';' && bytes[i] != b'\n' {
            i += 1;
        }
        if i < len && bytes[i] == b';' {
            i += 1;
        }
        (body_start, i)
    }

    /// Returns true if `<target>[<index>]` at byte offset `offset` is provably within bounds.
    pub fn is_index_provably_within_bounds(&self, target: &str, index: &str, offset: usize) -> bool {
        if self.invariants.is_empty() {
            return false;
        }
        let idx = match self.invariants.binary_search_by_key(&offset, |inv| inv.body_start) {
            Ok(i) => i,
            Err(i) => {
                if i == 0 {
                    return false;
                }
                i - 1
            }
        };
        let inv = &self.invariants[idx];
        if offset >= inv.body_start && offset <= inv.body_end {
            return inv.target == target && inv.induction_var == index;
        }
        false
    }

    /// Full pass checking array indexing in `source`. Returns violation detail if any access is out of bounds.
    pub fn check_array_bounds(source: &str) -> Option<String> {
        let tracker = Self::analyze(source);

        for m in RE_ARRAY_INDEX.find_iter(source) {
            let before = &source[..m.start()];

            // Skip comments
            let last_nl = before.rfind('\n').unwrap_or(0);
            let line = &before[last_nl..];
            let line_prefix = line.trim_start();
            if line_prefix.starts_with("//") || line_prefix.starts_with('*') {
                continue;
            }

            // Skip test code
            if before.contains("#[cfg(test)]") || before.contains("#[test]") {
                continue;
            }

            // Skip string literals on this line
            let dquotes = line.matches('"').count() - line.matches(r#"\""#).count();
            let squotes = line.matches('\'').count() - line.matches(r#"\'"#).count();
            let bquotes = line.matches('`').count() - line.matches(r#"\`"#).count();
            if !dquotes.is_multiple_of(2) || !squotes.is_multiple_of(2) || !bquotes.is_multiple_of(2) {
                continue;
            }

            // Parse target and index
            let full_match = m.as_str();
            let open_bracket = match full_match.find('[') {
                Some(idx) => idx,
                None => continue,
            };
            let close_bracket = match full_match.find(']') {
                Some(idx) => idx,
                None => continue,
            };

            let target = full_match[..open_bracket].trim();
            let index = full_match[open_bracket + 1..close_bracket].trim();

            // Skip common safe patterns (numeric literals like arr[0] when guarded, or type definitions like T[K])
            if index.chars().all(|c| c.is_ascii_digit()) {
                // Constant numeric index 0 is allowed if guarded by len or empty check
                let local_start = before.rmatch_indices('\n').nth(3).map(|(i, _)| i).unwrap_or(0);
                let local_ctx = &source[local_start..m.end()];
                if local_ctx.contains(".length")
                    || local_ctx.contains(".len()")
                    || local_ctx.contains(".is_empty()")
                    || local_ctx.contains("assert!")
                {
                    continue;
                }
            }

            // Check if proved by canonical loop invariant
            if tracker.is_index_provably_within_bounds(target, index, m.start()) {
                continue;
            }

            // Check local 4-line context for explicit bounds check
            let local_start = before.rmatch_indices('\n').nth(3).map(|(i, _)| i).unwrap_or(0);
            let local_ctx = &source[local_start..m.end()];
            if local_ctx.contains(&format!("{} < {}.length", index, target))
                || local_ctx.contains(&format!("{} < {}.len()", index, target))
                || local_ctx.contains(&format!("{} < {}.byteLength", index, target))
                || local_ctx.contains(&format!("{}.get({})", target, index))
                || local_ctx.contains(&format!("{}.get(", target))
                || local_ctx.contains(&format!("assert!({} < {}.len()", index, target))
                || local_ctx.contains(&format!("assert({} < {}.length", index, target))
            {
                continue;
            }

            return Some(format!(
                "Array index access without bounds guard at byte {}: `{}`",
                m.start(),
                m.as_str()
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_for_loop_passes() {
        let code = "for (let i = 0; i < arr.length; i++) arr[i] = i * 2;";
        assert_eq!(LoopBoundTracker::check_array_bounds(code), None);

        let code_block = r#"
        for (let i = 0; i < arr.length; i++) {
            arr[i] = i * 2;
        }
        "#;
        assert_eq!(LoopBoundTracker::check_array_bounds(code_block), None);
    }

    #[test]
    fn test_typed_array_passes() {
        let code = r#"
        const buf = new Float32Array(1024);
        for (let i = 0; i < buf.length; i++) {
            buf[i] = i * 1.5;
        }
        "#;
        assert_eq!(LoopBoundTracker::check_array_bounds(code), None);
    }

    #[test]
    fn test_unbounded_access_outside_loop_fails() {
        let code = "arr[i] = 42;";
        let violation = LoopBoundTracker::check_array_bounds(code);
        assert!(violation.is_some());
    }

    #[test]
    fn test_wrong_index_inside_loop_fails() {
        let code = r#"
        for (let i = 0; i < arr.length; i++) {
            arr[j] = 42;
        }
        "#;
        let violation = LoopBoundTracker::check_array_bounds(code);
        assert!(violation.is_some());
    }

    #[test]
    fn test_wrong_target_inside_loop_fails() {
        let code = r#"
        for (let i = 0; i < arr.length; i++) {
            otherArr[i] = 42;
        }
        "#;
        let violation = LoopBoundTracker::check_array_bounds(code);
        assert!(violation.is_some());
    }
}
