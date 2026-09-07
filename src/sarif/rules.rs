//! Canonical LOCUS Invariant Rule Definitions & OASIS SARIF Mapping.
//!
//! Maps AST verification invariant passes and security violation categories
//! to standardized SARIF reporting descriptors with full descriptions and severity ratings.

#![forbid(unsafe_code)]

use crate::meta_mcp::verifier::types::SecurityViolationKind;
use crate::sarif::types::{
    MultiformatMessageString, ReportingConfiguration, ReportingDescriptor, SarifLevel,
};
use crate::types::ViolationKind;
use serde_json::json;

/// Generates the complete, canonical list of LOCUS Invariant Reporting Descriptors for SARIF export.
pub fn get_canonical_rules() -> Vec<ReportingDescriptor> {
    vec![
        // Rule 0
        rule(
            "LOCUS001",
            "UnbalancedDelimiters",
            "Unbalanced parentheses, brackets, or braces detected in source AST.",
            "Syntactic delimiters (curly braces, square brackets, parentheses) must be strictly balanced across the source unit.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus001",
        ),
        // Rule 1
        rule(
            "LOCUS002",
            "AsyncMutexAcrossAwait",
            "Synchronous mutex lock held across an async await suspension point.",
            "Holding std::sync::Mutex or non-async locks across an .await boundary causes runtime deadlocks in task executors.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus002",
        ),
        // Rule 2
        rule(
            "LOCUS003",
            "DivisionByZero",
            "Unguarded arithmetic division by a variable operand.",
            "Division operations must have an explicit non-zero guard or verification proof on the denominator.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus003",
        ),
        // Rule 3
        rule(
            "LOCUS004",
            "ArrayOutOfBounds",
            "Array or slice indexed without an explicit bounds check.",
            "Direct array index operations without preceding bounds checking or safe iterator traversal risk runtime panics.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus004",
        ),
        // Rule 4
        rule(
            "LOCUS005",
            "UnsafeUnwrap",
            "Direct unwrap() or expect() invocation on an Option or Result.",
            "Unhandled unwrap calls violate pure safe software contracts; use pattern matching or proper error propagation.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus005",
        ),
        // Rule 5
        rule(
            "LOCUS006",
            "ReDoSPattern",
            "Catastrophic exponential backtracking regular expression detected.",
            "Regular expressions containing overlapping repeated groups or nested quantifiers risk Denial of Service.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus006",
        ),
        // Rule 6
        rule(
            "LOCUS007",
            "NullDereference",
            "Potential null or undefined property dereference without optional chaining.",
            "Deep property accesses must use optional chaining or explicit nullability guards.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus007",
        ),
        // Rule 7
        rule(
            "LOCUS008",
            "ConditionalHookCall",
            "React Hook called conditionally inside an if statement or loop.",
            "React hooks must be invoked at the top level of function components in a deterministic order.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus008",
        ),
        // Rule 8
        rule(
            "LOCUS009",
            "ClientSecretLeak",
            "Server-side secret or environment variable accessed in a client component.",
            "Client-rendered components marked with 'use client' must never directly reference private server secrets or tokens.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus009",
        ),
        // Rule 9
        rule(
            "LOCUS010",
            "UnsafeInnerHtml",
            "Direct unescaped HTML injection via dangerouslySetInnerHTML or innerHTML.",
            "Raw HTML string assignment exposes applications to Cross-Site Scripting (XSS); sanitize with trusted sanitizers.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus010",
        ),
        // Rule 10
        rule(
            "LOCUS011",
            "JsxTagMismatch",
            "Improperly nested or unbalanced JSX/HTML opening and closing tags.",
            "JSX element hierarchies must maintain strict balance between opening and closing tags.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus011",
        ),
        // Rule 11
        rule(
            "LOCUS012",
            "SqlInjection",
            "Unparameterized dynamic SQL query construction.",
            "SQL statements constructed via string interpolation or concatenation risk SQL injection; use parameterized queries.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus012",
        ),
        // Rule 12
        rule(
            "LOCUS013",
            "FloatingPromise",
            "Unhandled asynchronous Promise lacking await, then, or catch.",
            "Asynchronous Promises must be awaited or handled explicitly with catch to prevent silent failures.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus013",
        ),
        // Rule 13
        rule(
            "LOCUS014",
            "ReactStateRace",
            "Non-functional state update executed inside asynchronous callbacks.",
            "State updates dependent on previous state inside asynchronous handlers must use functional updaters.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus014",
        ),
        // Rule 14
        rule(
            "LOCUS015",
            "ListenerLeak",
            "Event listener or subscription added without unmount cleanup.",
            "Subscriptions and event listeners created in effect hooks must return a cleanup disposal callback.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus015",
        ),
        // Rule 15
        rule(
            "LOCUS016",
            "InsecureRandomness",
            "Weak pseudo-random number generator used in security-sensitive context.",
            "Cryptographic keys, nonces, and session tokens must use cryptographically secure RNGs.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus016",
        ),
        // Rule 16
        rule(
            "LOCUS017",
            "PathTraversal",
            "Unsanitized user input joined into filesystem path.",
            "Filesystem paths must be sanitized against directory traversal sequences (e.g. '../').",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus017",
        ),
        // Rule 17
        rule(
            "LOCUS018",
            "UnboundedRegex",
            "Unbounded regular expression processing untrusted inputs.",
            "Regular expressions should enforce maximum input lengths and execution timeouts.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus018",
        ),
        // Rule 18
        rule(
            "LOCUS019",
            "DynamicCodeEval",
            "Dynamic code execution via eval(), new Function(), or arbitrary string compilation.",
            "Dynamic code evaluation exposes the process to arbitrary code execution and violates sandbox integrity.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus019",
        ),
        // Rule 19
        rule(
            "LOCUS020",
            "UntypedUnionAccess",
            "Polymorphic union or dynamic object property accessed without discriminant narrowing.",
            "Union properties must be narrowed with type guards or discriminant checks before direct field access.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus020",
        ),
        // Extended Rules (Rules 20..31)
        rule(
            "LOCUS021",
            "CircularMemLeak",
            "Circular reference cycle in reference-counted types without Weak references.",
            "Self-referential or cyclic graph structures must break cycles using Weak pointers to prevent memory leaks.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus021",
        ),
        rule(
            "LOCUS022",
            "AsyncCancellationSafety",
            "Non-atomic state mutations across cancellation points in async tasks.",
            "State modifications across cancellation yield points must use transactions or cancellation tokens.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus022",
        ),
        rule(
            "LOCUS023",
            "ConstantTimeCrypto",
            "Variable-time comparison used in secret or password verification.",
            "Security-sensitive comparisons must execute in constant time to prevent side-channel timing attacks.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus023",
        ),
        rule(
            "LOCUS024",
            "ExhaustiveEnumNarrowing",
            "Incomplete pattern match omitting enum variants.",
            "Enum matches must be exhaustive to prevent unhandled default states.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus024",
        ),
        rule(
            "LOCUS025",
            "ResourceDescriptorLeak",
            "File handle, socket, or process descriptor lacking deterministic drop.",
            "Acquired system descriptors must implement RAII cleanup or deterministic close blocks.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus025",
        ),
        rule(
            "LOCUS026",
            "SsrfUnsafeFetch",
            "Outbound network request targeting internal or private IP address ranges.",
            "Outbound network clients must block internal/link-local/metadata IP ranges to prevent SSRF.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus026",
        ),
        rule(
            "LOCUS027",
            "UnboundedChannelDeadlock",
            "Unbounded synchronous channel send in single-threaded async executor.",
            "Channel operations should use bounded capacities with async await backpressure.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus027",
        ),
        rule(
            "LOCUS028",
            "PrototypePollution",
            "Unsanitized object merge/clone exposing Object.prototype pollution.",
            "Recursive object merge operations must reject '__proto__' and 'constructor' keys.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus028",
        ),
        rule(
            "LOCUS029",
            "CorsWildcardCredentials",
            "Insecure CORS wildcard origin header combined with credential inclusion.",
            "Access-Control-Allow-Origin must not be '*' when Access-Control-Allow-Credentials is true.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus029",
        ),
        rule(
            "LOCUS030",
            "HardcodedKeyEntropy",
            "High-entropy static cryptographic secret or token hardcoded in source.",
            "Secrets, tokens, and private keys must be loaded dynamically from secret stores.",
            SarifLevel::Error,
            "https://locus.dev/docs/invariants/locus030",
        ),
        rule(
            "LOCUS031",
            "UncheckedArithmeticOverflow",
            "Unchecked arithmetic operations risking integer overflow in critical loops.",
            "High-risk calculations must use checked or saturating arithmetic methods.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus031",
        ),
        rule(
            "LOCUS032",
            "AtomicStateMutation",
            "Direct non-functional state mutation in concurrent shared containers.",
            "Concurrent state mutations must use atomic primitives or synchronized transactions.",
            SarifLevel::Warning,
            "https://locus.dev/docs/invariants/locus032",
        ),
    ]
}

