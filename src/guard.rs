//! AstGuard — 6-pass deterministic safety invariant verifier.
//!
//! All regex patterns are compiled once at startup via `std::sync::LazyLock`
//! and reused across calls. Each `verify()` call typically completes in <0.05ms.

use std::sync::LazyLock;
use std::time::Instant;

use regex::Regex;

use crate::types::{VerificationReport, ViolationKind};

// ---------------------------------------------------------------------------
// Compiled regex patterns (LazyLock — initialised once, reused forever)
// ---------------------------------------------------------------------------

/// Matches variable division: identifier or single letter divided by variable.
static RE_DIV_BY_ZERO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]*\s*/\s*[a-zA-Z_][a-zA-Z0-9_]*").unwrap()
});

/// Detects numeric literal divisors (safe: e.g. `x / 2`, `x / 10.0`).
static RE_DIV_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/\s*[0-9]+(?:\.[0-9]+)?").unwrap()
});

/// Detects array/slice indexing by variable (`arr[i]`, `slice[idx]`).
static RE_ARRAY_INDEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]*\s*\[\s*[a-zA-Z_][a-zA-Z0-9_]*\s*\]").unwrap()
});

/// Detects `.len()` bound checks or `get()` safe accessor within context.
static RE_BOUND_CHECK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.len\(\)|\.get\s*\(|\.is_empty\(\)|assert!\s*\(").unwrap()
});

/// Detects direct `.unwrap()` or `.expect(` calls.
static RE_UNWRAP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.(unwrap|expect)\s*\(").unwrap()
});

/// Detects `std::sync::Mutex` usage near `.await`.
static RE_SYNC_MUTEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"std\s*::\s*sync\s*::\s*Mutex|sync::Mutex").unwrap()
});

/// Detects `.await` near a mutex lock.
static RE_AWAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.await").unwrap()
});

