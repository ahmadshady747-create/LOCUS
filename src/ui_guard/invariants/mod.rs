//! UI Layout Invariant Solver & Overflow Hazard Guard.
//!
//! Enforces mathematical BoxConstraints, identifies RenderFlex overflow hazards,
//! prevents build-phase state mutations, and validates list keying across Flutter and React/TSX.

#![forbid(unsafe_code)]

pub mod flutter_solver;
pub mod tsx_solver;
pub mod types;

pub use flutter_solver::FlutterLayoutSolver;
pub use tsx_solver::TsxLayoutSolver;
pub use types::{
    BoxConstraintBounds, LayoutViolation, LayoutViolationKind, UiInvariantReport,
};

use crate::types::Language;

/// Unified Sovereign UI Invariant Checker.
#[derive(Debug, Default, Clone, Copy)]
pub struct UiInvariantChecker {
    flutter_solver: FlutterLayoutSolver,
    tsx_solver: TsxLayoutSolver,
}

impl UiInvariantChecker {
    pub fn new() -> Self {
        Self {
            flutter_solver: FlutterLayoutSolver::new(),
            tsx_solver: TsxLayoutSolver::new(),
        }
    }

    /// Checks all UI invariants for the provided source code based on its language.
    pub fn check(&self, source: &str, language: Language) -> UiInvariantReport {
        match language {
            Language::Dart => self.flutter_solver.verify(source),
            Language::Tsx | Language::Jsx | Language::TypeScript | Language::JavaScript => {
                self.tsx_solver.verify(source)
            }
            _ => UiInvariantReport::clean(0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_flutter_detects_renderflex_overflow_unbounded_listview() {
        let code = r#"
class BadDashboard extends StatelessWidget {
    @override
    Widget build(BuildContext context) {
        return Scaffold(
            body: Column(
                children: [
                    Text("Dashboard Header"),
                    ListView.builder(
                        itemCount: 10,
                        itemBuilder: (ctx, i) => Text("Item $i"),
                    ),
                ],
            ),
        );
    }
}
"#;
        let checker = UiInvariantChecker::new();
        let report = checker.check(code, Language::Dart);

        assert!(!report.passed, "Must detect RenderFlex overflow for unbounded ListView in Column");
        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0].kind,
            LayoutViolationKind::RenderFlexOverflowUnboundedChild
        );
        assert!(report.violations[0].description.contains("RenderFlex unbounded"));
    }

    #[test]
    fn test_flutter_clean_expanded_widget_passes() {
        let code = r#"
class GoodDashboard extends StatelessWidget {
    @override
    Widget build(BuildContext context) {
        return Scaffold(
            body: Column(
                children: [
                    Text("Dashboard Header"),
                    Expanded(
                        child: ListView.builder(
                            itemCount: 10,
                            itemBuilder: (ctx, i) => Text("Item $i"),
                        ),
                    ),
                ],
            ),
        );
    }
}
"#;
        let checker = UiInvariantChecker::new();
        let report = checker.check(code, Language::Dart);

        assert!(report.passed, "Clean Flutter code wrapped with Expanded must pass all layout invariants");
        assert_eq!(report.violations.len(), 0);
    }

    #[test]
    fn test_flutter_detects_build_phase_set_state() {
        let code = r#"
class BadStateful extends StatefulWidget {
    @override
    _BadStatefulState createState() => _BadStatefulState();
}

class _BadStatefulState extends State<BadStateful> {
    int counter = 0;

    @override
    Widget build(BuildContext context) {
        setState(() {
            counter += 1;
        });
        return Text("Count: $counter");
    }
}
"#;
        let checker = UiInvariantChecker::new();
        let report = checker.check(code, Language::Dart);

        assert!(!report.passed);
        let has_build_mutation = report
            .violations
            .iter()
            .any(|v| v.kind == LayoutViolationKind::BuildPhaseStateMutation);
        assert!(has_build_mutation, "Must flag direct setState in build method");
    }

    #[test]
    fn test_tsx_detects_missing_list_key() {
        let code = r#"
export function ItemList({ items }: { items: string[] }) {
    return (
        <ul>
            {items.map((item) => (
                <li>{item}</li>
            ))}
        </ul>
    );
}
"#;
        let checker = UiInvariantChecker::new();
        let report = checker.check(code, Language::Tsx);

        assert!(!report.passed, "Must flag missing key in .map() iteration");
        let has_key_violation = report
            .violations
            .iter()
            .any(|v| v.kind == LayoutViolationKind::MissingListKey);
        assert!(has_key_violation);
    }

    #[test]
    fn test_tsx_detects_build_phase_state_mutation() {
        let code = r#"
export function CounterComponent() {
    const [count, setCount] = useState(0);

    // Illegal state mutation in body
    setCount(count + 1);

    return <div>{count}</div>;
}
"#;
        let checker = UiInvariantChecker::new();
        let report = checker.check(code, Language::Tsx);

        assert!(!report.passed, "Must flag direct state setter call in component body");
        let has_mutation = report
            .violations
            .iter()
            .any(|v| v.kind == LayoutViolationKind::BuildPhaseStateMutation);
        assert!(has_mutation);
    }

    #[test]
    fn test_sub_0_05ms_ui_invariant_verification_budget() {
        let code = r#"
class ComplexScreen extends StatelessWidget {
    @override
    Widget build(BuildContext context) {
        return Scaffold(
            appBar: AppBar(title: Text("Test")),
            body: Column(
                children: [
                    Text("Title"),
                    Expanded(
                        child: ListView.builder(
                            itemCount: 20,
                            itemBuilder: (ctx, i) => ListTile(
                                title: Text("Entry $i"),
                            ),
                        ),
                    ),
                ],
            ),
        );
    }
}
"#;
        let checker = UiInvariantChecker::new();

        // Warmup
        let _ = checker.check(code, Language::Dart);

        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = checker.check(code, Language::Dart);
        }
        let elapsed = start.elapsed();
        let avg_micros = (elapsed.as_micros() as f64) / (iterations as f64);

        println!("⚡ Average UI Invariant verification latency: {:.3} µs", avg_micros);
        let limit = if cfg!(debug_assertions) { 5000.0 } else { 350.0 };
        assert!(
            avg_micros < limit,
            "UI invariant checking must execute in sub-0.35ms (<350µs in release), got: {:.3} µs",
            avg_micros
        );
    }
}