/// Helper to build a `ReportingDescriptor`.
fn rule(
    id: &str,
    name: &str,
    short_desc: &str,
    full_desc: &str,
    level: SarifLevel,
    help_uri: &str,
) -> ReportingDescriptor {
    ReportingDescriptor {
        id: id.to_string(),
        name: Some(name.to_string()),
        short_description: Some(MultiformatMessageString::new(short_desc)),
        full_description: Some(MultiformatMessageString::new(full_desc)),
        default_configuration: Some(ReportingConfiguration {
            level,
            rank: Some(if level == SarifLevel::Error { 10.0 } else { 5.0 }),
        }),
        help_uri: Some(help_uri.to_string()),
        properties: Some(json!({
            "category": "security/correctness",
            "precision": "high",
            "tags": ["locus", "invariants", "zero-unsafe"]
        })),
    }
}

/// Maps a `ViolationKind` to its canonical SARIF rule ID.
pub fn rule_id_for_violation(kind: &ViolationKind) -> &'static str {
    match kind {
        ViolationKind::UnbalancedDelimiters => "LOCUS001",
        ViolationKind::AsyncMutexAcrossAwait => "LOCUS002",
        ViolationKind::DivisionByZero => "LOCUS003",
        ViolationKind::ArrayOutOfBounds => "LOCUS004",
        ViolationKind::UnsafeUnwrap => "LOCUS005",
        ViolationKind::ReDoSPattern => "LOCUS006",
        ViolationKind::NullDereference => "LOCUS007",
        ViolationKind::ConditionalHookCall => "LOCUS008",
        ViolationKind::ClientSecretLeak => "LOCUS009",
        ViolationKind::UnsafeInnerHtml => "LOCUS010",
        ViolationKind::JsxTagMismatch => "LOCUS011",
        ViolationKind::SqlInjection => "LOCUS012",
        ViolationKind::FloatingPromise => "LOCUS013",
        ViolationKind::ReactStateRace => "LOCUS014",
        ViolationKind::ListenerLeak => "LOCUS015",
        ViolationKind::InsecureRandomness => "LOCUS016",
        ViolationKind::PathTraversal => "LOCUS017",
        ViolationKind::UnboundedRegex => "LOCUS018",
        ViolationKind::DynamicCodeEval => "LOCUS019",
        ViolationKind::UntypedUnionAccess => "LOCUS020",
        ViolationKind::CircularMemLeak => "LOCUS021",
        ViolationKind::AsyncCancellationSafety => "LOCUS022",
        ViolationKind::ConstantTimeCrypto => "LOCUS023",
        ViolationKind::ExhaustiveEnumNarrowing => "LOCUS024",
        ViolationKind::ResourceDescriptorLeak => "LOCUS025",
        ViolationKind::SsrfUnsafeFetch => "LOCUS026",
        ViolationKind::UnboundedChannelDeadlock => "LOCUS027",
        ViolationKind::PrototypePollution => "LOCUS028",
        ViolationKind::CorsWildcardCredentials => "LOCUS029",
        ViolationKind::HardcodedKeyEntropy => "LOCUS030",
        ViolationKind::UncheckedArithmeticOverflow => "LOCUS031",
        ViolationKind::AtomicStateMutation => "LOCUS032",
    }
}

/// Maps a sandbox/verifier `SecurityViolationKind` to its canonical SARIF rule ID.
pub fn rule_id_for_security_violation(kind: &SecurityViolationKind) -> &'static str {
    match kind {
        SecurityViolationKind::UnauthorizedFileAccess => "LOCUS017",
        SecurityViolationKind::ProcessExecutionAttempt => "LOCUS019",
        SecurityViolationKind::UnboundedLoopDetected => "LOCUS018",
        SecurityViolationKind::CapabilityMismatch => "LOCUS020",
        SecurityViolationKind::ExecutionTimeoutViolation => "LOCUS018",
    }
}

/// Looks up a rule descriptor by rule ID.
pub fn lookup_rule(rule_id: &str) -> Option<ReportingDescriptor> {
    get_canonical_rules().into_iter().find(|r| r.id == rule_id)
}
