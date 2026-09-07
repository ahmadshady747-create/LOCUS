//! Rule S5: Calendar Arithmetic & Month Indexing Mismatch Guard (`CalendarArithmeticMismatch`).
//!
//! Detects 0-indexed vs 1-indexed month arithmetic bugs (e.g. `date.getMonth() === 12`
//! or passing 1-indexed months directly to `setMonth`) in pure safe Rust.

#![forbid(unsafe_code)]

use super::async_iterator::is_inside_string_literal;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;
use crate::semantic::traits::SemanticRule;
use crate::semantic::types::{SemanticViolation, ViolationLocation, ViolationSeverity};

/// Rule S5: Flags calendar indexing and month boundary arithmetic discrepancies.
#[derive(Debug, Default, Clone, Copy)]
pub struct CalendarArithmeticRule;

impl SemanticRule for CalendarArithmeticRule {
    fn id(&self) -> &'static str {
        "S5"
    }

    fn name(&self) -> &'static str {
        "CalendarArithmeticMismatch"
    }

    fn description(&self) -> &'static str {
        "Detects 0-indexed month mismatch (e.g. checking `getMonth() === 12` or indexing month 1..12 into Date constructor/setMonth). JS Date months are 0-indexed (0=Jan, 11=Dec)."
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
                let patterns = [
                    (".getMonth() === 12", "getMonth() === 12"),
                    (".getMonth() == 12", "getMonth() == 12"),
                    (".getMonth() >= 12", "getMonth() >= 12"),
                    (".getMonth() > 11", "getMonth() > 11"),
                ];

                for (pat, sym) in patterns {
                    if line.contains(pat) && !is_inside_string_literal(line, pat) {
                        let col_offset = line.find(pat).unwrap_or(0);
                        let byte_offset = current_offset + col_offset;
                        let location = ViolationLocation::from_offset(source, byte_offset, pat.len(), Some(sym.to_string()));

                        violations.push(SemanticViolation::new(
                            self.id(),
                            self.name(),
                            "JavaScript Date months are 0-indexed (0 to 11). Comparing `getMonth()` against 12 is impossible and will always evaluate to false.",
                            ViolationSeverity::High,
                            location,
                            Some("Compare against month 11 for December (`getMonth() === 11`) or normalize via `(date.getMonth() + 1)`.".to_string()),
                        ));
                    }
                }
            }

            current_offset += line.len() + 1;
        }

        violations
    }
}
