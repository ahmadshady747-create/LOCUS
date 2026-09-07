//! Edge Case and Specification Gap Detector.
//!
//! Analyzes natural language intent and architectural requirements to discover
//! unspecified error handling, concurrency hazards, nullability gaps, and resource leaks.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use regex::Regex;

use crate::intent::socratic::types::{EdgeCaseKind, EdgeCaseViolation};

// ---------------------------------------------------------------------------
// Lexical Triggers for Edge-Case Detection (LazyLock)
// ---------------------------------------------------------------------------

// 1. Fallible / I/O operations lacking error handling
static RE_FALLIBLE_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(parse|fetch|read|write|connect|load|download|delete|execute|send|receive|call_api|request|http|socket|fs|file)\b").unwrap()
});
static RE_ERROR_MENTIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(error|result|try|catch|fail|handling|fallback|status|exception|recover|reject|err)\b").unwrap()
});

// 2. Asynchronous / Concurrent operations lacking synchronization specifications
static RE_CONCURRENCY_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(async|concurrent|thread|parallel|stream|channel|spawn|background|worker|swarm|task|queue)\b").unwrap()
});
static RE_CONCURRENCY_GUARDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(lock|mutex|rwlock|atomic|cancel|timeout|deadlock|await|join|select|sync|bounded|graceful)\b").unwrap()
});

// 3. Lookup / Indexing / Collection operations lacking nullability or boundary handling
static RE_LOOKUP_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(find|get|lookup|query|search|index|arr\[|slice|first|last|item_at|element)\b").unwrap()
});
static RE_BOUNDARY_GUARDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(empty|none|not_found|null|nil|optional|default|fallback|bounds|exists|404|unwrap_or)\b").unwrap()
});

// 4. Resource acquisition lacking deterministic cleanup
static RE_RESOURCE_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(open_file|acquire_lease|begin_tx|open_socket|connect_db|allocate|lock_guard|session)\b").unwrap()
});
static RE_RESOURCE_CLEANUP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(close|drop|rollback|release|cleanup|raii|dispose|defer|finally)\b").unwrap()
});

// 5. External input concatenation lacking validation or sanitization
static RE_INPUT_OPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(user_input|param|parameter|form_data|url|route_param|query_param|payload|sql_query|command)\b").unwrap()
});
static RE_INPUT_GUARDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(validate|sanitize|schema|escape|parameterized|guard|filter|check|verify)\b").unwrap()
});

// ---------------------------------------------------------------------------
// EdgeCaseDetector Implementation
// ---------------------------------------------------------------------------

pub struct EdgeCaseDetector;

impl EdgeCaseDetector {
    /// Detects all underspecified edge cases and safety gaps in the developer intent.
    pub fn detect(intent: &str) -> Vec<EdgeCaseViolation> {
        let mut violations = Vec::new();

        // 1. Check Missing Error Handling
        if RE_FALLIBLE_OPS.is_match(intent) && !RE_ERROR_MENTIONS.is_match(intent) {
            violations.push(EdgeCaseViolation {
                kind: EdgeCaseKind::MissingErrorHandling,
                description: "Intent involves fallible I/O, parsing, or network operations without defining error recovery or error variants.".to_string(),
                suggested_guard: "Specify typed error enum (e.g. `Result<T, CustomError>`) and define fallback behavior on network/disk failure.".to_string(),
            });
        }

        // 2. Check Concurrency Gaps
        if RE_CONCURRENCY_OPS.is_match(intent) && !RE_CONCURRENCY_GUARDS.is_match(intent) {
            violations.push(EdgeCaseViolation {
                kind: EdgeCaseKind::ConcurrencyGap,
                description: "Intent specifies concurrent, asynchronous, or worker execution without concurrency control, timeout, or cancellation mechanisms.".to_string(),
                suggested_guard: "Define thread-safe state container (e.g. `parking_lot::RwLock`, atomic counters) and timeout/cancellation handling.".to_string(),
            });
        }

        // 3. Check Nullability & Boundary Gaps
        if RE_LOOKUP_OPS.is_match(intent) && !RE_BOUNDARY_GUARDS.is_match(intent) {
            violations.push(EdgeCaseViolation {
                kind: EdgeCaseKind::NullabilityBoundaryGap,
                description: "Intent requests item lookup or array indexing without specifying the outcome when the target element is absent or index is out of bounds.".to_string(),
                suggested_guard: "Return `Option<T>` or specify a default fallback value when element is missing or collection is empty.".to_string(),
            });
        }

        // 4. Check Resource Cleanup Gaps
        if RE_RESOURCE_OPS.is_match(intent) && !RE_RESOURCE_CLEANUP.is_match(intent) {
            violations.push(EdgeCaseViolation {
                kind: EdgeCaseKind::ResourceCleanupGap,
                description: "Intent acquires persistent handles, database connections, or transactional locks without specifying release or rollback behavior.".to_string(),
                suggested_guard: "Ensure RAII Drop semantics or explicit transaction rollback / lease release blocks.".to_string(),
            });
        }

        // 5. Check Input Validation Gaps
        if RE_INPUT_OPS.is_match(intent) && !RE_INPUT_GUARDS.is_match(intent) {
            violations.push(EdgeCaseViolation {
                kind: EdgeCaseKind::InputValidationGap,
                description: "Intent accepts external inputs, URL params, or SQL constructs without defining validation boundaries or sanitization.".to_string(),
                suggested_guard: "Define strict schema validation and use parameterized queries / path traversal guards.".to_string(),
            });
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_missing_error_handling() {
        let prompt = "Fetch the user profile from the remote REST API and parse JSON response";
        let violations = EdgeCaseDetector::detect(prompt);
        assert!(violations.iter().any(|v| v.kind == EdgeCaseKind::MissingErrorHandling));
    }

    #[test]
    fn test_no_violation_when_error_handling_specified() {
        let prompt = "Fetch user profile from API, return Result<User, ApiError> and handle network timeout gracefully";
        let violations = EdgeCaseDetector::detect(prompt);
        assert!(!violations.iter().any(|v| v.kind == EdgeCaseKind::MissingErrorHandling));
    }

    #[test]
    fn test_detects_concurrency_gap() {
        let prompt = "Spawn 10 background worker tasks to process the job queue concurrently";
        let violations = EdgeCaseDetector::detect(prompt);
        assert!(violations.iter().any(|v| v.kind == EdgeCaseKind::ConcurrencyGap));
    }

    #[test]
    fn test_detects_lookup_boundary_gap() {
        let prompt = "Get user from array by index and return their name";
        let violations = EdgeCaseDetector::detect(prompt);
        assert!(violations.iter().any(|v| v.kind == EdgeCaseKind::NullabilityBoundaryGap));
    }
}
