//! Semantic Logic Solver & Hoare Triple Invariant Prover.
//!
//! Evaluates implication satisfiability (P ==> wp(S, Q)), detects logical contradictions,
//! and synthesizes concrete counterexamples in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::formal::wp::calculus::WeakestPreconditionEngine;
use crate::formal::wp::types::{
    HoareTriple, ProofVerdict, SemanticPredicate,
};

pub struct SemanticSolver;

impl SemanticSolver {
    /// Simplifies and normalizes a semantic predicate formula using boolean laws.
    pub fn simplify(predicate: &SemanticPredicate) -> SemanticPredicate {
        match predicate {
            SemanticPredicate::True => SemanticPredicate::True,
            SemanticPredicate::False => SemanticPredicate::False,
            SemanticPredicate::Atom(s) => SemanticPredicate::Atom(s.clone()),
            SemanticPredicate::BoundsCheck { var, min, max } => {
                if min > max {
                    SemanticPredicate::False
                } else {
                    SemanticPredicate::BoundsCheck {
                        var: var.clone(),
                        min: *min,
                        max: *max,
                    }
                }
            }
            SemanticPredicate::NonZero(v) => SemanticPredicate::NonZero(v.clone()),
            SemanticPredicate::Gt(v, val) => SemanticPredicate::Gt(v.clone(), *val),
            SemanticPredicate::Lt(v, val) => SemanticPredicate::Lt(v.clone(), *val),
            SemanticPredicate::Eq(v, val) => SemanticPredicate::Eq(v.clone(), *val),

            SemanticPredicate::Not(inner) => {
                let s = Self::simplify(inner);
                match s {
                    SemanticPredicate::True => SemanticPredicate::False,
                    SemanticPredicate::False => SemanticPredicate::True,
                    SemanticPredicate::Not(nested) => *nested,
                    other => SemanticPredicate::Not(Box::new(other)),
                }
            }

            SemanticPredicate::And(a, b) => {
                let sa = Self::simplify(a);
                let sb = Self::simplify(b);
                match (sa, sb) {
                    (SemanticPredicate::False, _) | (_, SemanticPredicate::False) => {
                        SemanticPredicate::False
                    }
                    (SemanticPredicate::True, p) | (p, SemanticPredicate::True) => p,
                    (p1, p2) if p1 == p2 => p1,
                    (p1, p2) => SemanticPredicate::And(Box::new(p1), Box::new(p2)),
                }
            }

            SemanticPredicate::Or(a, b) => {
                let sa = Self::simplify(a);
                let sb = Self::simplify(b);
                match (sa, sb) {
                    (SemanticPredicate::True, _) | (_, SemanticPredicate::True) => {
                        SemanticPredicate::True
                    }
                    (SemanticPredicate::False, p) | (p, SemanticPredicate::False) => p,
                    (p1, p2) if p1 == p2 => p1,
                    (p1, p2) => SemanticPredicate::Or(Box::new(p1), Box::new(p2)),
                }
            }

            SemanticPredicate::Imply(a, b) => {
                let sa = Self::simplify(a);
                let sb = Self::simplify(b);
                match (sa, sb) {
                    (SemanticPredicate::False, _) | (_, SemanticPredicate::True) => {
                        SemanticPredicate::True
                    }
                    (SemanticPredicate::True, p) => p,
                    (p1, p2) if p1 == p2 => SemanticPredicate::True,
                    (p1, p2) => SemanticPredicate::Imply(Box::new(p1), Box::new(p2)),
                }
            }
        }
    }

