//! React & TSX/JSX Layout Invariant Solver & Render Hazard Guard.
//!
//! Enforces list keying governance (`key={...}` on `.map()`), prevents build-phase state mutations,
//! and guards against Tailwind flexbox height collapses and overflow hazards.

#![forbid(unsafe_code)]

use super::types::{LayoutViolation, LayoutViolationKind, UiInvariantReport};
use crate::cst::green::SyntaxKind;
use crate::cst::lexer::tokenize;

/// High-throughput React/TSX UI Invariant Solver.
#[derive(Debug, Default, Clone, Copy)]
pub struct TsxLayoutSolver;

impl TsxLayoutSolver {
    pub fn new() -> Self {
        Self
    }

    /// Verifies all React/TSX UI layout and lifecycle invariants against the source code.
    pub fn verify(&self, source: &str) -> UiInvariantReport {
        let start = std::time::Instant::now();
        let mut violations = Vec::new();

        self.check_missing_list_keys(source, &mut violations);
        self.check_build_phase_state_mutations(source, &mut violations);
        self.check_flex_overflow_hazards(source, &mut violations);
        self.check_negative_dimensions(source, &mut violations);

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        UiInvariantReport::with_violations(violations, elapsed)
    }

    /// Detects JSX elements mapped via `.map(...)` missing the mandatory `key={...}` prop.
    fn check_missing_list_keys(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let lines: Vec<&str> = source.lines().collect();
        let len = lines.len();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if (trimmed.contains(".map(") || trimmed.contains(".map (")) && !trimmed.starts_with("//") {
                // Scan the next 6 lines for the opening JSX element
                let end_look = (idx + 8).min(len);
                let mut found_jsx = false;
                let mut has_key = false;
                let mut jsx_line_idx = idx;

                for check_idx in idx..end_look {
                    let l = lines[check_idx].trim();
                    if l.contains('<') && !l.starts_with("</") && !l.starts_with("/*") && !l.starts_with("//") {
                        found_jsx = true;
                        jsx_line_idx = check_idx;
                    }
                    if l.contains("key=") || l.contains("key =") {
                        has_key = true;
                        break;
                    }
                    if l.contains(");") || l.contains("})") || l.contains("}))") {
                        break;
                    }
                }

                if found_jsx && !has_key {
                    violations.push(LayoutViolation {
                        kind: LayoutViolationKind::MissingListKey,
                        element_id: "MappedJsxElement".to_string(),
                        line_offset: jsx_line_idx + 1,
                        description: "JSX elements dynamically generated inside '.map()' must have a unique 'key={...}' prop on the root element to ensure deterministic React VDOM reconciliation.".to_string(),
                        suggested_patch: "Add a stable unique key attribute: e.g. <div key={item.id}> or <Component key={item.id} />.".to_string(),
                    });
                }
            }
        }
    }

    /// Detects state setters (e.g. `setCount(...)`, `dispatch(...)`) invoked directly in component body outside of hooks/callbacks.
    fn check_build_phase_state_mutations(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let tokens = tokenize(source);
        let len = tokens.len();

        let mut in_component = false;
        let mut brace_depth = 0;
        let mut in_effect_or_callback = false;
        let mut effect_brace_depth = 0;

        let mut i = 0;
        let mut line_offset = 1;

        while i < len {
            let (kind, text) = tokens[i];
            line_offset += text.matches('\n').count();

            // Detect functional component declaration: `function MyComponent` or `const MyComponent = ` or `export function MyComponent`
            if (kind == SyntaxKind::Ident && text == "function")
                || kind == SyntaxKind::ConstKw
                || kind == SyntaxKind::LetKw
                || kind == SyntaxKind::ExportKw
            {
                let mut k = i + 1;
                while k < len && tokens[k].0.is_trivia() {
                    k += 1;
                }
                if k < len && tokens[k].0 == SyntaxKind::Ident && tokens[k].1 == "function" {
                    k += 1;
                    while k < len && tokens[k].0.is_trivia() {
                        k += 1;
                    }
                }
                if k < len && tokens[k].0 == SyntaxKind::Ident {
                    let name = tokens[k].1;
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        in_component = true;
                        brace_depth = 0;
                    }
                }
            }

            if in_component {
                if kind == SyntaxKind::OpenBrace {
                    brace_depth += 1;
                } else if kind == SyntaxKind::CloseBrace {
                    if in_effect_or_callback && brace_depth == effect_brace_depth {
                        in_effect_or_callback = false;
                    }
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    if brace_depth == 0 {
                        in_component = false;
                    }
                }

                // Detect entering `useEffect`, `useCallback`, `useLayoutEffect`, or event handler
                if kind == SyntaxKind::Ident {
                    if text == "useEffect"
                        || text == "useLayoutEffect"
                        || text == "useCallback"
                        || text == "useMemo"
                        || text.starts_with("handle")
                        || text.starts_with("on")
                    {
                        in_effect_or_callback = true;
                        effect_brace_depth = brace_depth;
                    } else if (text.starts_with("set") || text == "dispatch") && text.len() > 3 {
                        // Check if followed by `(`
                        let mut next_idx = i + 1;
                        while next_idx < len && tokens[next_idx].0.is_trivia() {
                            next_idx += 1;
                        }
                        if next_idx < len && tokens[next_idx].0 == SyntaxKind::OpenParen {
                            if !in_effect_or_callback && brace_depth == 1 {
                                violations.push(LayoutViolation {
                                    kind: LayoutViolationKind::BuildPhaseStateMutation,
                                    element_id: text.to_string(),
                                    line_offset,
                                    description: format!(
                                        "Direct state mutation '{}()' in React functional component body triggers infinite re-render loops during render phase.",
                                        text
                                    ),
                                    suggested_patch: format!(
                                        "Wrap '{}()' in a 'useEffect(() => {{ ... }}, [])' hook or invoke inside an event handler callback.",
                                        text
                                    ),
                                });
                            }
                        }
                    }
                }
            }

            i += 1;
        }
    }

    /// Detects flex-col layout containers missing `min-h-0` or overflow control on scrollable children.
    fn check_flex_overflow_hazards(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        let lines: Vec<&str> = source.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.contains("flex-col") && trimmed.contains("overflow-hidden") {
                // If contains scrollable child or flex-1 without min-h-0 in subsequent lines
                if trimmed.contains("flex-1") && !trimmed.contains("min-h-0") && !trimmed.contains("overflow-y-auto") {
                    violations.push(LayoutViolation {
                        kind: LayoutViolationKind::UnboundedFlexHeight,
                        element_id: "TailwindFlexCol".to_string(),
                        line_offset: idx + 1,
                        description: "Flexbox column child with 'flex-1' in Tailwind requires 'min-h-0' to allow shrinking below its intrinsic content height without overflowing.".to_string(),
                        suggested_patch: "Add 'min-h-0' and 'overflow-y-auto' to the 'flex-1' container classes.".to_string(),
                    });
                }
            }
        }
    }

    /// Detects negative dimensions or malformed styles.
    fn check_negative_dimensions(&self, source: &str, violations: &mut Vec<LayoutViolation>) {
        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.contains("height: -") || trimmed.contains("width: -") || trimmed.contains("height: \"-")) && !trimmed.starts_with("//") {
                violations.push(LayoutViolation {
                    kind: LayoutViolationKind::NegativeConstraintOverflow,
                    element_id: "NegativeDimensionStyle".to_string(),
                    line_offset: idx + 1,
                    description: "Negative height or width dimensions produce invalid CSS bounding boxes.".to_string(),
                    suggested_patch: "Use positive pixel/rem values or transform: translateY(...) for offset positioning.".to_string(),
                });
            }
        }
    }
}