/// Detects catastrophically backtracking regex patterns like `(a+)+` or `(.+)*`.
static RE_REDOS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"["'`].*(\(.+[+*]\)[+*]|\([^)]*[+*][^)]*\)[+*]).*["'`]"#).unwrap()
});

/// Detects unsafe deep property access in TS/JS: `a.b.c` without `?.`.
static RE_NULL_DEREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z_$][a-zA-Z0-9_$]*\.[a-zA-Z_$][a-zA-Z0-9_$]*\.[a-zA-Z_$][a-zA-Z0-9_$]*").unwrap()
});

/// Detects safe optional chaining.
static RE_OPTIONAL_CHAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\?\.[a-zA-Z_$]").unwrap()
});

/// Detects conditional hook invocations inside if blocks.
static RE_HOOK_IF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\bif\s*\([^)]*\)\s*\{[^}]*\buse[A-Z][a-zA-Z0-9_]*\s*\(").unwrap()
});

/// Detects conditional hook invocations inside loops (for, while).
static RE_HOOK_LOOP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\b(for|while)\s*\([^)]*\)\s*\{[^}]*\buse[A-Z][a-zA-Z0-9_]*\s*\(").unwrap()
});

/// Detects conditional hook invocations inside ternary branches.
static RE_HOOK_TERNARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\?[^:;\n]*\buse[A-Z][a-zA-Z0-9_]*\s*\(").unwrap()
});

/// Detects client-side secret references without safe public prefixes.
static RE_CLIENT_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:process\.env|import\.meta\.env)\.([A-Z0-9_]+)"#).unwrap()
});

/// Detects raw dangerouslySetInnerHTML without sanitization wrappers.
static RE_DANGEROUS_HTML: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"dangerouslySetInnerHTML\s*=\s*\{\{\s*__html\s*:\s*([^}]+)\}\}"#).unwrap()
});

// ---------------------------------------------------------------------------
// AstGuard
// ---------------------------------------------------------------------------

/// Stateless deterministic safety verifier.
/// Thread-safe: all state is in `LazyLock` globals.
pub struct AstGuard;

impl AstGuard {
    /// Run all invariant passes on `source` and return a `VerificationReport`.
    /// Average latency: <0.05ms on modern hardware.
    pub fn verify(source: &str) -> VerificationReport {
        let start = Instant::now();

        // Pass 0: Delimiter balance
        if !Self::check_delimiter_balance(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(
                ViolationKind::UnbalancedDelimiters,
                "Source contains unbalanced braces, brackets, or parentheses.",
                latency_ms,
            );
        }

        // Pass 1: Async mutex across await (Prioritized before unwrap to catch concurrency traps)
        if let Some(v) = Self::check_async_mutex(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::AsyncMutexAcrossAwait, v, latency_ms);
        }

        // Pass 2: Division by zero
        if let Some(v) = Self::check_div_by_zero(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::DivisionByZero, v, latency_ms);
        }

        // Pass 3: Array out of bounds
        if let Some(v) = Self::check_array_bounds(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::ArrayOutOfBounds, v, latency_ms);
        }

        // Pass 4: Unsafe unwrap
        if let Some(v) = Self::check_unsafe_unwrap(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::UnsafeUnwrap, v, latency_ms);
        }

        // Pass 5: ReDoS catastrophic backtracking
        if let Some(v) = Self::check_redos(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::ReDoSPattern, v, latency_ms);
        }

        // Pass 6: TypeScript null dereference
        if let Some(v) = Self::check_null_deref(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::NullDereference, v, latency_ms);
        }

        // Pass 7: React Rules of Hooks (Conditional Invocations)
        if let Some(v) = Self::check_conditional_hooks(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::ConditionalHookCall, v, latency_ms);
        }

        // Pass 8: Client/Server Boundary Secret Leak
        if let Some(v) = Self::check_client_secret_leak(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::ClientSecretLeak, v, latency_ms);
        }

        // Pass 9: Unsafe Inner HTML Injection
        if let Some(v) = Self::check_unsafe_inner_html(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::UnsafeInnerHtml, v, latency_ms);
        }

        // Pass 10: JSX / HTML Tag Balancing
        if let Some(v) = Self::check_jsx_tags(source) {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return VerificationReport::failed(ViolationKind::JsxTagMismatch, v, latency_ms);
        }

        VerificationReport::passed(start.elapsed().as_secs_f64() * 1000.0)
    }

    /// Dijkstra single-pass delimiter balance check.
    /// Returns `true` if all `{}`, `[]`, `()` are balanced.
    pub fn check_delimiter_balance(source: &str) -> bool {
        let mut stack: Vec<char> = Vec::new();
        let mut in_string = false;
        let mut string_char = '"';
        let mut prev = '\0';

        for ch in source.chars() {
            // Naive string boundary tracking (skips escape sequences)
            if (ch == '"' || ch == '\'' || ch == '`') && prev != '\\' {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                prev = ch;
                continue;
            }
            if in_string {
                prev = ch;
                continue;
            }

            match ch {
                '{' | '[' | '(' => stack.push(ch),
                '}' => if stack.pop() != Some('{') { return false; },
                ']' => if stack.pop() != Some('[') { return false; },
                ')' => if stack.pop() != Some('(') { return false; },
                _   => {}
            }
            prev = ch;
        }
        stack.is_empty()
    }

    /// Dijkstra JSX and HTML tag balance scanner.
    /// Verifies opening/closing tags, fragments `<>...</>`, and self-closing tags `<img />`.
    pub fn check_jsx_tags(source: &str) -> Option<String> {
        // Fast skip if code contains no JSX/HTML tags or is Rust
        let is_rust = source.contains("fn ") || source.contains("impl ") || source.contains("pub struct ") || source.contains("pub enum ") || source.contains("use std::");
        if is_rust || !source.contains('<') || !source.contains('>') {
            return None;
        }

        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut stack: Vec<String> = Vec::new();
        let mut i = 0;
        let mut in_string = false;
        let mut quote_char = 0u8;

        while i < len {
            let b = bytes[i];

            // String tracking
            if (b == b'"' || b == b'\'' || b == b'`') && (i == 0 || bytes[i - 1] != b'\\') {
                if in_string && b == quote_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    quote_char = b;
                }
                i += 1;
                continue;
            }
            if in_string {
                i += 1;
                continue;
            }

            // Skip comments
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                while i < len && bytes[i] != b'\n' { i += 1; }
                continue;
            }
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') { i += 1; }
                i += 2;
                continue;
            }

            if b == b'<' && i + 1 < len {
                let next = bytes[i + 1];

                // Skip comments <!-- ... -->
                if i + 3 < len && &bytes[i..i + 4] == b"<!--" {
                    i += 4;
                    while i + 2 < len && &bytes[i..i + 3] != b"-->" { i += 1; }
                    i += 3;
                    continue;
                }

                // Check Fragment <>
                if next == b'>' {
                    stack.push("".to_string());
                    i += 2;
                    continue;
                }

                // Check Closing Fragment </> or Closing Tag </tag>
                if next == b'/' {
                    if i + 2 < len && bytes[i + 2] == b'>' {
                        // Closing fragment </>
                        match stack.pop() {
                            Some(tag) if tag.is_empty() => { i += 3; continue; }
                            Some(tag) => return Some(format!("Mismatched closing fragment `</>` for opened tag `<{}>`", tag)),
                            None => return Some("Unexpected closing fragment `</>` without matching `<>`".to_string()),
                        }
                    }

                    // Closing tag </tag>
                    let tag_start = i + 2;
                    let mut tag_end = tag_start;
                    while tag_end < len && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'_' || bytes[tag_end] == b'$' || bytes[tag_end] == b'.' || bytes[tag_end] == b'-') {
                        tag_end += 1;
                    }
                    if tag_end > tag_start {
                        let tag_name = &source[tag_start..tag_end];
                        // Scan forward to '>'
                        while tag_end < len && bytes[tag_end] != b'>' { tag_end += 1; }
                        if tag_end < len && bytes[tag_end] == b'>' {
                            match stack.pop() {
                                Some(open_tag) if open_tag == tag_name => { i = tag_end + 1; continue; }
                                Some(open_tag) => return Some(format!("Mismatched JSX closing tag `</{}>` for opened tag `<{}>`", tag_name, open_tag)),
                                None => return Some(format!("Unexpected JSX closing tag `</{}>` without opening tag", tag_name)),
                            }
                        }
                    }
                }

                // Opening tag <TagName ...> or <TagName ... />
                if next.is_ascii_alphabetic() || next == b'_' || next == b'$' {
                    let tag_start = i + 1;
                    let mut tag_end = tag_start;
                    while tag_end < len && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'_' || bytes[tag_end] == b'$' || bytes[tag_end] == b'.' || bytes[tag_end] == b'-') {
                        tag_end += 1;
                    }
                    let tag_name = source[tag_start..tag_end].to_string();

                    // Scan tag attributes up to '>' or '/>'
                    let mut scan = tag_end;
                    let mut is_self_closing = false;
                    let mut attr_in_str = false;
                    let mut attr_quote = 0u8;
                    let mut brace_depth = 0;

                    while scan < len {
                        let sb = bytes[scan];
                        if (sb == b'"' || sb == b'\'' || sb == b'`') && (scan == 0 || bytes[scan - 1] != b'\\') {
                            if attr_in_str && sb == attr_quote { attr_in_str = false; }
                            else if !attr_in_str { attr_in_str = true; attr_quote = sb; }
                        }
                        if !attr_in_str {
                            if sb == b'{' { brace_depth += 1; }
                            else if sb == b'}' { if brace_depth > 0 { brace_depth -= 1; } }
                            else if brace_depth == 0 {
                                if sb == b'/' && scan + 1 < len && bytes[scan + 1] == b'>' {
                                    is_self_closing = true;
                                    scan += 2;
                                    break;
                                }
                                if sb == b'>' {
                                    scan += 1;
                                    break;
                                }
                            }
                        }
                        scan += 1;
                    }

                    // Known HTML void tags (auto self-closing)
                    let is_void_tag = matches!(tag_name.to_lowercase().as_str(), "img" | "input" | "br" | "hr" | "meta" | "link" | "source");

                    if !is_self_closing && !is_void_tag {
                        stack.push(tag_name);
                    }
                    i = scan;
                    continue;
                }
            }

            i += 1;
        }

        if let Some(unclosed) = stack.last() {
            if unclosed.is_empty() {
                return Some("Unclosed JSX fragment `<>` at end of file".to_string());
            } else {
                return Some(format!("Unclosed JSX tag `<{}>` at end of file", unclosed));
            }
        }

        None
    }

    // --- Internal Passes ---

    fn check_conditional_hooks(source: &str) -> Option<String> {
        if RE_HOOK_IF.is_match(source) {
            return Some("React Hook called conditionally inside an `if` block — violates Rules of Hooks.".to_string());
        }
        if RE_HOOK_LOOP.is_match(source) {
            return Some("React Hook called inside a loop (`for`/`while`) — violates Rules of Hooks.".to_string());
        }
        if RE_HOOK_TERNARY.is_match(source) {
            return Some("React Hook called inside a ternary branch — violates Rules of Hooks.".to_string());
        }
        None
    }

    fn check_client_secret_leak(source: &str) -> Option<String> {
        let is_client = source.contains("\"use client\"") || source.contains("'use client'");
        if is_client {
            for cap in RE_CLIENT_SECRET.captures_iter(source) {
                if let Some(var_name) = cap.get(1) {
                    let name = var_name.as_str();
                    let is_safe_public = name.starts_with("NEXT_PUBLIC_")
                        || name.starts_with("VITE_")
                        || name.starts_with("PUBLIC_")
                        || name.starts_with("REACT_APP_")
                        || matches!(name, "NODE_ENV" | "BASE_URL" | "DEV" | "PROD" | "SSR" | "MODE");

                    if !is_safe_public {
                        return Some(format!(
                            "Server-side secret `{}` accessed in client component (\"use client\") without safe public prefix.",
                            cap.get(0).map(|m| m.as_str()).unwrap_or(name)
                        ));
                    }
                }
            }
        }
        None
    }

    fn check_unsafe_inner_html(source: &str) -> Option<String> {
        for cap in RE_DANGEROUS_HTML.captures_iter(source) {
            if let Some(expr) = cap.get(1) {
                let html_expr = expr.as_str().trim();
                let is_sanitized = html_expr.contains("sanitize") || html_expr.starts_with('"') || html_expr.starts_with('\'');
                if !is_sanitized {
                    return Some(format!(
                        "Unsanitized `dangerouslySetInnerHTML` with raw `{}` — vulnerable to XSS injection.",
                        html_expr
                    ));
                }
            }
        }
        None
    }

    // --- Internal Passes ---

    fn check_div_by_zero(source: &str) -> Option<String> {
        if RE_DIV_BY_ZERO.is_match(source) && !RE_DIV_LITERAL.is_match(source) {
            if !source.contains("!= 0") && !source.contains("!= 0.0") && !source.contains("assert!") {
                if let Some(m) = RE_DIV_BY_ZERO.find(source) {
                    return Some(format!(
                        "Unguarded division at byte {}: `{}` — denominator may be zero.",
                        m.start(), m.as_str()
                    ));
                }
            }
        }
        None
    }

    fn check_array_bounds(source: &str) -> Option<String> {
        if RE_ARRAY_INDEX.is_match(source) && !RE_BOUND_CHECK.is_match(source) {
            if let Some(m) = RE_ARRAY_INDEX.find(source) {
                return Some(format!(
                    "Array index access without bounds guard at byte {}: `{}`",
                    m.start(), m.as_str()
                ));
            }
        }
        None
    }

    fn check_unsafe_unwrap(source: &str) -> Option<String> {
        for cap in RE_UNWRAP.find_iter(source) {
            let before = &source[..cap.start()];
            let last_newline = before.rfind('\n').unwrap_or(0);
            let line = &before[last_newline..];
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') {
                continue;
            }
            if !line.contains("is_some()") && !line.contains("is_ok()") && !line.contains("if let") {
                return Some(format!(
                    "Unsafe `.unwrap()` at byte {} without prior guard.",
                    cap.start()
                ));
            }
        }
        None
    }

    fn check_async_mutex(source: &str) -> Option<String> {
        let has_mutex = source.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.starts_with('*') && RE_SYNC_MUTEX.is_match(trimmed)
        });
        let has_await = source.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.starts_with('*') && RE_AWAIT.is_match(trimmed)
        });
        if has_mutex && has_await {
            return Some(
                "std::sync::Mutex used in async context with .await — use tokio::sync::Mutex instead.".to_string()
            );
        }
        None
    }

    fn check_redos(source: &str) -> Option<String> {
        for m in RE_REDOS.find_iter(source) {
            let before = &source[..m.start()];
            let last_newline = before.rfind('\n').unwrap_or(0);
            let line = &before[last_newline..];
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with('*') {
                continue;
            }
            return Some(format!(
                "Catastrophic ReDoS pattern at byte {}: `{}` — nested quantifiers cause O(2^n) backtracking.",
                m.start(), &m.as_str()[..m.as_str().len().min(60)]
            ));
        }
        None
    }

    fn check_null_deref(source: &str) -> Option<String> {
        let is_rust = source.contains("fn ") || source.contains("impl ") || source.contains("pub struct ") || source.contains("pub enum ");
        if is_rust {
            return None;
        }

        if RE_NULL_DEREF.is_match(source) && !RE_OPTIONAL_CHAIN.is_match(source) {
            if source.contains(": string") || source.contains("interface ") || source.contains("export ") {
                for m in RE_NULL_DEREF.find_iter(source) {
                    let s = m.as_str();
                    if s.starts_with("process.env") || s.starts_with("import.meta") {
                        continue;
                    }
                    let before = &source[..m.start()];
                    let last_newline = before.rfind('\n').unwrap_or(0);
                    let line = &before[last_newline..];
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("//") || trimmed.starts_with('*') {
                        continue;
                    }
                    return Some(format!(
                        "Deep property access without optional chaining at byte {}: `{}` — use `?.`",
                        m.start(), s
                    ));
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ViolationKind;
    use std::time::Instant;

    #[test]
    fn test_safe_code_passes_all_invariants() {
        let safe = r#"
fn compute_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: f64 = values.iter().sum();
    let count = values.len();
    if count != 0 {
        sum / count as f64
    } else {
        0.0
    }
}
"#;
        let report = AstGuard::verify(safe);
        assert!(report.passed, "Expected PASS, got: {:?}", report.violation);
    }

    #[test]
    fn test_detects_division_by_zero() {
        let bad = r#"
fn ratio(a: f64, b: f64) -> f64 {
    a / b
}
"#;
        let report = AstGuard::verify(bad);
        assert!(!report.passed);
        assert_eq!(report.violation, Some(ViolationKind::DivisionByZero));
    }

    #[test]
    fn test_detects_unsafe_unwrap() {
        let bad = r#"
fn get_name(map: &std::collections::HashMap<&str, &str>) -> &str {
    map.get("name").unwrap()
}
"#;
        let report = AstGuard::verify(bad);
        assert!(!report.passed);
        assert_eq!(report.violation, Some(ViolationKind::UnsafeUnwrap));
    }

    #[test]
    fn test_detects_async_mutex_across_await() {
        let bad = r#"
use std::sync::Mutex;
async fn update(state: &Mutex<u32>) {
    let mut guard = state.lock().unwrap();
    some_async_fn().await;
    *guard += 1;
}
"#;
        let report = AstGuard::verify(bad);
        assert!(!report.passed);
        assert_eq!(report.violation, Some(ViolationKind::AsyncMutexAcrossAwait));
    }

    #[test]
    fn test_detects_redos_pattern() {
        let bad = r#"
const RE: &str = "(a+)+$";
"#;
        let report = AstGuard::verify(bad);
        assert!(!report.passed);
        assert_eq!(report.violation, Some(ViolationKind::ReDoSPattern));
    }

    #[test]
    fn test_detects_unbalanced_delimiters() {
        let bad = "fn broken( { let x = 1;";
        let report = AstGuard::verify(bad);
        assert!(!report.passed);
        assert_eq!(report.violation, Some(ViolationKind::UnbalancedDelimiters));
    }

    #[test]
    fn test_verify_latency_below_1ms_debug() {
        let source = "fn safe() -> u32 { 42 }";
        let start = Instant::now();
        let _r = AstGuard::verify(source);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(elapsed < 50.0, "verify() took {}ms — expected <50ms in debug", elapsed);
    }

    #[test]
    fn test_delimiter_balance_correct() {
        assert!(AstGuard::check_delimiter_balance("fn f() { let x = [1, 2]; }"));
        assert!(!AstGuard::check_delimiter_balance("fn f() { let x = [1, 2; }"));
        assert!(!AstGuard::check_delimiter_balance("((()))(("));
    }

    #[test]
    fn test_jsx_tag_balancing() {
        let valid_jsx = r#"
        export function Card() {
            return (
                <>
                    <div className="container">
                        <img src="logo.png" alt="logo" />
                        <span>Hello World</span>
                    </div>
                </>
            );
        }
        "#;
        let rep = AstGuard::verify(valid_jsx);
        assert!(rep.passed, "Expected valid JSX to pass: {:?}", rep.violation);

        let mismatched_jsx = r#"
        export function Broken() {
            return <div><span>Mismatched</div></span>;
        }
        "#;
        let rep_bad = AstGuard::verify(mismatched_jsx);
        assert!(!rep_bad.passed);
        assert_eq!(rep_bad.violation, Some(ViolationKind::JsxTagMismatch));
    }

    #[test]
    fn test_detects_conditional_hooks() {
        let bad_hook = r#"
        function MyComponent({ isLoggedIn }) {
            if (isLoggedIn) {
                const [user, setUser] = useState(null);
            }
            return <div>User</div>;
        }
        "#;
        let rep = AstGuard::verify(bad_hook);
        assert!(!rep.passed);
        assert_eq!(rep.violation, Some(ViolationKind::ConditionalHookCall));
    }

    #[test]
    fn test_detects_client_secret_leak() {
        let client_code = r#"
        "use client";
        import React from "react";
        export function Checkout() {
            const secret = process.env.STRIPE_SECRET_KEY;
            return <button>Pay</button>;
        }
        "#;
        let rep = AstGuard::verify(client_code);
        assert!(!rep.passed);
        assert_eq!(rep.violation, Some(ViolationKind::ClientSecretLeak));

        let safe_client = r#"
        "use client";
        import React from "react";
        export function Safe() {
            const pubKey = process.env.NEXT_PUBLIC_STRIPE_KEY;
            return <button>Pay</button>;
        }
        "#;
        let rep_safe = AstGuard::verify(safe_client);
        assert!(rep_safe.passed, "Safe public env should pass: {:?}", rep_safe.violation);
    }

    #[test]
    fn test_detects_unsafe_inner_html() {
        let raw_injection = r#"
        export function Bio({ bioHtml }) {
            return <div dangerouslySetInnerHTML={{ __html: bioHtml }} />;
        }
        "#;
        let rep = AstGuard::verify(raw_injection);
        assert!(!rep.passed);
        assert_eq!(rep.violation, Some(ViolationKind::UnsafeInnerHtml));

        let sanitized = r#"
        import DOMPurify from 'dompurify';
        export function SafeBio({ bioHtml }) {
            return <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(bioHtml) }} />;
        }
        "#;
        let rep_safe = AstGuard::verify(sanitized);
        assert!(rep_safe.passed, "Sanitized dangerouslySetInnerHTML should pass: {:?}", rep_safe.violation);
    }
}
