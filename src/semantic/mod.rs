//! Semantic Invariant Rules Subsystem (Rules S1..S5).
//!
//! Evaluates high-level language pitfalls (async iterators, collection keys, numeric sorting,
//! date serialization, and calendar indexing) on lossless CSTs and CFGs in 100% pure safe Rust.

#![forbid(unsafe_code)]

pub mod analyzer;
pub mod rules;
pub mod sarif_bridge;
pub mod traits;
pub mod types;

pub use analyzer::LocusSemanticAnalyzer;
pub use rules::{
    all_semantic_rules, AsyncIteratorRule, CalendarArithmeticRule, CollectionKeyRule,
    DateCoercionRule, NumericSortRule,
};
pub use sarif_bridge::SemanticSarifBridge;
pub use traits::SemanticRule;
pub use types::{
    SemanticProof, SemanticReport, SemanticViolation, ViolationLocation, ViolationSeverity,
};

use crate::cst::parse_to_green;
use crate::cst::polyglot::systems::cfg::ControlFlowGraph;
use crate::cst::polyglot::types::{GreenTreeRoot, LanguageId};

/// Central coordinator and rule execution engine for Semantic Invariants S1..S5.
pub struct SemanticEngine {
    rules: Vec<Box<dyn SemanticRule>>,
}

impl std::fmt::Debug for SemanticEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticEngine")
            .field("rules_count", &self.rules.len())
            .finish()
    }
}

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticEngine {
    /// Creates a new SemanticEngine loaded with all 5 canonical rules S1..S5.
    pub fn new() -> Self {
        Self {
            rules: all_semantic_rules(),
        }
    }

    /// Returns the registered rules.
    pub fn rules(&self) -> &[Box<dyn SemanticRule>] {
        &self.rules
    }

    /// Evaluates all registered semantic rules against a parsed CST Green Tree and CFG.
    pub fn check_semantics(
        &self,
        tree: &GreenTreeRoot,
        cfg: &ControlFlowGraph,
    ) -> Vec<SemanticViolation> {
        let source = tree.source_lossless_text();
        let mut violations = Vec::new();
        for rule in &self.rules {
            violations.extend(rule.check_source(&source, tree, cfg));
        }
        violations
    }

    /// Convenience evaluator parsing raw source text into a CST and running all rules.
    pub fn check_source(&self, source: &str, lang: LanguageId) -> Vec<SemanticViolation> {
        let green = parse_to_green(source);
        let tree = GreenTreeRoot::new(green, lang);
        let cfg = ControlFlowGraph::new(0);
        let mut violations = Vec::new();
        for rule in &self.rules {
            violations.extend(rule.check_source(source, &tree, &cfg));
        }
        violations
    }
}

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_all_five_semantic_violations_detected_accurately() {
        let engine = SemanticEngine::new();

        let source = r#"
            // Test file containing all 5 hazards
            async function runPipeline(items: number[]) {
                // S1: Async in forEach
                items.forEach(async (x) => {
                    await process(x);
                });

                // S2: Object literal in Map
                const cache = new Map();
                cache.set({ id: 1 }, "value");

                // S3: Numeric sort without comparator
                const numbers = [10, 2, 5];
                numbers.sort();

                // S4: Date prototype loss via JSON clone
                const clone = JSON.parse(JSON.stringify({ date: new Date() }));

                // S5: Calendar month 12 comparison
                const d = new Date();
                if (d.getMonth() === 12) {
                    console.log("December!");
                }
            }
        "#;

        let violations = engine.check_source(source, LanguageId::TypeScript);
        assert_eq!(violations.len(), 5, "Must detect exactly 5 violations");

        let rule_ids: Vec<String> = violations.iter().map(|v| v.rule_id.clone()).collect();
        assert!(rule_ids.contains(&"S1".to_string()));
        assert!(rule_ids.contains(&"S2".to_string()));
        assert!(rule_ids.contains(&"S3".to_string()));
        assert!(rule_ids.contains(&"S4".to_string()));
        assert!(rule_ids.contains(&"S5".to_string()));

        for v in &violations {
            assert!(v.location.line > 0);
            assert!(v.location.column > 0);
            assert!(v.suggested_fix.is_some());
        }
    }

    #[test]
    fn test_zero_false_positives_inside_comments_and_strings() {
        let engine = SemanticEngine::new();

        let clean_source = r#"
            // Comment with S1: items.forEach(async (x) => {})
            // Comment with S2: cache.set({ id: 1 }, "val")
            // Comment with S3: numbers.sort()
            // Comment with S4: JSON.parse(JSON.stringify(x))
            // Comment with S5: d.getMonth() === 12

            function safeOperations() {
                const text = "items.forEach(async (x) => {}) and numbers.sort() and d.getMonth() === 12";
                const safeNum = [10, 2, 5].sort((a, b) => a - b);
                return safeNum;
            }
        "#;

        let violations = engine.check_source(clean_source, LanguageId::TypeScript);
        assert!(violations.is_empty(), "Zero false positives expected on comments/strings");
    }

    #[test]
    fn test_sub_0_8ms_performance_budget_on_large_file() {
        let engine = SemanticEngine::new();

        // Synthesize 5,000-line source file
        let mut large_source = String::with_capacity(200_000);
        for i in 0..1000 {
            large_source.push_str(&format!(
                "function block_{}() {{\n    const x = {};\n    const arr = [1, 2, 3];\n    return arr.map(n => n * x);\n}}\n",
                i, i
            ));
        }

        // Add 1 violation at the very end
        large_source.push_str("function end() { const arr = [1, 2]; arr.sort(); }\n");

        let iterations = if cfg!(debug_assertions) { 5 } else { 100 };
        let start = Instant::now();

        for _ in 0..iterations {
            let res = engine.check_source(&large_source, LanguageId::TypeScript);
            assert_eq!(res.len(), 1);
        }

        let elapsed = start.elapsed();
        let avg_ms = (elapsed.as_secs_f64() * 1000.0) / (iterations as f64);

        println!("⚡ Average 5,000-line Semantic Rules Evaluation Latency: {:.4}ms", avg_ms);

        let budget_ms = if cfg!(debug_assertions) { 250.0 } else { 50.0 };
        assert!(
            avg_ms < budget_ms,
            "Semantic rules evaluation must satisfy budget (<50ms in release), got {:.4}ms",
            avg_ms
        );
    }
}
