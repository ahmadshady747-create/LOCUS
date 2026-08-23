//! High-Throughput Bitset-Driven Invariant Scanner.
//!
//! Executes all 20 deterministic invariant rules in <50µs per file with
//! configurable bitset mask for surgical or comprehensive verification.

#![forbid(unsafe_code)]

use std::time::Instant;
use crate::types::{VerificationReport, ViolationKind};
use crate::guard::AstGuard;
use crate::guard::invariants_extended::InvariantsExtended;

/// 32-bit bitset representing active verification rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMask(pub u32);

impl RuleMask {
    pub const ALL_RULES: Self = Self(0xFFFFF); // 20 rules (bits 0..19)
    pub const CORE_RULES: Self = Self(0x7FF);   // Rules 0..10
    pub const EXTENDED_RULES: Self = Self(0xFF800); // Rules 11..19

    pub fn with_rule(mut self, bit: u8) -> Self {
        self.0 |= 1 << bit;
        self
    }

    pub fn without_rule(mut self, bit: u8) -> Self {
        self.0 &= !(1 << bit);
        self
    }

    pub fn is_active(&self, bit: u8) -> bool {
        (self.0 & (1 << bit)) != 0
    }
}

impl Default for RuleMask {
    fn default() -> Self {
        Self::ALL_RULES
    }
}

/// Bitset-driven invariant rule runner.
pub struct RuleRunner;

impl RuleRunner {
    /// Execute configured rule suite on source string.
    pub fn verify_with_mask(source: &str, mask: RuleMask) -> VerificationReport {
        let start = Instant::now();
        let mut violations = Vec::new();
        let mut first_violation: Option<ViolationKind> = None;
        let mut first_detail: Option<String> = None;

        macro_rules! check_pass {
            ($bit:expr, $kind:expr, $check_expr:expr) => {
                if mask.is_active($bit) {
                    if let Some(detail) = $check_expr {
                        let msg = format!("{}: {}", $kind, detail);
                        if first_violation.is_none() {
                            first_violation = Some($kind);
                            first_detail = Some(detail);
                        }
                        violations.push(msg);
                    }
                }
            };
        }

        // Rule 0: Delimiter Balance
        if mask.is_active(0) && !AstGuard::check_delimiter_balance(source) {
            let kind = ViolationKind::UnbalancedDelimiters;
            let detail = "Source contains unbalanced braces, brackets, or parentheses.".to_string();
            first_violation = Some(kind.clone());
            first_detail = Some(detail.clone());
            violations.push(format!("{}: {}", kind, detail));
        }

        // Core rules (1..10)
        check_pass!(1, ViolationKind::AsyncMutexAcrossAwait, AstGuard::check_async_mutex(source));
        check_pass!(2, ViolationKind::DivisionByZero, AstGuard::check_div_by_zero(source));
        check_pass!(3, ViolationKind::ArrayOutOfBounds, AstGuard::check_array_bounds(source));
        check_pass!(4, ViolationKind::UnsafeUnwrap, AstGuard::check_unsafe_unwrap(source));
        check_pass!(5, ViolationKind::ReDoSPattern, AstGuard::check_redos(source));
        check_pass!(6, ViolationKind::NullDereference, AstGuard::check_null_deref(source));
        check_pass!(7, ViolationKind::ConditionalHookCall, AstGuard::check_conditional_hooks(source));
        check_pass!(8, ViolationKind::ClientSecretLeak, AstGuard::check_client_secret_leak(source));
        check_pass!(9, ViolationKind::UnsafeInnerHtml, AstGuard::check_unsafe_inner_html(source));
        check_pass!(10, ViolationKind::JsxTagMismatch, AstGuard::check_jsx_tags(source));

        // Extended rules (11..19)
        check_pass!(11, ViolationKind::SqlInjection, InvariantsExtended::check_sql_injection(source));
        check_pass!(12, ViolationKind::FloatingPromise, InvariantsExtended::check_floating_promise(source));
        check_pass!(13, ViolationKind::ReactStateRace, InvariantsExtended::check_react_state_race(source));
        check_pass!(14, ViolationKind::ListenerLeak, InvariantsExtended::check_listener_leak(source));
        check_pass!(15, ViolationKind::InsecureRandomness, InvariantsExtended::check_insecure_randomness(source));
        check_pass!(16, ViolationKind::PathTraversal, InvariantsExtended::check_path_traversal(source));
        check_pass!(17, ViolationKind::UnboundedRegex, InvariantsExtended::check_unbounded_regex(source));
        check_pass!(18, ViolationKind::DynamicCodeEval, InvariantsExtended::check_dynamic_code_eval(source));
        check_pass!(19, ViolationKind::UntypedUnionAccess, InvariantsExtended::check_untyped_union_access(source));

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        if violations.is_empty() {
            VerificationReport::passed(latency_ms)
        } else {
            VerificationReport {
                passed: false,
                violation: first_violation,
                detail: first_detail,
                violations,
                latency_ms,
            }
        }
    }

    /// Run all 20 rules on source code string.
    pub fn verify_all(source: &str) -> VerificationReport {
        Self::verify_with_mask(source, RuleMask::ALL_RULES)
    }
}