    /// Formally proves whether a Hoare Triple `{P} S {Q}` is mathematically valid.
    pub fn verify_hoare_triple(triple: &HoareTriple) -> ProofVerdict {
        let wp = WeakestPreconditionEngine::compute_wp(&triple.statement, &triple.postcondition);
        let simplified_wp = Self::simplify(&wp);

        // If weakest precondition simplifies to True, S unconditionally satisfies Q
        if simplified_wp == SemanticPredicate::True {
            return ProofVerdict::ProvenSafe;
        }

        // Test if Precondition P logically entails WP: P ==> WP
        if Self::proves_implication(&triple.precondition, &simplified_wp) {
            ProofVerdict::ProvenSafe
        } else {
            // Synthesize concrete counterexample
            let counterexample = Self::synthesize_counterexample(&triple.precondition, &simplified_wp);
            ProofVerdict::ViolationFound {
                counterexample,
                failed_predicate: simplified_wp,
            }
        }
    }

    /// Checks whether hypothesis `p` logically implies conclusion `q`.
    pub fn proves_implication(p: &SemanticPredicate, q: &SemanticPredicate) -> bool {
        let p_simp = Self::simplify(p);
        let q_simp = Self::simplify(q);

        if p_simp == q_simp || q_simp == SemanticPredicate::True || p_simp == SemanticPredicate::False {
            return true;
        }

        match (p_simp, q_simp) {
            // NonZero checks: Gt(x, 0) ==> NonZero(x)
            (SemanticPredicate::Gt(v1, val), SemanticPredicate::NonZero(v2))
                if v1 == v2 && val >= 0 =>
            {
                true
            }

            // Gt implication: Gt(x, 6) ==> Gt(x, 5)
            (SemanticPredicate::Gt(v1, val1), SemanticPredicate::Gt(v2, val2))
                if v1 == v2 && val1 >= val2 =>
            {
                true
            }

            // Lt implication: Lt(x, 4) ==> Lt(x, 5)
            (SemanticPredicate::Lt(v1, val1), SemanticPredicate::Lt(v2, val2))
                if v1 == v2 && val1 <= val2 =>
            {
                true
            }

            // BoundsCheck containment: [p_min, p_max] subset of [q_min, q_max]
            (
                SemanticPredicate::BoundsCheck {
                    var: v1,
                    min: min1,
                    max: max1,
                },
                SemanticPredicate::BoundsCheck {
                    var: v2,
                    min: min2,
                    max: max2,
                },
            ) if v1 == v2 && min1 >= min2 && max1 <= max2 => true,

            // Conjunctions in hypothesis: (P1 /\ P2) ==> Q
            (SemanticPredicate::And(p1, p2), q_target) => {
                Self::proves_implication(&p1, &q_target) || Self::proves_implication(&p2, &q_target)
            }

            // Conjunctions in conclusion: P ==> (Q1 /\ Q2)
            (p_source, SemanticPredicate::And(q1, q2)) => {
                Self::proves_implication(&p_source, &q1) && Self::proves_implication(&p_source, &q2)
            }

            _ => false,
        }
    }

    /// Synthesizes a concrete counterexample when P does not entail WP.
    fn synthesize_counterexample(
        precondition: &SemanticPredicate,
        wp: &SemanticPredicate,
    ) -> String {
        match wp {
            SemanticPredicate::NonZero(v) => format!(
                "Counterexample: Variable '{}' = 0 violates non-zero division safety (precondition was '{}')",
                v, precondition
            ),
            SemanticPredicate::BoundsCheck { var, min, max } => format!(
                "Counterexample: Index variable '{}' violates bounds [{}..{}] (precondition was '{}')",
                var, min, max, precondition
            ),
            SemanticPredicate::And(a, b) => {
                if !Self::proves_implication(precondition, a) {
                    Self::synthesize_counterexample(precondition, a)
                } else {
                    Self::synthesize_counterexample(precondition, b)
                }
            }
            SemanticPredicate::False => format!(
                "Counterexample: Statement leads unconditionally to failure state (precondition was '{}')",
                precondition
            ),
            other => format!(
                "Counterexample: Precondition '{}' fails to establish required safety invariant '{}'",
                precondition, other
            ),
        }
    }
}
