//! Flutter Widget Tree Layout Invariant & RenderFlex Overflow Solver.
//!
//! Enforces mathematical BoxConstraints, identifies RenderFlex overflow hazards (e.g. unconstrained
//! `ListView` inside `Column`), prevents build-phase `setState()` mutations, and validates list keying.

#![forbid(unsafe_code)]

use super::types::{LayoutViolation, LayoutViolationKind, UiInvariantReport};
use crate::cst::green::SyntaxKind;
use crate::cst::lexer::tokenize;

/// High-throughput Flutter UI Invariant & Layout Constraint Solver.
#[derive(Debug, Default, Clone, Copy)]
pub struct FlutterLayoutSolver;

impl FlutterLayoutSolver {
    pub fn new() -> Self {
        Self
    }

    /// Verifies all Flutter UI invariants against the source code.
    pub fn verify(&self, source: &str) -> UiInvariantReport {
        let start = std::time::Instant::now();
        let mut violations = Vec::new();

        self.check_renderflex_overflow(source, &mut violations);
        self.check_nested_scrollables(source, &mut violations);
        self.check_build_phase_state_mutations(source, &mut violations);
        self.check_missing_list_keys(source, &mut violations);
        self.check_negative_constraints(source, &mut violations);

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        UiInvariantReport::with_violations(violations, elapsed)
    }

