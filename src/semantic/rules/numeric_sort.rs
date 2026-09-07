//! Rule S3: Lexicographic Numeric Sort Hazard Guard (`NumericSortLexicographic`).
//!
//! Detects un-parameterized `.sort()` calls on numeric arrays which causes lexicographical
//! string sorting (`[10, 2]` sorting to `[10, 2]`) instead of numerical ordering in pure safe Rust.

#![forbid(unsafe_code)]

use super::async_iterator::is_inside_string_literal;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;
use crate::semantic::traits::SemanticRule;
use crate::semantic::types::{SemanticViolation, ViolationLocation, ViolationSeverity};

/// Rule S3: Flags un-parameterized `.sort()` invocations on numeric data structures.
#[derive(Debug, Default, Clone, Copy)]
pub struct NumericSortRule;

impl SemanticRule for NumericSortRule {
    fn id(&self) -> &'static str {
        "S3"
    }

    fn name(&self) -> &'static str {
        "NumericSortLexicographic"
    }

    fn description(&self) -> &'static str {
        "Detects `.sort()` invocations without a comparator callback. In JavaScript/TypeScript, default sorting converts numbers to strings, sorting [10, 2, 5] as [10, 2, 5]."
    }

    fn check_source(
        &self,
        source: &str,
        _tree: &GreenTreeRoot,
        _cfg: &ControlFlowGraph,
    ) -> Vec<SemanticViolation> {
        let mut violations = Vec::new();

        let lines: Vec<&str> = source.lines().collect();
        let mut current_offset = 0;

        for line in lines {
            let trimmed = line.trim();
            // Skip comments
            if !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*') {
                if (line.contains(".sort()") || line.contains(".sort( )"))
                    && !is_inside_string_literal(line, ".sort")
                {
                    let col_offset = line.find(".sort").unwrap_or(0);
                    let byte_offset = current_offset + col_offset;
                    let location = ViolationLocation::from_offset(source, byte_offset, 7, Some("sort()".to_string()));

                    violations.push(SemanticViolation::new(
                        self.id(),
                        self.name(),
                        "Calling `.sort()` without an explicit comparison callback performs lexicographical string sorting instead of numerical ordering.",
                        ViolationSeverity::High,
                        location,
                        Some("Provide an arithmetic comparator: `.sort((a, b) => a - b)` for ascending or `(a, b) => b - a` for descending.".to_string()),
                    ));
                }
            }

            current_offset += line.len() + 1;
        }

        violations
    }
}
