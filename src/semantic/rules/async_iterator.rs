//! Rule S1: Async Iterator Race Condition Guard (`AsyncIteratorRace`).
//!
//! Detects async callback functions inside un-awaited iteration methods (`forEach`, `filter`),
//! which causes concurrency race conditions or faulty truthiness filtering in pure safe Rust.

#![forbid(unsafe_code)]

use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;
use crate::semantic::traits::SemanticRule;
use crate::semantic::types::{SemanticViolation, ViolationLocation, ViolationSeverity};

/// Rule S1: Flags asynchronous iteration traps in array iteration callbacks.
#[derive(Debug, Default, Clone, Copy)]
pub struct AsyncIteratorRule;

impl SemanticRule for AsyncIteratorRule {
    fn id(&self) -> &'static str {
        "S1"
    }

    fn name(&self) -> &'static str {
        "AsyncIteratorRace"
    }

    fn description(&self) -> &'static str {
        "Detects async callbacks in forEach or filter methods. forEach does not await promises causing race conditions, while filter always treats returned Promise objects as truthy."
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
                // Check .forEach(async ... or .forEach( async ...
                if (line.contains(".forEach(async") || line.contains(".forEach( async"))
                    && !is_inside_string_literal(line, ".forEach")
                {
                    let col_offset = line.find(".forEach").unwrap_or(0);
                    let byte_offset = current_offset + col_offset;
                    let location = ViolationLocation::from_offset(source, byte_offset, 8, Some("forEach".to_string()));

                    violations.push(SemanticViolation::new(
                        self.id(),
                        self.name(),
                        "Invoking an async callback inside `.forEach()` does not await execution, causing uncontrolled concurrent execution and unhandled rejections.",
                        ViolationSeverity::High,
                        location,
                        Some("Replace with `for (const item of array) { await ... }` or `await Promise.all(array.map(async ...))`".to_string()),
                    ));
                }

                // Check .filter(async ... or .filter( async ...
                if (line.contains(".filter(async") || line.contains(".filter( async"))
                    && !is_inside_string_literal(line, ".filter")
                {
                    let col_offset = line.find(".filter").unwrap_or(0);
                    let byte_offset = current_offset + col_offset;
                    let location = ViolationLocation::from_offset(source, byte_offset, 7, Some("filter".to_string()));

                    violations.push(SemanticViolation::new(
                        self.id(),
                        self.name(),
                        "Invoking an async callback inside `.filter()` returns a Promise object which evaluates to truthy for every element, bypassing filter predicates.",
                        ViolationSeverity::Critical,
                        location,
                        Some("Map elements to booleans with `Promise.all(array.map(async ...))` then filter synchronously.".to_string()),
                    ));
                }
            }

            current_offset += line.len() + 1; // +1 for newline
        }

        violations
    }
}

/// Helper to ensure a needle is not inside an unescaped string literal on the line.
pub(crate) fn is_inside_string_literal(line: &str, needle: &str) -> bool {
    if let Some(pos) = line.find(needle) {
        let prefix = &line[..pos];
        let dquotes = prefix.matches('"').count() - prefix.matches(r#"\""#).count();
        let squotes = prefix.matches('\'').count() - prefix.matches(r#"\'"#).count();
        let bquotes = prefix.matches('`').count() - prefix.matches(r#"\`"#).count();
        dquotes % 2 != 0 || squotes % 2 != 0 || bquotes % 2 != 0
    } else {
        false
    }
}
