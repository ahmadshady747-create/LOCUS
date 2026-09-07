//! Rule S2: Map / Set Object Key Identity Pitfall Guard (`MapObjectKeyIdentity`).
//!
//! Detects using non-primitive object or array literals as keys in `Map` or `Set` collections,
//! which causes reference identity mismatches during retrieval in pure safe Rust.

#![forbid(unsafe_code)]

use super::async_iterator::is_inside_string_literal;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::GreenTreeRoot;
use crate::semantic::traits::SemanticRule;
use crate::semantic::types::{SemanticViolation, ViolationLocation, ViolationSeverity};

/// Rule S2: Flags reference identity mismatch traps when indexing collections with object literals.
#[derive(Debug, Default, Clone, Copy)]
pub struct CollectionKeyRule;

impl SemanticRule for CollectionKeyRule {
    fn id(&self) -> &'static str {
        "S2"
    }

    fn name(&self) -> &'static str {
        "MapObjectKeyIdentity"
    }

    fn description(&self) -> &'static str {
        "Detects using inline object or array literals directly as keys in Map or Set collections. Key lookup compares reference identity (===), causing lookups on distinct literal instances to always fail."
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
                    (".set({", "map.set"),
                    (".get({", "map.get"),
                    (".has({", "map.has"),
                    (".delete({", "map.delete"),
                    ("new Set([{", "new Set"),
                ];

                for (pat, sym) in patterns {
                    if line.contains(pat) && !is_inside_string_literal(line, pat) {
                        let col_offset = line.find(pat).unwrap_or(0);
                        let byte_offset = current_offset + col_offset;
                        let location = ViolationLocation::from_offset(source, byte_offset, pat.len(), Some(sym.to_string()));

                        violations.push(SemanticViolation::new(
                            self.id(),
                            self.name(),
                            format!("Using an inline object literal as a collection key in `{}` compares reference identity (===), preventing retrieval by structural equality.", sym),
                            ViolationSeverity::High,
                            location,
                            Some("Use a unique scalar property (e.g. `item.id`) as key or serialize via `JSON.stringify(key)`.".to_string()),
                        ));
                    }
                }
            }

            current_offset += line.len() + 1;
        }

        violations
    }
}
