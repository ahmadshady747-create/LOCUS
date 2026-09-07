//! Rule S4: Date Prototype Loss Via JSON Serialization Guard (`DatePrototypeLossViaJson`).
//!
//! Detects serialization of Date instances through JSON roundtripping without Date object
//! reconstruction, preventing runtime `TypeError` prototype loss in pure safe Rust.

#![forbid(unsafe_code)]

use super::async_iterator::is_inside_string_literal;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;
use crate::semantic::traits::SemanticRule;
use crate::semantic::types::{SemanticViolation, ViolationLocation, ViolationSeverity};

/// Rule S4: Flags Date object prototype loss caused by naive JSON serialization roundtripping.
#[derive(Debug, Default, Clone, Copy)]
pub struct DateCoercionRule;

impl SemanticRule for DateCoercionRule {
    fn id(&self) -> &'static str {
        "S4"
    }

    fn name(&self) -> &'static str {
        "DatePrototypeLossViaJson"
    }

    fn description(&self) -> &'static str {
        "Detects roundtripping Date structures via JSON.stringify/JSON.parse. Dates are coerced to ISO strings and lose prototype methods (e.g. .getTime(), .getFullYear()), causing runtime TypeErrors."
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
                    ("JSON.parse(JSON.stringify(", "JSON roundtrip clone"),
                ];

                for (pat, sym) in patterns {
                    if line.contains(pat) && !is_inside_string_literal(line, pat) {
                        let col_offset = line.find(pat).unwrap_or(0);
                        let byte_offset = current_offset + col_offset;
                        let location = ViolationLocation::from_offset(source, byte_offset, pat.len(), Some(sym.to_string()));

                        violations.push(SemanticViolation::new(
                            self.id(),
                            self.name(),
                            "Cloning objects via `JSON.parse(JSON.stringify(...))` strips Date, RegExp, and Map prototypes, converting Dates to plain strings.",
                            ViolationSeverity::High,
                            location,
                            Some("Use `structuredClone(obj)` or reconstruct Date instances using `new Date(parsed.date)`.".to_string()),
                        ));
                    }
                }
            }

            current_offset += line.len() + 1;
        }

        violations
    }
}
