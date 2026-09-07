//! Dijkstra's Weakest Precondition Calculus Engine.
//!
//! Synthesizes weakest preconditions wp(S, Q) for assignments, sequences,
//! conditionals, and arithmetic assertions in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::formal::wp::types::{
    AstExpression, AstStatement, SemanticPredicate,
};

pub struct WeakestPreconditionEngine;

impl WeakestPreconditionEngine {
    /// Computes the weakest precondition wp(S, Q) for a statement S and postcondition Q.
    pub fn compute_wp(statement: &AstStatement, postcondition: &SemanticPredicate) -> SemanticPredicate {
        match statement {
            AstStatement::Skip => postcondition.clone(),

            AstStatement::Assign { var, expr } => {
                // Check if the expression contains division -> enforce divisor != 0
                let division_guard = Self::extract_division_guards(expr);
                let substituted = Self::substitute_predicate(postcondition, var, expr);
                if let Some(guard) = division_guard {
                    guard.and(substituted)
                } else {
                    substituted
                }
            }

            AstStatement::Seq(s1, s2) => {
                let wp2 = Self::compute_wp(s2, postcondition);
                Self::compute_wp(s1, &wp2)
            }

            AstStatement::IfElse {
                cond_var,
                cond_op,
                cond_val,
                then_branch,
                else_branch,
            } => {
                let cond_pred = match cond_op.as_str() {
                    ">" => SemanticPredicate::Gt(cond_var.clone(), *cond_val),
                    "<" => SemanticPredicate::Lt(cond_var.clone(), *cond_val),
                    "=" | "==" => SemanticPredicate::Eq(cond_var.clone(), *cond_val),
                    "!=" => SemanticPredicate::Not(Box::new(SemanticPredicate::Eq(
                        cond_var.clone(),
                        *cond_val,
                    ))),
                    _ => SemanticPredicate::Atom(format!("{} {} {}", cond_var, cond_op, cond_val)),
                };

                let not_cond_pred = cond_pred.clone().not();

                let wp_then = Self::compute_wp(then_branch, postcondition);
                let wp_else = Self::compute_wp(else_branch, postcondition);

                let then_impl = cond_pred.imply(wp_then);
                let else_impl = not_cond_pred.imply(wp_else);

                then_impl.and(else_impl)
            }

            AstStatement::Assert(pred) => pred.clone().and(postcondition.clone()),
        }
    }

    /// Extracts safety invariant requirements for any division operation in the expression.
    fn extract_division_guards(expr: &AstExpression) -> Option<SemanticPredicate> {
        match expr {
            AstExpression::Div(_numerator, denominator) => {
                let divisor_guard = match denominator.as_ref() {
                    AstExpression::Var(v) => SemanticPredicate::NonZero(v.clone()),
                    AstExpression::Const(c) if *c == 0 => SemanticPredicate::False,
                    AstExpression::Const(_) => SemanticPredicate::True,
                    _ => SemanticPredicate::True,
                };
                Some(divisor_guard)
            }
            AstExpression::Add(a, b)
            | AstExpression::Sub(a, b)
            | AstExpression::Mul(a, b) => {
                let g1 = Self::extract_division_guards(a);
                let g2 = Self::extract_division_guards(b);
                match (g1, g2) {
                    (Some(a), Some(b)) => Some(a.and(b)),
                    (Some(a), None) | (None, Some(a)) => Some(a),
                    (None, None) => None,
                }
            }
            _ => None,
        }
    }

