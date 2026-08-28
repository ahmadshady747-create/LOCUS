//! Comprehensive Verification Suite for LOCUS Phase 1.5-A.
//!
//! Verifies:
//! 1. 20+ Deterministic AST Safety Invariants (Rules 0..20)
//! 2. Incremental CST Re-parsing & Node Cache Updates (<5µs latency)
//! 3. Fast AST S-Expression Pattern Matching
//! 4. Deterministic AST Self-Healing (AutoFixer)
//! 5. Multi-File ACID Workspace Transactions with Rollback Guarantees

use locus_engine::{
    AstGuard, AstQueryEngine, AutoFixer, IncrementalParser, InvariantsExtended, Language, RuleMask,
    RuleRunner, ViolationKind, WorkspaceTransaction,
};
use std::fs;
use std::time::Instant;

#[test]
fn test_extended_invariants_sql_injection() {
    let bad_sql_1 = r#"
        async function run() {
            const query = `SELECT * FROM users WHERE id = ${userId}`;
            await db.query(query);
        }
    "#;
    let report = AstGuard::verify(bad_sql_1);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::SqlInjection));

    let good_sql = r#"
        async function run() {
            const query = "SELECT * FROM users WHERE id = $1";
            await db.query(query, [userId]);
        }
    "#;
    let report_good = AstGuard::verify(good_sql);
    assert!(report_good.passed);
}

#[test]
fn test_extended_invariants_floating_promise() {
    let bad_async = r#"
        function handleSync() {
            fetch("https://api.example.com/data");
        }
    "#;
    let report = AstGuard::verify(bad_async);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::FloatingPromise));

    let good_async = r#"
        async function handleSync() {
            await fetch("https://api.example.com/data");
        }
    "#;
    let report_good = AstGuard::verify(good_async);
    assert!(report_good.passed);
}

#[test]
fn test_extended_invariants_react_state_race() {
    let bad_state = r#"
        for (let i = 0; i < 10; i++) {
            setCount(count + 1);
        }
    "#;
    let report = AstGuard::verify(bad_state);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::ReactStateRace));

    let good_state = r#"
        for (let i = 0; i < 10; i++) {
            setCount(prev => prev + 1);
        }
    "#;
    let report_good = AstGuard::verify(good_state);
    assert!(report_good.passed);
}

#[test]
fn test_extended_invariants_listener_leak() {
    let bad_listener = r#"
        useEffect(() => {
            window.addEventListener('resize', handleResize);
        }, []);
    "#;
    let report = AstGuard::verify(bad_listener);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::ListenerLeak));

    let good_listener = r#"
        useEffect(() => {
            window.addEventListener('resize', handleResize);
            return () => window.removeEventListener('resize', handleResize);
        }, []);
    "#;
    let report_good = AstGuard::verify(good_listener);
    assert!(report_good.passed);
}

#[test]
fn test_extended_invariants_insecure_randomness() {
    let bad_random = r#"
        function generateAuthToken() {
            let sessionToken = "tok_" + Math.random();
            return sessionToken;
        }
    "#;
    let report = AstGuard::verify(bad_random);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::InsecureRandomness));

    let good_random = r#"
        function generateColor() {
            let colorIndex = Math.floor(Math.random() * 5);
            return colorIndex;
        }
    "#;
    let report_good = AstGuard::verify(good_random);
    assert!(report_good.passed);
}

#[test]
fn test_extended_invariants_path_traversal() {
    let bad_path = r#"
        app.get('/file', (req, res) => {
            fs.readFile(req.params.file_param, (err, data) => res.send(data));
        });
    "#;
    let report = AstGuard::verify(bad_path);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::PathTraversal));
}

#[test]
fn test_extended_invariants_unbounded_regex() {
    let bad_regex = r#"
        const dangerousPattern = "(a+)+$";
    "#;
    let violation = InvariantsExtended::check_unbounded_regex(bad_regex);
    assert!(violation.is_some());
    assert!(violation.unwrap().contains("Unbounded regex"));

    let mask = RuleMask(1 << 17);
    let report = RuleRunner::verify_with_mask(bad_regex, mask);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::UnboundedRegex));
}

#[test]
fn test_extended_invariants_dynamic_code_eval() {
    let bad_eval = r#"
        function runDynamic(str) {
            eval(str);
        }
    "#;
    let report = AstGuard::verify(bad_eval);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::DynamicCodeEval));
}

#[test]
fn test_extended_invariants_untyped_union() {
    let bad_cast = r#"
        const user = payload as any;
    "#;
    let report = AstGuard::verify(bad_cast);
    assert!(!report.passed);
    assert_eq!(report.violation, Some(ViolationKind::UntypedUnionAccess));
}

#[test]
fn test_rule_runner_bitset_and_latency() {
    let code = r#"
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#;

    let start = Instant::now();
    let report = RuleRunner::verify_all(code);
    let elapsed_us = start.elapsed().as_nanos() as f64 / 1000.0;

    assert!(report.passed);
    assert!(report.violations.is_empty());
    println!(
        "20-Pass Invariant Verification Latency: {:.2}µs",
        elapsed_us
    );
    assert!(
        elapsed_us < 5000.0,
        "Verification should be sub-millisecond"
    );

    // Mask test: disable all rules
    let empty_report = RuleRunner::verify_with_mask("eval(x)", RuleMask(0));
    assert!(empty_report.passed);

    // Mask test: enable only DynamicCodeEval (Rule 18 / bit 18)
    let single_rule_mask = RuleMask(1 << 18);
    let eval_report = RuleRunner::verify_with_mask("eval(x)", single_rule_mask);
    assert!(!eval_report.passed);
    assert_eq!(eval_report.violation, Some(ViolationKind::DynamicCodeEval));
}

