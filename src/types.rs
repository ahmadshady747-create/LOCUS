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

/// Risk level classification for code modifications and architectural refactors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskScore {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RiskScore::Low => "LOW",
            RiskScore::Medium => "MEDIUM",
            RiskScore::High => "HIGH",
            RiskScore::Critical => "CRITICAL",
        };
        write!(f, "{}", s)
    }
}

/// An exact call-site reference or import occurrence of a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub file: String,
    pub line: usize,
    pub byte_offset: usize,
    pub context_snippet: String,
}

/// Fully resolved symbol metadata across module boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub signature: String,
    pub doc_comment: Option<String>,
    pub is_exported: bool,
}

/// Impact analysis and blast-radius report for a modified symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusReport {
    pub symbol: String,
    pub origin_file: String,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub affected_files: Vec<String>,
    pub risk_score: RiskScore,
    pub reference_count: usize,
    pub latency_ms: f64,
}

/// Workspace-wide architectural health audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalHealth {
    pub circular_dependencies: Vec<Vec<String>>,
    pub orphan_exports: Vec<String>,
    pub total_files: usize,
    pub total_symbols: usize,
    pub total_edges: usize,
    pub latency_ms: f64,
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
    /// Unmatched or improperly balanced JSX/HTML opening/closing tags or fragments.
    JsxTagMismatch,
    /// React hook called conditionally inside if statements, loops, or nested scopes.
    ConditionalHookCall,
    /// Server-side secret accessed directly in a client component ("use client").
    ClientSecretLeak,
    /// Direct raw HTML injection without sanitization.
    UnsafeInnerHtml,
    /// Unparameterized SQL queries or raw template string concatenation.
    SqlInjection,
    /// Unhandled async floating promises lacking await, catch, or return.
    FloatingPromise,
    /// Non-functional state updates inside asynchronous loops or delayed callbacks.
    ReactStateRace,
    /// Event listeners or subscriptions added without cleanup in unmount handlers.
    ListenerLeak,
    /// Weak pseudo-random number generator (e.g. Math.random) used in security contexts.
    InsecureRandomness,
    /// Unsanitized user inputs concatenated into filesystem paths.
    PathTraversal,
    /// Unbounded regex execution risking high-complexity denial of service.
    UnboundedRegex,
    /// Dynamic code execution via eval(), new Function(), or unvalidated dynamic imports.
    DynamicCodeEval,
    /// Direct access to polymorphic union properties without discriminant narrowing.
    UntypedUnionAccess,
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
            ViolationKind::JsxTagMismatch          => "JSX_TAG_MISMATCH",
            ViolationKind::ConditionalHookCall     => "CONDITIONAL_HOOK_CALL",
            ViolationKind::ClientSecretLeak        => "CLIENT_SECRET_LEAK",
            ViolationKind::UnsafeInnerHtml         => "UNSAFE_INNER_HTML",
            ViolationKind::SqlInjection            => "SQL_INJECTION",
            ViolationKind::FloatingPromise         => "FLOATING_PROMISE",
            ViolationKind::ReactStateRace          => "REACT_STATE_RACE",
            ViolationKind::ListenerLeak            => "LISTENER_LEAK",
            ViolationKind::InsecureRandomness      => "INSECURE_RANDOMNESS",
            ViolationKind::PathTraversal           => "PATH_TRAVERSAL",
            ViolationKind::UnboundedRegex          => "UNBOUNDED_REGEX",
            ViolationKind::DynamicCodeEval         => "DYNAMIC_CODE_EVAL",
            ViolationKind::UntypedUnionAccess      => "UNTYPED_UNION_ACCESS",
        };
        write!(f, "{}", s)
    }
}

/// Result produced by `AstGuard::verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// True if all invariant passes succeeded.
    pub passed: bool,
    /// The first violation detected, if any.
    pub violation: Option<ViolationKind>,
    /// Human-readable explanation of the violation.
    pub detail: Option<String>,
    /// All violations detected in this pass.
    pub violations: Vec<String>,
    /// Wall-clock time taken by the full verification pass.
    pub latency_ms: f64,
}

impl VerificationReport {
    pub fn passed(latency_ms: f64) -> Self {
        Self {
            passed: true,
            violation: None,
            detail: None,
            violations: Vec::new(),
            latency_ms,
        }
    }

    pub fn failed(violation: ViolationKind, detail: impl Into<String>, latency_ms: f64) -> Self {
        let d = detail.into();
        Self {
            passed: false,
            violation: Some(violation.clone()),
            detail: Some(d.clone()),
            violations: vec![format!("{}: {}", violation, d)],
            latency_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Transaction Types (Multi-File ACID Engine)
// ---------------------------------------------------------------------------

/// Unique identifier for an in-memory workspace transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub String);

impl TransactionId {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(format!("tx_{:x}", nanos))
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle status of an ACID workspace transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Open,
    Staged,
    Committed,
    RolledBack,
    Failed(String),
}

/// A single file modification staged in the in-memory shadow buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStagedFile {
    pub path: String,
    pub original_content: Option<String>,
    pub staged_content: String,
    pub language: Language,
}

/// Consolidated audit report of a committed or rolled-back transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReport {
    pub tx_id: TransactionId,
    pub status: TransactionStatus,
    pub total_staged_files: usize,
    pub passed_verification: bool,
    pub violations: Vec<String>,
    pub committed_files: Vec<String>,
    pub latency_ms: f64,
}

// ---------------------------------------------------------------------------
// Auto-Remediation Types
// ---------------------------------------------------------------------------

/// The structural class of automated code remediation applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemediationKind {
    JsxCloseTag,
    OptionalChaining,
    HookHoisting,
    Custom(String),
}

