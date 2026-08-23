//! Extended Deterministic Safety Invariants (Rules 12 to 20+).
//!
//! Enforces sub-millisecond AST invariant verification for enterprise security,
//! concurrency soundness, frontend leak prevention, and memory safety.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use regex::Regex;
use crate::types::ViolationKind;

// ---------------------------------------------------------------------------
// Compiled Regex Patterns for Rules 12 - 20
// ---------------------------------------------------------------------------

/// Rule 12: SQL Injection - Unparameterized string interpolation / concatenation in SQL
static RE_SQL_RAW_CONCAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(SELECT|INSERT\s+INTO|UPDATE|DELETE\s+FROM|WHERE)\s+.*(\$\{[^}]+\}|"\s*\+\s*[a-zA-Z_$]|\bformat!\s*\(\s*"[^"]*\{\}.*"\s*,)"#).unwrap()
});

/// Rule 13: Floating Promise - Async calls without await / then / catch / void / return
static RE_FLOATING_PROMISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*(?:fetch|axios\.(?:get|post|put|delete)|db\.(?:query|execute)|[a-zA-Z0-9_$]+Async)\s*\([^;\n]*\)\s*;"#).unwrap()
});

/// Rule 14: React State Race - Direct non-functional setState inside async loops or after await
static RE_REACT_STATE_RACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)\b(?:for|while)\s*\([^)]*\)\s*\{[^}]*(?:await\s+[^;]+;[^}]*)?\bset[A-Z][a-zA-Z0-9_$]*\s*\(\s*[a-zA-Z0-9_$]+\s*[+\-*]\s*\d+\s*\)"#).unwrap()
});

/// Rule 15: Event Listener Leak - addEventListener in useEffect without removeEventListener cleanup
static RE_ADD_EVENT_LISTENER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"addEventListener\s*\(\s*["']([a-zA-Z0-9_-]+)["']"#).unwrap()
});
static RE_REMOVE_EVENT_LISTENER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"removeEventListener\s*\(\s*["']([a-zA-Z0-9_-]+)["']"#).unwrap()
});
static RE_USE_EFFECT_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)useEffect\s*\(\s*\(\s*\)\s*=>\s*\{(?P<body>.*?)\}\s*,\s*\["#).unwrap()
});

/// Rule 16: Insecure Randomness - Math.random used in security / token / auth / session contexts
static RE_INSECURE_RANDOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(token|secret|key|password|auth|session|nonce|salt|pin|apiKey|crypto)\b[^;\n]*=\s*[^;\n]*Math\.random\s*\(\s*\)"#).unwrap()
});
static RE_RANDOM_IN_SECURITY_VAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)let\s+(?:mut\s+)?(?:[a-zA-Z0-9_]*[Tt]oken|[a-zA-Z0-9_]*[Ss]ecret|[a-zA-Z0-9_]*[Kk]ey|[a-zA-Z0-9_]*[Aa]uth|[a-zA-Z0-9_]*[Ss]ession|[a-zA-Z0-9_]*[Nn]once)\s*=[^;\n]*Math\.random"#).unwrap()
});

/// Rule 17: Path Traversal - Direct user inputs concatenated into filesystem calls
static RE_PATH_TRAVERSAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:fs\.(?:readFile|readFileSync|writeFile|writeFileSync|unlink|readdir|createReadStream)|path\.(?:join|resolve))\s*\([^)]*(?:req\.(?:params|query|body)|params\.|userInput|user_path|file_param)"#).unwrap()
});

