//! Core type definitions for the locus-engine AST verification and graph subsystems.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Symbol Graph Types
// ---------------------------------------------------------------------------

/// Classifies the syntactic category of an extracted symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    Impl,
    Module,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SymbolKind::Function  => "fn",
            SymbolKind::Struct    => "struct",
            SymbolKind::Enum      => "enum",
            SymbolKind::Trait     => "trait",
            SymbolKind::TypeAlias => "type",
            SymbolKind::Const     => "const",
            SymbolKind::Impl      => "impl",
            SymbolKind::Module    => "mod",
        };
        write!(f, "{}", s)
    }
}

/// A single extracted symbol node in the cross-file SymbolGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    /// Stable 64-bit content-addressed ID (FNV-1a hash of file+name+kind).
    pub id: u64,
    pub name: String,
    pub kind: SymbolKind,
    /// Source file path relative to the index root.
    pub file: String,
    /// Byte offset of the start of the symbol declaration.
    pub byte_start: usize,
    /// Byte offset of the end of the symbol (closing brace / semicolon).
    pub byte_end: usize,
    /// First line of the declaration (signature only, no body).
    pub signature: String,
}

/// The relationship type carried by a `SymbolEdge`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Symbol A calls / uses symbol B.
    Uses,
    /// Symbol A implements trait B.
    Implements,
    /// File A imports symbol B from file B.
    Imports,
}

/// A directed edge in the cross-file SymbolGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEdge {
    pub from_id: u64,
    pub to_id: u64,
    pub edge_type: EdgeKind,
}

// ---------------------------------------------------------------------------
// Verification Types
// ---------------------------------------------------------------------------

/// A specific class of safety violation detected by `AstGuard`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationKind {
    /// Division by variable without a non-zero guard (e.g. `x / y` without `y != 0`).
    DivisionByZero,
    /// Array index access without bounds check.
    ArrayOutOfBounds,
    /// Direct `.unwrap()` / `.expect()` on Option or Result.
    UnsafeUnwrap,
    /// `std::sync::Mutex` lock held across a `.await` suspension point.
    AsyncMutexAcrossAwait,
    /// Catastrophic backtracking regex pattern (e.g. `(a+)+`).
    ReDoSPattern,
    /// Deep property access without null / optional-chaining guard (TS/JS).
    NullDereference,
    /// Unbalanced delimiters (braces, brackets, parentheses).
    UnbalancedDelimiters,
}

impl std::fmt::Display for ViolationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ViolationKind::DivisionByZero         => "DIV_BY_ZERO",
            ViolationKind::ArrayOutOfBounds        => "ARRAY_OOB",
            ViolationKind::UnsafeUnwrap            => "UNSAFE_UNWRAP",
            ViolationKind::AsyncMutexAcrossAwait   => "ASYNC_MUTEX_DEADLOCK",
            ViolationKind::ReDoSPattern            => "REDOS_PATTERN",
            ViolationKind::NullDereference         => "NULL_DEREF",
            ViolationKind::UnbalancedDelimiters    => "UNBALANCED_DELIMITERS",
        };
        write!(f, "{}", s)
    }
}

/// Result produced by `AstGuard::verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// True if all 6 invariant passes succeeded.
    pub passed: bool,
    /// The first violation detected, if any.
    pub violation: Option<ViolationKind>,
    /// Human-readable explanation of the violation.
    pub detail: Option<String>,
    /// Wall-clock time taken by the full verification pass.
    pub latency_ms: f64,
}

impl VerificationReport {
    pub fn passed(latency_ms: f64) -> Self {
        Self { passed: true, violation: None, detail: None, latency_ms }
    }

    pub fn failed(violation: ViolationKind, detail: impl Into<String>, latency_ms: f64) -> Self {
        Self {
            passed: false,
            violation: Some(violation),
            detail: Some(detail.into()),
            latency_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Language Enum (used by AstDiffEngine and SymbolGraph)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Unknown,
}

impl Language {
    /// Infer language from file extension or language name.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().trim() {
            "rs" | "rust" => Language::Rust,
            "ts" | "tsx" | "js" | "jsx" | "typescript" | "javascript" => Language::TypeScript,
            "py" | "python" => Language::Python,
            _ => Language::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// Hashing Utility
// ---------------------------------------------------------------------------

/// Fast non-cryptographic FNV-1a 64-bit hash for symbol IDs.
pub fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME:  u64 = 1099511628211;
    let mut hash = OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_display() {
        assert_eq!(SymbolKind::Function.to_string(),  "fn");
        assert_eq!(SymbolKind::Struct.to_string(),    "struct");
        assert_eq!(SymbolKind::Enum.to_string(),      "enum");
        assert_eq!(SymbolKind::Trait.to_string(),     "trait");
        assert_eq!(SymbolKind::TypeAlias.to_string(), "type");
        assert_eq!(SymbolKind::Impl.to_string(),      "impl");
    }

    #[test]
    fn test_violation_kind_display() {
        assert_eq!(ViolationKind::DivisionByZero.to_string(),       "DIV_BY_ZERO");
        assert_eq!(ViolationKind::AsyncMutexAcrossAwait.to_string(),"ASYNC_MUTEX_DEADLOCK");
        assert_eq!(ViolationKind::ReDoSPattern.to_string(),         "REDOS_PATTERN");
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"),  Language::Rust);
        assert_eq!(Language::from_extension("ts"),  Language::TypeScript);
        assert_eq!(Language::from_extension("py"),  Language::Python);
        assert_eq!(Language::from_extension("csv"), Language::Unknown);
    }

    #[test]
    fn test_fnv1a_stability() {
        let a = fnv1a_64(b"locus-engine");
        let b = fnv1a_64(b"locus-engine");
        assert_eq!(a, b);
        assert_ne!(a, fnv1a_64(b"locus-engine2"));
    }

    #[test]
    fn test_verification_report_constructors() {
        let r = VerificationReport::passed(0.021);
        assert!(r.passed);
        assert!(r.violation.is_none());

        let r2 = VerificationReport::failed(ViolationKind::DivisionByZero, "x / y", 0.01);
        assert!(!r2.passed);
        assert_eq!(r2.violation, Some(ViolationKind::DivisionByZero));
    }
}