/// A deterministic, byte-accurate code edit applied by the remediation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationEdit {
    pub kind: RemediationKind,
    pub description: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub replacement: String,
}

/// Comprehensive outcome of an automated AST remediation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationResult {
    pub success: bool,
    pub original_code: String,
    pub remediated_code: String,
    pub edits_applied: Vec<RemediationEdit>,
    pub passed_verification: bool,
    pub latency_ms: f64,
}

// ---------------------------------------------------------------------------
// AST Query & Pattern Matching Types
// ---------------------------------------------------------------------------

/// Syntactic category of an incremental AST query node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AstQueryKind {
    CallExpression,
    FunctionDeclaration,
    Identifier,
    MemberAccess,
    BinaryExpression,
    TemplateLiteral,
    JsxElement,
    ImportDeclaration,
}

/// A matched AST node captured during an S-expression query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstQueryMatch {
    pub pattern: String,
    pub capture_name: String,
    pub node_kind: AstQueryKind,
    pub text: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

// ---------------------------------------------------------------------------
// Multi-Agent Symbol Lease Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolLease {
    pub lease_id: String,
    pub fqn: String,
    pub holder_agent_id: String,
    pub acquired_at_ms: u64,
    pub ttl_ms: u64,
    pub expires_at_ms: u64,
}

impl SymbolLease {
    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        current_time_ms >= self.expires_at_ms
    }

    pub fn remaining_ttl_ms(&self, current_time_ms: u64) -> u64 {
        self.expires_at_ms.saturating_sub(current_time_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseStatus {
    Acquired(SymbolLease),
    Conflict {
        fqn: String,
        current_holder: String,
        remaining_ttl_ms: u64,
    },
    Released,
    NotFound,
    Renewed(SymbolLease),
}

// ---------------------------------------------------------------------------
// Cross-File Taint & Data-Flow Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaintKind {
    UnvalidatedInput,
    NullableReturn,
    UncheckedEnvVar,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintSource {
    pub file: String,
    pub symbol: String,
    pub variable: String,
    pub kind: TaintKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintSink {
    pub file: String,
    pub symbol: String,
    pub line: usize,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintFlowReport {
    pub taint_id: String,
    pub source: TaintSource,
    pub flow_path: Vec<String>,
    pub sinks: Vec<TaintSink>,
    pub is_sanitized: bool,
    pub violation_risk: RiskScore,
    pub latency_ms: f64,
}

// ---------------------------------------------------------------------------
// In-Memory Quantized HNSW Search Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub symbol_name: String,
    pub file_path: String,
    pub signature: String,
    pub score: f32,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub query: String,
    pub total_hits: usize,
    pub hits: Vec<SearchHit>,
    pub latency_ms: f64,
}

// ---------------------------------------------------------------------------
// Language Enum (used by AstDiffEngine and SymbolGraph)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Tsx,
    Jsx,
    Svelte,
    Astro,
    Vue,
    Python,
    Unknown,
}

impl Language {
    /// Returns true if this language is a frontend component or script language.
    pub fn is_frontend(&self) -> bool {
        matches!(
            self,
            Language::TypeScript
                | Language::JavaScript
                | Language::Tsx
                | Language::Jsx
                | Language::Svelte
                | Language::Astro
                | Language::Vue
        )
    }

    /// Infer language from file extension or language name.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().trim() {
            "rs" | "rust" => Language::Rust,
            "tsx" | "react-tsx" => Language::Tsx,
            "jsx" | "react-jsx" => Language::Jsx,
            "svelte" => Language::Svelte,
            "astro" => Language::Astro,
            "vue" => Language::Vue,
            "ts" | "typescript" => Language::TypeScript,
            "js" | "javascript" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "python" => Language::Python,
            _ => Language::Unknown,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::TypeScript => write!(f, "typescript"),
            Language::JavaScript => write!(f, "javascript"),
            Language::Tsx => write!(f, "tsx"),
            Language::Jsx => write!(f, "jsx"),
            Language::Svelte => write!(f, "svelte"),
            Language::Astro => write!(f, "astro"),
            Language::Vue => write!(f, "vue"),
            Language::Python => write!(f, "python"),
            Language::Unknown => write!(f, "unknown"),
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
        assert_eq!(ViolationKind::JsxTagMismatch.to_string(),       "JSX_TAG_MISMATCH");
        assert_eq!(ViolationKind::ConditionalHookCall.to_string(),  "CONDITIONAL_HOOK_CALL");
        assert_eq!(ViolationKind::ClientSecretLeak.to_string(),     "CLIENT_SECRET_LEAK");
        assert_eq!(ViolationKind::UnsafeInnerHtml.to_string(),      "UNSAFE_INNER_HTML");
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"),     Language::Rust);
        assert_eq!(Language::from_extension("ts"),     Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"),    Language::Tsx);
        assert_eq!(Language::from_extension("jsx"),    Language::Jsx);
        assert_eq!(Language::from_extension("svelte"), Language::Svelte);
        assert_eq!(Language::from_extension("astro"),  Language::Astro);
        assert_eq!(Language::from_extension("vue"),    Language::Vue);
        assert_eq!(Language::from_extension("py"),     Language::Python);
        assert_eq!(Language::from_extension("csv"),    Language::Unknown);
        assert!(Language::Tsx.is_frontend());
        assert!(!Language::Rust.is_frontend());
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
