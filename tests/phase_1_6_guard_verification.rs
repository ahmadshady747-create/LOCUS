//! Phase 1.6 Verification: 32-Pass Deterministic Invariant Safety Engine.

use locus_engine::guard::{AstGuard, RuleMask, RuleRunner};
use locus_engine::types::ViolationKind;

#[test]
fn test_rule_20_circular_mem_leak() {
    let code = r#"
    use std::rc::Rc;
    use std::cell::RefCell;
    pub struct GraphNode {
        pub id: usize,
        pub parent: Option<Rc<RefCell<GraphNode>>>,
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::CircularMemLeak));
}

#[test]
fn test_rule_21_async_cancellation_safety() {
    let code = r#"
    pub async fn process(mut state: State) {
        tokio::select! {
            _ = cancel_rx.recv() => {
                state.pending_balance = 0;
                flush_to_db().await;
            }
        }
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::AsyncCancellationSafety));
}

#[test]
fn test_rule_22_constant_time_crypto() {
    let code = r#"
    pub fn verify_token(user_input: &str, secretToken: &str) -> bool {
        user_input == secretToken
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::ConstantTimeCrypto));
}

#[test]
fn test_rule_23_exhaustive_enum_narrowing() {
    let code = r#"
    export function handleAction(action: { type: string }) {
        switch (action.type) {
            case "LOGIN":
                return true;
            case "LOGOUT":
                return false;
        }
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::ExhaustiveEnumNarrowing));
}

#[test]
fn test_rule_24_resource_descriptor_leak() {
    let code = r#"
    function processFile(path: string) {
        const fd = fs.openSync(path, 'r');
        const data = fs.readFileSync(fd);
        return data;
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::ResourceDescriptorLeak));
}

#[test]
fn test_rule_25_ssrf_unsafe_fetch() {
    let code = r#"
    export async function fetchMetadata() {
        return fetch("http://169.254.169.254/latest/meta-data/");
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::SsrfUnsafeFetch));
}

#[test]
fn test_rule_26_unbounded_channel_deadlock() {
    let code = r#"
    pub fn spawn_workers() {
        let (tx, _rx) = std::sync::mpsc::sync_channel(0);
        for i in 0..10 {
            let _ = tx.send(i);
        }
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::UnboundedChannelDeadlock));
}

#[test]
fn test_rule_27_prototype_pollution() {
    let code = r#"
    export function extend(target: any, source: any) {
        target["__proto__"] = source;
        return target;
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::PrototypePollution));
}

#[test]
fn test_rule_28_cors_wildcard_credentials() {
    let code = r#"
    const corsConfig = {
        origin: "*",
        credentials: true,
    };
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::CorsWildcardCredentials));
}

#[test]
fn test_rule_29_hardcoded_key_entropy() {
    let code = concat!("const apiKey = \"", "sk_", "live_1234567890abcdef1234567890abcdef\";");
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::HardcodedKeyEntropy));
}

#[test]
fn test_rule_30_unchecked_arithmetic_overflow() {
    let code = r#"
    pub fn count_forever() {
        while true {
            let mut counter: u8 = 0;
            counter += 1;
        }
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::UncheckedArithmeticOverflow));
}

#[test]
fn test_rule_31_atomic_state_mutation() {
    let code = r#"
    export function updateProfile(useStore: any) {
        useStore.setState((state: any) => {
            state.userName = "Alice";
        });
    }
    "#;
    let rep = AstGuard::verify(code);
    assert!(!rep.passed);
    assert_eq!(rep.violation, Some(ViolationKind::AtomicStateMutation));
}

#[test]
fn test_rule_runner_32_bitset_and_latency() {
    let safe_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let rep = RuleRunner::verify_all(safe_code);
    assert!(rep.passed);
    assert_eq!(RuleMask::ALL_RULES.0, 0xFFFFFFFF);
    assert_eq!(RuleMask::CORE_RULES.0, 0x7FF);
    assert_eq!(RuleMask::EXTENDED_RULES.0, 0xFFFFF800);
}
