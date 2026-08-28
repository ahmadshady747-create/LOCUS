//! High-Throughput Bitset-Driven Invariant Scanner (32 Enterprise Safety Rules).
//!
//! Executes all 32 deterministic invariant rules in <50µs per file with
//! configurable 32-bit bitset mask for surgical or comprehensive verification.

#![forbid(unsafe_code)]

use crate::guard::invariants_extended::InvariantsExtended;
use crate::guard::AstGuard;
use crate::types::{VerificationReport, ViolationKind};
use std::time::Instant;

/// 32-bit bitset representing active verification rules (Rules 0..31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMask(pub u32);

impl RuleMask {
    pub const ALL_RULES: Self = Self(0xFFFFFFFF); // All 32 rules (bits 0..31)
    pub const ALL_32: Self = Self::ALL_RULES;     // Alias for All 32 rules
    pub const CORE_RULES: Self = Self(0x7FF);      // Rules 0..10 (Core AST & JSX)
    pub const EXTENDED_RULES: Self = Self(0xFFFFF800); // Rules 11..31 (Enterprise Security & Concurrency)

    pub fn all() -> Self {
        Self::ALL_RULES
    }

    pub fn core() -> Self {
        Self::CORE_RULES
    }

    pub fn extended() -> Self {
        Self::EXTENDED_RULES
    }

    pub fn with_rule(mut self, bit: u8) -> Self {
        if bit < 32 {
            self.0 |= 1 << bit;
        }
        self
    }

    pub fn without_rule(mut self, bit: u8) -> Self {
        if bit < 32 {
            self.0 &= !(1 << bit);
        }
        self
    }

    pub fn is_active(&self, bit: u8) -> bool {
        if bit < 32 {
            (self.0 & (1 << bit)) != 0
        } else {
            false
        }
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
        check_pass!(
            1,
            ViolationKind::AsyncMutexAcrossAwait,
            AstGuard::check_async_mutex(source)
        );
        check_pass!(
            2,
            ViolationKind::DivisionByZero,
            AstGuard::check_div_by_zero(source)
        );
        check_pass!(
            3,
            ViolationKind::ArrayOutOfBounds,
            AstGuard::check_array_bounds(source)
        );
        check_pass!(
            4,
            ViolationKind::UnsafeUnwrap,
            AstGuard::check_unsafe_unwrap(source)
        );
        check_pass!(
            5,
            ViolationKind::ReDoSPattern,
            AstGuard::check_redos(source)
        );
        check_pass!(
            6,
            ViolationKind::NullDereference,
            AstGuard::check_null_deref(source)
        );
        check_pass!(
            7,
            ViolationKind::ConditionalHookCall,
            AstGuard::check_conditional_hooks(source)
        );
        check_pass!(
            8,
            ViolationKind::ClientSecretLeak,
            AstGuard::check_client_secret_leak(source)
        );
        check_pass!(
            9,
            ViolationKind::UnsafeInnerHtml,
            AstGuard::check_unsafe_inner_html(source)
        );
        check_pass!(
            10,
            ViolationKind::JsxTagMismatch,
            AstGuard::check_jsx_tags(source)
        );

        // Extended rules (11..19)
        check_pass!(
            11,
            ViolationKind::SqlInjection,
            InvariantsExtended::check_sql_injection(source)
        );
        check_pass!(
            12,
            ViolationKind::FloatingPromise,
            InvariantsExtended::check_floating_promise(source)
        );
        check_pass!(
            13,
            ViolationKind::ReactStateRace,
            InvariantsExtended::check_react_state_race(source)
        );
        check_pass!(
            14,
            ViolationKind::ListenerLeak,
            InvariantsExtended::check_listener_leak(source)
        );
        check_pass!(
            15,
            ViolationKind::InsecureRandomness,
            InvariantsExtended::check_insecure_randomness(source)
        );
        check_pass!(
            16,
            ViolationKind::PathTraversal,
            InvariantsExtended::check_path_traversal(source)
        );
        check_pass!(
            17,
            ViolationKind::UnboundedRegex,
            InvariantsExtended::check_unbounded_regex(source)
        );
        check_pass!(
            18,
            ViolationKind::DynamicCodeEval,
            InvariantsExtended::check_dynamic_code_eval(source)
        );
        check_pass!(
            19,
            ViolationKind::UntypedUnionAccess,
            InvariantsExtended::check_untyped_union_access(source)
        );

        // Enterprise & Concurrency Invariants (20..31)
        check_pass!(
            20,
            ViolationKind::CircularMemLeak,
            InvariantsExtended::check_circular_mem_leak(source)
        );
        check_pass!(
            21,
            ViolationKind::AsyncCancellationSafety,
            InvariantsExtended::check_async_cancellation_safety(source)
        );
        check_pass!(
            22,
            ViolationKind::ConstantTimeCrypto,
            InvariantsExtended::check_constant_time_crypto(source)
        );
        check_pass!(
            23,
            ViolationKind::ExhaustiveEnumNarrowing,
            InvariantsExtended::check_exhaustive_enum_narrowing(source)
        );
        check_pass!(
            24,
            ViolationKind::ResourceDescriptorLeak,
            InvariantsExtended::check_resource_descriptor_leak(source)
        );
        check_pass!(
            25,
            ViolationKind::SsrfUnsafeFetch,
            InvariantsExtended::check_ssrf_unsafe_fetch(source)
        );
        check_pass!(
            26,
            ViolationKind::UnboundedChannelDeadlock,
            InvariantsExtended::check_unbounded_channel_deadlock(source)
        );
        check_pass!(
            27,
            ViolationKind::PrototypePollution,
            InvariantsExtended::check_prototype_pollution(source)
        );
        check_pass!(
            28,
            ViolationKind::CorsWildcardCredentials,
            InvariantsExtended::check_cors_wildcard_credentials(source)
        );
        check_pass!(
            29,
            ViolationKind::HardcodedKeyEntropy,
            InvariantsExtended::check_hardcoded_key_entropy(source)
        );
        check_pass!(
            30,
            ViolationKind::UncheckedArithmeticOverflow,
            InvariantsExtended::check_unchecked_arithmetic_overflow(source)
        );
        check_pass!(
            31,
            ViolationKind::AtomicStateMutation,
            InvariantsExtended::check_atomic_state_mutation(source)
        );

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

    /// Run all 32 rules on source code string.
    pub fn verify_all(source: &str) -> VerificationReport {
        Self::verify_with_mask(source, RuleMask::ALL_RULES)
    }
}
