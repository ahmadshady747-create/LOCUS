//! Dijkstra's Weakest Precondition Calculus & Hoare Logic Prover Barrel.
//!
//! Exposes wp-calculus engines, semantic logic solvers, Hoare triple verification,
//! and automated counterexample synthesis in 100% safe Rust.

#![forbid(unsafe_code)]

pub mod calculus;
pub mod solver;
pub mod types;

pub use calculus::WeakestPreconditionEngine;
pub use solver::SemanticSolver;
pub use types::{
    AstExpression, AstStatement, FormalError, HoareTriple, ProofVerdict, SemanticPredicate,
};

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_division_by_zero_wp_detection_and_safe_guard_verification() {
        // Statement: x := 100 / y
        let div_stmt = AstStatement::Assign {
            var: "x".to_string(),
            expr: AstExpression::Div(
                Box::new(AstExpression::Const(100)),
                Box::new(AstExpression::Var("y".to_string())),
            ),
        };

        // Postcondition: true (we just want the division to be safe)
        let post = SemanticPredicate::True;

        // 1. Unsafe case: Precondition is True (no non-zero check on y)
        let unsafe_triple = HoareTriple::new(SemanticPredicate::True, div_stmt.clone(), post.clone());
        let verdict_unsafe = SemanticSolver::verify_hoare_triple(&unsafe_triple);
        match verdict_unsafe {
            ProofVerdict::ViolationFound { counterexample, .. } => {
                assert!(counterexample.contains("violates non-zero division safety"));
            }
            ProofVerdict::ProvenSafe => panic!("Expected violation on unguarded division"),
        }

        // 2. Safe case: Precondition is y > 0
        let safe_triple = HoareTriple::new(
            SemanticPredicate::Gt("y".to_string(), 0),
            div_stmt,
            post,
        );
        let verdict_safe = SemanticSolver::verify_hoare_triple(&safe_triple);
        assert_eq!(verdict_safe, ProofVerdict::ProvenSafe);
    }

    #[test]
    fn test_array_bounds_check_hoare_triple_verification() {
        // Statement: i := i + 1
        let inc_stmt = AstStatement::Assign {
            var: "i".to_string(),
            expr: AstExpression::Add(
                Box::new(AstExpression::Var("i".to_string())),
                Box::new(AstExpression::Const(1)),
            ),
        };

        // Postcondition: 0 <= i <= 10
        let post_bounds = SemanticPredicate::BoundsCheck {
            var: "i".to_string(),
            min: 0,
            max: 10,
        };

        // 1. Safe case: Precondition is 0 <= i <= 9
        let safe_triple = HoareTriple::new(
            SemanticPredicate::BoundsCheck {
                var: "i".to_string(),
                min: 0,
                max: 9,
            },
            inc_stmt.clone(),
            post_bounds.clone(),
        );
        let verdict_safe = SemanticSolver::verify_hoare_triple(&safe_triple);
        assert_eq!(verdict_safe, ProofVerdict::ProvenSafe);

        // 2. Unsafe case: Precondition is 0 <= i <= 10 (which allows i = 10, resulting in i = 11)
        let unsafe_triple = HoareTriple::new(
            SemanticPredicate::BoundsCheck {
                var: "i".to_string(),
                min: 0,
                max: 10,
            },
            inc_stmt,
            post_bounds,
        );
        let verdict_unsafe = SemanticSolver::verify_hoare_triple(&unsafe_triple);
        match verdict_unsafe {
            ProofVerdict::ViolationFound { counterexample, .. } => {
                assert!(counterexample.contains("violates bounds"));
            }
            ProofVerdict::ProvenSafe => panic!("Expected violation on out-of-bounds increment"),
        }
    }

    #[test]
    fn test_conditional_branching_wp_calculus() {
        // Statement: if x > 0 then y := 10 else y := 20
        let if_stmt = AstStatement::IfElse {
            cond_var: "x".to_string(),
            cond_op: ">".to_string(),
            cond_val: 0,
            then_branch: Box::new(AstStatement::Assign {
                var: "y".to_string(),
                expr: AstExpression::Const(10),
            }),
            else_branch: Box::new(AstStatement::Assign {
                var: "y".to_string(),
                expr: AstExpression::Const(20),
            }),
        };

        // Postcondition: y > 0
        let post = SemanticPredicate::Gt("y".to_string(), 0);

        // Precondition: True (both branches set y > 0)
        let triple = HoareTriple::new(SemanticPredicate::True, if_stmt, post);
        let verdict = SemanticSolver::verify_hoare_triple(&triple);
        assert_eq!(verdict, ProofVerdict::ProvenSafe);
    }

    #[test]
    fn test_sub_0_1ms_formal_verification_latency_budget() {
        let stmt = AstStatement::Assign {
            var: "x".to_string(),
            expr: AstExpression::Add(
                Box::new(AstExpression::Var("x".to_string())),
                Box::new(AstExpression::Const(5)),
            ),
        };
        let post = SemanticPredicate::Gt("x".to_string(), 10);
        let pre = SemanticPredicate::Gt("x".to_string(), 6);
        let triple = HoareTriple::new(pre, stmt, post);

        let start = Instant::now();
        for _ in 0..100 {
            let verdict = SemanticSolver::verify_hoare_triple(&triple);
            assert_eq!(verdict, ProofVerdict::ProvenSafe);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 100.0;

        assert!(
            avg_us < 100.0,
            "Formal wp verification must execute in sub-0.1ms (100µs), took: {:.2}µs",
            avg_us
        );
    }
}