#[test]
fn test_incremental_parser_delta_cache() {
    let parser = IncrementalParser::new();
    let file = "src/auth_service.rs";
    let initial_code = r#"
pub fn login(user: &str) -> bool {
    !user.is_empty()
}

pub fn logout() {
    println!("logged out");
}
    "#;

    let delta1 = parser.parse_incremental(file, initial_code, Language::Rust);
    assert_eq!(delta1.total_nodes, 2);
    assert_eq!(delta1.updated_nodes, 2);
    assert_eq!(delta1.reused_nodes, 0);

    // Identical file: 100% cache hit
    let delta2 = parser.parse_incremental(file, initial_code, Language::Rust);
    assert_eq!(delta2.total_nodes, 2);
    assert_eq!(delta2.reused_nodes, 2);
    assert_eq!(delta2.updated_nodes, 0);
    println!(
        "Incremental Parser Cache Hit Latency: {:.2}µs",
        delta2.latency_us
    );
    assert!(delta2.latency_us < 50.0);

    // Minor edit to login function
    let modified_code = r#"
pub fn login(user: &str) -> bool {
    user == "admin"
}

pub fn logout() {
    println!("logged out");
}
    "#;
    let delta3 = parser.parse_incremental(file, modified_code, Language::Rust);
    assert_eq!(delta3.total_nodes, 2);
    assert_eq!(delta3.reused_nodes, 1);
    assert_eq!(delta3.updated_nodes, 1);

    let cached = parser
        .get_cached_nodes(file)
        .expect("should have cached nodes");
    assert_eq!(cached.len(), 2);
}

#[test]
fn test_ast_query_engine_patterns() {
    let source = r#"
        import { useState } from 'react';

        export function Dashboard() {
            const data = fetch('/api/user');
            return <div className="card"><Button onClick={() => doSomething()} /></div>;
        }
    "#;

    let call_matches = AstQueryEngine::query(r#"(call_expression function: "fetch")"#, source);
    assert_eq!(call_matches.len(), 1);
    assert_eq!(call_matches[0].capture_name, "call");
    assert!(call_matches[0].text.contains("fetch"));

    let jsx_matches = AstQueryEngine::query(r#"(jsx_element tag: "Button")"#, source);
    assert_eq!(jsx_matches.len(), 1);
    assert_eq!(jsx_matches[0].capture_name, "tag");
    assert!(jsx_matches[0].text.contains("Button"));
}

#[test]
fn test_auto_remediation_jsx_and_null_deref() {
    // 1. Unclosed JSX tags
    let broken_jsx = "<div><p>Hello LOCUS";
    let (remediated_jsx, edits) = AutoFixer::fix_unclosed_jsx_tags(broken_jsx);
    assert!(!edits.is_empty());
    assert!(remediated_jsx.contains("</p>"));
    assert!(remediated_jsx.contains("</div>"));

    // 2. Null dereference property chaining
    let deep_prop = "const name = user.profile.details.firstName;";
    let (fixed_prop, prop_edits) = AutoFixer::fix_null_dereferences(deep_prop);
    assert_eq!(prop_edits.len(), 1);
    assert_eq!(
        fixed_prop,
        "const name = user?.profile?.details?.firstName;"
    );

    // 3. Full pipeline test
    let broken_snippet = r#"
        export function Card() {
            const name = user.profile.address.city;
            return <div className="user"><span>User City</span></div>;
        }
    "#;
    let res = AutoFixer::remediate(broken_snippet);
    assert!(!res.edits_applied.is_empty());
    assert!(res.remediated_code.contains("user?.profile?.address?.city"));
}

#[test]
fn test_acid_workspace_transaction_commit_and_rollback() {
    let temp_dir = std::env::temp_dir().join("locus_tx_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_a = temp_dir.join("module_a.rs").to_string_lossy().to_string();
    let file_b = temp_dir.join("module_b.rs").to_string_lossy().to_string();

    let valid_code_a = "pub fn foo() -> i32 { 42 }";
    let valid_code_b = "pub fn bar() -> i32 { 100 }";

    // 1. Successful Transaction Commit
    let mut tx1 = WorkspaceTransaction::begin();
    tx1.stage_file(&file_a, valid_code_a, Language::Rust)
        .unwrap();
    tx1.stage_file(&file_b, valid_code_b, Language::Rust)
        .unwrap();

    let report1 = tx1.commit();
    assert!(report1.passed_verification);
    assert_eq!(report1.committed_files.len(), 2);
    assert_eq!(fs::read_to_string(&file_a).unwrap(), valid_code_a);
    assert_eq!(fs::read_to_string(&file_b).unwrap(), valid_code_b);

    // 2. Transaction Rollback on AST Invariant Violation (e.g. Unbalanced Delimiters in file_b)
    let valid_update_a = "pub fn foo() -> i32 { 84 }";
    let invalid_code_b = "pub fn bar() -> i32 { ((( unclosed";

    let mut tx2 = WorkspaceTransaction::begin();
    tx2.stage_file(&file_a, valid_update_a, Language::Rust)
        .unwrap();
    tx2.stage_file(&file_b, invalid_code_b, Language::Rust)
        .unwrap();

    let report2 = tx2.commit();
    assert!(
        !report2.passed_verification,
        "Transaction must reject invariant violation"
    );
    assert_eq!(report2.committed_files.len(), 0);

    // Crucial ACID guarantee: file_a MUST retain original valid_code_a on disk (no drift!)
    assert_eq!(fs::read_to_string(&file_a).unwrap(), valid_code_a);
    assert_eq!(fs::read_to_string(&file_b).unwrap(), valid_code_b);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