/// Rule 18: Unbounded Memory Regex - Exponential catastrophic backtracking patterns
static RE_UNBOUNDED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"["'`/].*(\([a-zA-Z0-9_.]+\+[a-zA-Z0-9_.]*\)\+|\([a-zA-Z0-9_.]+\*[a-zA-Z0-9_.]*\)\*|\([a-zA-Z0-9_.]+\|\s*[a-zA-Z0-9_.]+\)\+).*["'`/]"#).unwrap()
});

/// Rule 19: Dynamic Code Eval - eval(), new Function(), or dangerous dynamic code execution
static RE_DYNAMIC_EVAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:eval\s*\(|new\s+Function\s*\(|window\.eval\s*\(|global\.eval\s*\()"#).unwrap()
});

/// Rule 20: Untyped Union Access - Direct unsafe type cast or union property without narrowing
static RE_UNSAFE_AS_ANY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bas\s+any\b|\bas\s+never\s+as\s+any\b"#).unwrap()
});

// ---------------------------------------------------------------------------
// Extended Invariants Verifier
// ---------------------------------------------------------------------------

pub struct InvariantsExtended;

impl InvariantsExtended {
    /// Rule 12: SQL Injection check
    pub fn check_sql_injection(source: &str) -> Option<String> {
        if let Some(m) = RE_SQL_RAW_CONCAT.find(source) {
            return Some(format!(
                "Potential SQL injection: unparameterized query interpolation at: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 13: Floating Promise check
    pub fn check_floating_promise(source: &str) -> Option<String> {
        if let Some(m) = RE_FLOATING_PROMISE.find(source) {
            return Some(format!(
                "Floating unhandled promise detected (missing await, void, or .catch()): '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 14: React State Race check
    pub fn check_react_state_race(source: &str) -> Option<String> {
        if let Some(m) = RE_REACT_STATE_RACE.find(source) {
            return Some(format!(
                "React state race condition: non-functional setState inside loop or async scope: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 15: Event Listener Leak check
    pub fn check_listener_leak(source: &str) -> Option<String> {
        for cap in RE_USE_EFFECT_BLOCK.captures_iter(source) {
            if let Some(body_match) = cap.name("body") {
                let body = body_match.as_str();
                if let Some(add_match) = RE_ADD_EVENT_LISTENER.find(body) {
                    if !RE_REMOVE_EVENT_LISTENER.is_match(body) {
                        return Some(format!(
                            "Event listener leak: '{}' in useEffect without removeEventListener cleanup",
                            add_match.as_str()
                        ));
                    }
                }
            }
        }
        None
    }

    /// Rule 16: Insecure Randomness check
    pub fn check_insecure_randomness(source: &str) -> Option<String> {
        if let Some(m) = RE_INSECURE_RANDOM.find(source) {
            return Some(format!(
                "Insecure randomness: Math.random() used in security-sensitive context: '{}'",
                m.as_str().trim()
            ));
        }
        if let Some(m) = RE_RANDOM_IN_SECURITY_VAR.find(source) {
            return Some(format!(
                "Insecure randomness: Math.random() assigned to security token/secret: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 17: Path Traversal check
    pub fn check_path_traversal(source: &str) -> Option<String> {
        if let Some(m) = RE_PATH_TRAVERSAL.find(source) {
            return Some(format!(
                "Potential path traversal: direct user parameter concatenated into filesystem path: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 18: Unbounded Memory Regex check
    pub fn check_unbounded_regex(source: &str) -> Option<String> {
        if let Some(m) = RE_UNBOUNDED_REGEX.find(source) {
            return Some(format!(
                "Unbounded regex catastrophic backtracking risk: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 19: Dynamic Code Eval check
    pub fn check_dynamic_code_eval(source: &str) -> Option<String> {
        if let Some(m) = RE_DYNAMIC_EVAL.find(source) {
            return Some(format!(
                "Forbidden dynamic code evaluation: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 20: Untyped Union Access check
    pub fn check_untyped_union_access(source: &str) -> Option<String> {
        if let Some(m) = RE_UNSAFE_AS_ANY.find(source) {
            return Some(format!(
                "Unsafe type escape 'as any' bypasses strict type safety: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Run all 9 extended checks and return first violation if any
    pub fn check_all(source: &str) -> Option<(ViolationKind, String)> {
        if let Some(detail) = Self::check_sql_injection(source) {
            return Some((ViolationKind::SqlInjection, detail));
        }
        if let Some(detail) = Self::check_floating_promise(source) {
            return Some((ViolationKind::FloatingPromise, detail));
        }
        if let Some(detail) = Self::check_react_state_race(source) {
            return Some((ViolationKind::ReactStateRace, detail));
        }
        if let Some(detail) = Self::check_listener_leak(source) {
            return Some((ViolationKind::ListenerLeak, detail));
        }
        if let Some(detail) = Self::check_insecure_randomness(source) {
            return Some((ViolationKind::InsecureRandomness, detail));
        }
        if let Some(detail) = Self::check_path_traversal(source) {
            return Some((ViolationKind::PathTraversal, detail));
        }
        if let Some(detail) = Self::check_unbounded_regex(source) {
            return Some((ViolationKind::UnboundedRegex, detail));
        }
        if let Some(detail) = Self::check_dynamic_code_eval(source) {
            return Some((ViolationKind::DynamicCodeEval, detail));
        }
        if let Some(detail) = Self::check_untyped_union_access(source) {
            return Some((ViolationKind::UntypedUnionAccess, detail));
        }
        None
    }
}