    /// Checks for unbounded widgets (e.g. `ListView`, `GridView`) directly placed in `Column`/`Row` without `Expanded`/`Flexible`.
    fn check_renderflex_overflow(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let lines: Vec<&str> = source.lines().collect();
        let _len = lines.len();

        let mut flex_stack: Vec<(usize, &str)> = Vec::new(); // (line_idx, flex_type: "Column" | "Row" | "Flex")
        let unbounded_widgets = [
            "ListView",
            "ListView.builder",
            "GridView",
            "GridView.builder",
            "CustomScrollView",
        ];

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("Column(") || trimmed.contains(" Column(") || trimmed.contains(" Column (") {
                flex_stack.push((idx, "Column"));
            } else if trimmed.starts_with("Row(") || trimmed.contains(" Row(") || trimmed.contains(" Row (") {
                flex_stack.push((idx, "Row"));
            } else if trimmed.starts_with("Flex(") || trimmed.contains(" Flex(") {
                flex_stack.push((idx, "Flex"));
            }

            if !flex_stack.is_empty() {
                for &unbounded in &unbounded_widgets {
                    if trimmed.contains(&format!("{}(", unbounded)) || trimmed.contains(&format!("{} (", unbounded)) {
                        // Check if immediately wrapped by Expanded or Flexible or SizedBox in preceding 3 lines
                        let mut is_wrapped = false;
                        let start_check = idx.saturating_sub(3);
                        for check_idx in start_check..=idx {
                            let l = lines[check_idx].trim();
                            if l.contains("Expanded(")
                                || l.contains("Flexible(")
                                || l.contains("SizedBox(")
                                || l.contains("SingleChildScrollView(")
                                || l.contains("shrinkWrap: true")
                            {
                                is_wrapped = true;
                                break;
                            }
                        }

                        if !is_wrapped {
                            let (_, flex_name) = flex_stack.last().copied().unwrap_or((0, "Column"));
                            violations.push(LayoutViolation {
                                kind: LayoutViolationKind::RenderFlexOverflowUnboundedChild,
                                element_id: unbounded.to_string(),
                                line_offset: idx + 1,
                                description: format!(
                                    "Unbounded widget '{}' placed directly inside '{}' will cause RenderFlex unbounded vertical/horizontal constraint overflow.",
                                    unbounded, flex_name
                                ),
                                suggested_patch: format!(
                                    "Wrap '{}' with 'Expanded(child: ...)' or provide explicit constraints with 'SizedBox(height: ...)' or 'shrinkWrap: true'.",
                                    unbounded
                                ),
                            });
                        }
                    }
                }
            }

            // Simple stack unwinding on closing parenthesis
            if (trimmed.ends_with("),") || trimmed.ends_with(");") || trimmed.ends_with(')')) && !flex_stack.is_empty() {
                if trimmed.contains("children:") || trimmed.starts_with(']') || trimmed.starts_with('}') {
                    flex_stack.pop();
                }
            }
        }
    }

    /// Checks for nested scrollables in the same axis missing `shrinkWrap: true` or `NeverScrollableScrollPhysics`.
    fn check_nested_scrollables(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let lines: Vec<&str> = source.lines().collect();
        let len = lines.len();

        let mut scrollable_depth = 0;
        let mut parent_scroll_line = 0;

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let is_scrollable = trimmed.contains("ListView")
                || trimmed.contains("GridView")
                || trimmed.contains("SingleChildScrollView")
                || trimmed.contains("CustomScrollView");

            if is_scrollable {
                scrollable_depth += 1;
                if scrollable_depth == 1 {
                    parent_scroll_line = idx;
                } else if scrollable_depth > 1 {
                    // Check if inner scrollable has shrinkWrap or NeverScrollableScrollPhysics
                    let mut has_shrinkwrap = false;
                    let mut has_never_scroll = false;
                    let end_look = (idx + 8).min(len);
                    for check_idx in idx..end_look {
                        let l = lines[check_idx];
                        if l.contains("shrinkWrap: true") {
                            has_shrinkwrap = true;
                        }
                        if l.contains("NeverScrollableScrollPhysics") {
                            has_never_scroll = true;
                        }
                    }

                    if !has_shrinkwrap || !has_never_scroll {
                        violations.push(LayoutViolation {
                            kind: LayoutViolationKind::NestedScrollableSameAxis,
                            element_id: "NestedScrollable".to_string(),
                            line_offset: idx + 1,
                            description: format!(
                                "Nested scrollable at line {} inside parent scrollable (line {}) lacks 'shrinkWrap: true' and 'NeverScrollableScrollPhysics()', causing scroll contention and layout crash.",
                                idx + 1, parent_scroll_line + 1
                            ),
                            suggested_patch: "Set 'shrinkWrap: true' and 'physics: const NeverScrollableScrollPhysics()' on the inner scrollable widget.".to_string(),
                        });
                    }
                }
            }

            if (trimmed.ends_with("),") || trimmed.ends_with(");")) && scrollable_depth > 0 {
                scrollable_depth -= 1;
            }
        }
    }

    /// Detects state mutations (e.g. `setState()`) invoked directly inside the `build()` method pass.
    fn check_build_phase_state_mutations(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let tokens = tokenize(source);
        let len = tokens.len();

        let mut in_build_method = false;
        let mut build_brace_depth = 0;

        let mut i = 0;
        let mut line_offset = 1;

        while i < len {
            let (kind, text) = tokens[i];
            line_offset += text.matches('\n').count();

            // Detect `Widget build(BuildContext context)`
            if kind == SyntaxKind::Ident && text == "build" && i + 1 < len {
                let mut scan = i + 1;
                while scan < len && tokens[scan].0.is_trivia() {
                    scan += 1;
                }
                if scan < len && tokens[scan].0 == SyntaxKind::OpenParen {
                    in_build_method = true;
                    build_brace_depth = 0;
                }
            }

            if in_build_method {
                if kind == SyntaxKind::OpenBrace {
                    build_brace_depth += 1;
                } else if kind == SyntaxKind::CloseBrace {
                    if build_brace_depth > 0 {
                        build_brace_depth -= 1;
                    }
                    if build_brace_depth == 0 {
                        in_build_method = false;
                    }
                } else if kind == SyntaxKind::Ident && text == "setState" {
                    // Check if called inside an inline handler like `onPressed: () { setState(...) }`
                    let mut in_handler = false;
                    let lookback = i.saturating_sub(15);
                    for k in lookback..i {
                        let prev_text = tokens[k].1;
                        if prev_text.starts_with("on") || prev_text == "onPressed" || prev_text == "onTap" || prev_text == "onChanged" {
                            in_handler = true;
                            break;
                        }
                    }

                    if !in_handler {
                        violations.push(LayoutViolation {
                            kind: LayoutViolationKind::BuildPhaseStateMutation,
                            element_id: "setState".to_string(),
                            line_offset,
                            description: "Direct invocation of 'setState()' during Flutter widget build pass causes infinite rebuild loops and state corruption.".to_string(),
                            suggested_patch: "Move 'setState()' into an event handler callback (e.g. onPressed) or lifecycle hook like 'initState()'.".to_string(),
                        });
                    }
                }
            }

            i += 1;
        }
    }

    /// Checks for missing Keys in `.map(...)` or `ListView.builder`.
    fn check_missing_list_keys(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let lines: Vec<&str> = source.lines().collect();
        let len = lines.len();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains(".map((e) =>") || trimmed.contains(".map((item) =>") || trimmed.contains(".map(") {
                // Check if the returned widget has a key: property
                let end_look = (idx + 5).min(len);
                let mut has_key = false;
                for check_idx in idx..end_look {
                    if lines[check_idx].contains("key:") || lines[check_idx].contains("Key(") {
                        has_key = true;
                        break;
                    }
                }

                if !has_key {
                    violations.push(LayoutViolation {
                        kind: LayoutViolationKind::MissingListKey,
                        element_id: "ListIteration".to_string(),
                        line_offset: idx + 1,
                        description: "List elements generated dynamically via .map() must provide an explicit 'key: ValueKey(...)' to ensure stable element reconciliation.".to_string(),
                        suggested_patch: "Add 'key: ValueKey(item.id)' to the root returned widget in .map().".to_string(),
                    });
                }
            }
        }
    }

    /// Checks for negative padding/margins.
    fn check_negative_constraints(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("EdgeInsets") && (trimmed.contains("(-") || trimmed.contains(", -") || trimmed.contains(": -")) {
                violations.push(LayoutViolation {
                    kind: LayoutViolationKind::NegativeConstraintOverflow,
                    element_id: "NegativeEdgeInsets".to_string(),
                    line_offset: idx + 1,
                    description: "Negative padding or margin values break BoxConstraints geometry calculations.".to_string(),
                    suggested_patch: "Use positive EdgeInsets with Transform.translate() if negative visual offset is required.".to_string(),
                });
            }
        }
    }
}