    /// Syntactically substitutes `var` with `expr` within predicate `Q`: `Q[var -> expr]`.
    pub fn substitute_predicate(
        predicate: &SemanticPredicate,
        target_var: &str,
        replacement: &AstExpression,
    ) -> SemanticPredicate {
        match predicate {
            SemanticPredicate::True => SemanticPredicate::True,
            SemanticPredicate::False => SemanticPredicate::False,
            SemanticPredicate::Atom(name) => SemanticPredicate::Atom(name.clone()),

            SemanticPredicate::And(a, b) => {
                let sa = Self::substitute_predicate(a, target_var, replacement);
                let sb = Self::substitute_predicate(b, target_var, replacement);
                sa.and(sb)
            }

            SemanticPredicate::Or(a, b) => {
                let sa = Self::substitute_predicate(a, target_var, replacement);
                let sb = Self::substitute_predicate(b, target_var, replacement);
                sa.or(sb)
            }

            SemanticPredicate::Not(inner) => {
                let s = Self::substitute_predicate(inner, target_var, replacement);
                s.not()
            }

            SemanticPredicate::Imply(a, b) => {
                let sa = Self::substitute_predicate(a, target_var, replacement);
                let sb = Self::substitute_predicate(b, target_var, replacement);
                sa.imply(sb)
            }

            SemanticPredicate::NonZero(var_name) => {
                if var_name == target_var {
                    match replacement {
                        AstExpression::Const(c) => {
                            if *c != 0 {
                                SemanticPredicate::True
                            } else {
                                SemanticPredicate::False
                            }
                        }
                        AstExpression::Var(other) => SemanticPredicate::NonZero(other.clone()),
                        _ => SemanticPredicate::NonZero(format!("{:?}", replacement)),
                    }
                } else {
                    SemanticPredicate::NonZero(var_name.clone())
                }
            }

            SemanticPredicate::BoundsCheck { var, min, max } => {
                if var == target_var {
                    match replacement {
                        AstExpression::Add(a, b) => {
                            if let (AstExpression::Var(v), AstExpression::Const(k)) =
                                (a.as_ref(), b.as_ref())
                            {
                                if v == target_var {
                                    return SemanticPredicate::BoundsCheck {
                                        var: target_var.to_string(),
                                        min: min.saturating_sub(*k),
                                        max: max.saturating_sub(*k),
                                    };
                                }
                            }
                            SemanticPredicate::BoundsCheck {
                                var: format!("{:?}", replacement),
                                min: *min,
                                max: *max,
                            }
                        }
                        AstExpression::Const(c) => {
                            if *c >= *min && *c <= *max {
                                SemanticPredicate::True
                            } else {
                                SemanticPredicate::False
                            }
                        }
                        _ => SemanticPredicate::BoundsCheck {
                            var: format!("{:?}", replacement),
                            min: *min,
                            max: *max,
                        },
                    }
                } else {
                    SemanticPredicate::BoundsCheck {
                        var: var.clone(),
                        min: *min,
                        max: *max,
                    }
                }
            }

            SemanticPredicate::Gt(var, val) => {
                if var == target_var {
                    match replacement {
                        AstExpression::Const(c) => {
                            if *c > *val {
                                SemanticPredicate::True
                            } else {
                                SemanticPredicate::False
                            }
                        }
                        AstExpression::Add(a, b) => {
                            if let (AstExpression::Var(v), AstExpression::Const(k)) =
                                (a.as_ref(), b.as_ref())
                            {
                                if v == target_var {
                                    return SemanticPredicate::Gt(
                                        target_var.to_string(),
                                        val.saturating_sub(*k),
                                    );
                                }
                            }
                            SemanticPredicate::Gt(var.clone(), *val)
                        }
                        _ => SemanticPredicate::Gt(var.clone(), *val),
                    }
                } else {
                    SemanticPredicate::Gt(var.clone(), *val)
                }
            }

            SemanticPredicate::Lt(var, val) => {
                if var == target_var {
                    match replacement {
                        AstExpression::Const(c) => {
                            if *c < *val {
                                SemanticPredicate::True
                            } else {
                                SemanticPredicate::False
                            }
                        }
                        _ => SemanticPredicate::Lt(var.clone(), *val),
                    }
                } else {
                    SemanticPredicate::Lt(var.clone(), *val)
                }
            }

            SemanticPredicate::Eq(var, val) => {
                if var == target_var {
                    match replacement {
                        AstExpression::Const(c) => {
                            if *c == *val {
                                SemanticPredicate::True
                            } else {
                                SemanticPredicate::False
                            }
                        }
                        _ => SemanticPredicate::Eq(var.clone(), *val),
                    }
                } else {
                    SemanticPredicate::Eq(var.clone(), *val)
                }
            }
        }
    }
}
