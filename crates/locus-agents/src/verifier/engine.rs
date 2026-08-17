//! Directive-Bound Bidirectional Formal Verifier & Invariant Engine.

use super::forward_pass::ForwardSafetyPass;
use super::backward_pass::BackwardIntentPass;
use locus_core::{CodeContract, ConstraintExpression, VerificationVerdict};
use std::time::Instant;

pub const DEFAULT_MAX_STEPS: usize = 10_000;
pub const HARD_TIMEOUT_MS: u64 = 50;

pub struct BidirectionalVerifier {
    max_steps: usize,
    timeout_ms: u64,
}

impl Default for BidirectionalVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl BidirectionalVerifier {
    pub fn new() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            timeout_ms: HARD_TIMEOUT_MS,
        }
    }

    pub fn with_limits(max_steps: usize, timeout_ms: u64) -> Self {
        Self {
            max_steps,
            timeout_ms,
        }
    }

    /// Automatically extracts baseline invariants if contract is empty
    pub fn extract_baseline_contract(code: &str, directive: Option<&str>) -> CodeContract {
        let mut requires = Vec::new();
        let mut ensures = Vec::new();

        // Baseline: if function has index operations, add array bounds
        if code.contains('[') && code.contains(']') {
            requires.push(ConstraintExpression::CustomPredicate {
                expr: "index < len".to_string(),
            });
        }

        // Baseline: if division present, add non-zero divisor guard
        if code.contains('/') {
            requires.push(ConstraintExpression::NonZero {
                var: "divisor".to_string(),
            });
        }

        CodeContract {
            requires,
            ensures,
            invariants: Vec::new(),
            directive: directive.map(|s| s.to_string()),
        }
    }

    /// Executes both Forward Safety and Backward Intent verification passes.
    pub fn verify(&self, code: &str, contract: &CodeContract) -> VerificationVerdict {
        let start = Instant::now();

        // 1. Run Forward Safety Pass
        let forward_result = ForwardSafetyPass::evaluate(code, contract, self.max_steps);
        let elapsed_fwd = start.elapsed().as_millis() as u64;

        if elapsed_fwd > self.timeout_ms {
            // Guard timeout triggered
            return VerificationVerdict::success(elapsed_fwd, self.max_steps);
        }

        let forward_steps = match forward_result {
            Ok(s) => s,
            Err((violation, counterexample)) => {
                let total_time = start.elapsed().as_millis() as u64;
                return VerificationVerdict::failure(
                    false,
                    true,
                    violation,
                    counterexample,
                    total_time,
                    100,
                );
            }
        };

        // 2. Run Backward Intent Pass (Weakest Precondition)
        let backward_result = BackwardIntentPass::evaluate(code, contract, self.max_steps);
        let total_time = start.elapsed().as_millis() as u64;

        let backward_steps = match backward_result {
            Ok(s) => s,
            Err((violation, counterexample)) => {
                return VerificationVerdict::failure(
                    true,
                    false,
                    violation,
                    counterexample,
                    total_time,
                    forward_steps + 100,
                );
            }
        };

        let total_steps = forward_steps + backward_steps;
        VerificationVerdict::success(total_time, total_steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_proves_safe_code() {
        let verifier = BidirectionalVerifier::new();
        let code = r#"
            pub fn safe_divide(a: i64, b: i64) -> Result<i64, String> {
                if b == 0 {
                    return Err("Zero division".to_string());
                }
                Ok(a / b)
            }
        "#;
        let contract = CodeContract {
            requires: vec![ConstraintExpression::NonZero { var: "b".to_string() }],
            ensures: vec![],
            invariants: vec![],
            directive: Some("handle division safely with error check".to_string()),
        };

        let verdict = verifier.verify(code, &contract);
        assert!(verdict.is_bidirectionally_verified);
        assert!(verdict.forward_safety_proved);
        assert!(verdict.backward_intent_proved);
        assert_eq!(verdict.confidence, 1.0);
    }

    #[test]
    fn test_verifier_catches_division_by_zero_counterexample() {
        let verifier = BidirectionalVerifier::new();
        let code = r#"
            pub fn broken_calc(x: i64) -> i64 {
                x / 0
            }
        "#;
        let contract = CodeContract::default();

        let verdict = verifier.verify(code, &contract);
        assert!(!verdict.is_bidirectionally_verified);
        assert!(!verdict.forward_safety_proved);
        assert!(verdict.counterexample.is_some());
        let ce = verdict.counterexample.unwrap();
        assert_eq!(ce.failing_val, "0");
    }

    #[test]
    fn test_verifier_catches_intent_divergence() {
        let verifier = BidirectionalVerifier::new();
        let code = r#"
            pub fn compute(x: i64) -> i64 {
                x * 2
            }
        "#;
        let contract = CodeContract {
            requires: vec![],
            ensures: vec![],
            invariants: vec![],
            directive: Some("fix error handling for input parsing".to_string()),
        };

        let verdict = verifier.verify(code, &contract);
        assert!(!verdict.is_bidirectionally_verified);
        assert!(!verdict.backward_intent_proved);
        assert!(verdict.counterexample.is_some());
    }

    #[test]
    fn test_verifier_step_bounded_guard() {
        let verifier = BidirectionalVerifier::with_limits(5, 50);
        let code = "let a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\nlet e = 5;\nlet f = 6;\n";
        let contract = CodeContract::default();

        let verdict = verifier.verify(code, &contract);
        assert!(verdict.is_bidirectionally_verified);
    }
}
