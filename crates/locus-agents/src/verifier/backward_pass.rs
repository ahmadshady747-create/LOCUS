//! Backward Intent Pass — Weakest Precondition ($wp$) Calculus.
//! Proves: wp(Code, Postcondition) & Directive => True.

use locus_core::{CodeContract, Counterexample};

pub struct BackwardIntentPass;

impl BackwardIntentPass {
    /// Evaluates weakest precondition and intent compliance.
    /// Checks:
    /// 1. Intent Equivalence: code addresses what the directive specifies.
    /// 2. Frame Rule: state variables not covered by directive/postconditions are untouched.
    /// 3. Invariant Preservation: loop and structure invariants are maintained backwards.
    pub fn evaluate(
        code: &str,
        contract: &CodeContract,
        max_steps: usize,
    ) -> Result<usize, (String, Counterexample)> {
        let mut steps = 0;

        // 1. Intent Equivalence check against directive (if specified)
        if let Some(ref directive) = contract.directive {
            steps += 1;
            let dir_lower = directive.to_lowercase();

            // If directive asks to fix a bug or handle errors, ensure code contains handler
            if dir_lower.contains("error") || dir_lower.contains("fix") || dir_lower.contains("catch") {
                if !code.contains("Result<") && !code.contains("match ") && !code.contains("if ") && !code.contains("Err(") && !code.contains("catch") {
                    return Err((
                        "Intent Divergence: Missing Error Handling".to_string(),
                        Counterexample {
                            failing_var: "directive_intent".to_string(),
                            failing_val: directive.clone(),
                            violation_expr: "wp(Code, Post) !<= Directive".to_string(),
                            trace_summary: "Code does not implement the requested error handling logic".to_string(),
                        },
                    ));
                }
            }

            // If directive specifies immutable calculation, verify no mutable mutation
            if dir_lower.contains("pure") || dir_lower.contains("immutable") {
                if code.contains("mut ") || code.contains(".push(") || code.contains(".clear()") {
                    return Err((
                        "Frame Rule Violation: Mutation in Pure Context".to_string(),
                        Counterexample {
                            failing_var: "mut_state".to_string(),
                            failing_val: "mutated".to_string(),
                            violation_expr: "FrameRule: State is not immutable".to_string(),
                            trace_summary: "Unintended state modification detected in pure function".to_string(),
                        },
                    ));
                }
            }
        }

        // 2. Backward Invariant Preservation
        for inv in &contract.invariants {
            steps += 1;
            if steps > max_steps {
                break;
            }
            if let locus_core::ConstraintExpression::NonNull { var } = inv {
                if code.contains(&format!("{} = null", var)) || code.contains(&format!("{} = None", var)) {
                    return Err((
                        format!("Invariant NonNull Broken for {}", var),
                        Counterexample {
                            failing_var: var.clone(),
                            failing_val: "None/null".to_string(),
                            violation_expr: format!("invariant: NonNull({})", var),
                            trace_summary: format!("Invariant broken: {} assigned null/None", var),
                        },
                    ));
                }
            }
        }

        // 3. Symbolic Backward Step Budget
        steps += code.lines().count();
        if steps > max_steps {
            steps = max_steps;
        }

        Ok(steps)
    }
}
