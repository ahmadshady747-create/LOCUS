//! Phase 1.6 Verification: Swarm Consensus, Hierarchical Wildcard Leases, OCC & Deadlock Resolution.

use locus_engine::lease::{LeaseBroker, LeaseRegistry};
use locus_engine::types::LeaseStatus;

#[test]
fn test_hierarchical_wildcard_lease_blocks_child_symbol() {
    let registry = LeaseRegistry::new();

    // 1. Agent Alpha acquires module-wide wildcard lease on src/auth/*
    let status_a = registry.acquire("src/auth/*", "agent_alpha", 10_000);
    assert!(matches!(status_a, LeaseStatus::Acquired(_)));

    // 2. Agent Beta attempts to acquire specific child symbol src/auth/jwt.rs::verify -> HierarchicalConflict
    let status_b = registry.acquire("src/auth/jwt.rs::verify", "agent_beta", 10_000);
    match status_b {
        LeaseStatus::HierarchicalConflict {
            requested_fqn,
            blocking_lease_fqn,
            current_holder,
            remaining_ttl_ms,
        } => {
            assert_eq!(requested_fqn, "src/auth/jwt.rs::verify");
            assert_eq!(blocking_lease_fqn, "src/auth/*");
            assert_eq!(current_holder, "agent_alpha");
            assert!(remaining_ttl_ms > 0);
        }
        _ => panic!("Expected HierarchicalConflict when acquiring child symbol of active wildcard lease"),
    }

    // 3. Agent Beta can acquire non-overlapping module
    let status_c = registry.acquire("src/billing/stripe.rs::charge", "agent_beta", 10_000);
    assert!(matches!(status_c, LeaseStatus::Acquired(_)));
}

#[test]
fn test_child_symbol_lease_blocks_parent_wildcard() {
    let registry = LeaseRegistry::new();

    // 1. Agent Alpha acquires specific leaf symbol
    let status_a = registry.acquire("crate::billing::invoice::generate", "agent_alpha", 10_000);
    assert!(matches!(status_a, LeaseStatus::Acquired(_)));

    // 2. Agent Beta attempts to acquire parent wildcard crate::billing::* -> HierarchicalConflict
    let status_b = registry.acquire("crate::billing::*", "agent_beta", 10_000);
    match status_b {
        LeaseStatus::HierarchicalConflict {
            requested_fqn,
            blocking_lease_fqn,
            current_holder,
            ..
        } => {
            assert_eq!(requested_fqn, "crate::billing::*");
            assert_eq!(blocking_lease_fqn, "crate::billing::invoice::generate");
            assert_eq!(current_holder, "agent_alpha");
        }
        _ => panic!("Expected HierarchicalConflict when parent wildcard covers existing leaf lease"),
    }
}

#[test]
fn test_occ_version_advancement_and_conflict_rejection() {
    let registry = LeaseRegistry::new();
    let fqn = "src/models/user.rs::update_profile";

    // 1. Default initial version is 1
    assert_eq!(registry.get_occ_version(fqn), 1);
    assert_eq!(registry.verify_occ_token(fqn, 1), Ok(1));

    // 2. Agent Alpha successfully commits with expected version 1 -> advances to version 2
    let new_ver = registry.commit_occ(fqn, 1).expect("OCC commit should succeed for version 1");
    assert_eq!(new_ver, 2);
    assert_eq!(registry.get_occ_version(fqn), 2);

    // 3. Agent Beta attempts stale commit with expected version 1 -> OccMismatch
    let stale_commit = registry.commit_occ(fqn, 1);
    match stale_commit {
        Err(LeaseStatus::OccMismatch {
            fqn: err_fqn,
            current_version,
            expected_version,
        }) => {
            assert_eq!(err_fqn, fqn);
            assert_eq!(current_version, 2);
            assert_eq!(expected_version, 1);
        }
        _ => panic!("Expected OccMismatch on stale OCC commit"),
    }

    // 4. Agent Beta re-reads version 2 and successfully commits -> advances to version 3
    let next_ver = registry.commit_occ(fqn, 2).expect("OCC commit should succeed for version 2");
    assert_eq!(next_ver, 3);
}

#[test]
fn test_deadlock_detection_and_automatic_cycle_breaking() {
    let registry = LeaseRegistry::new();
    let fqn_1 = "src/core/resource_a.rs::lock";
    let fqn_2 = "src/core/resource_b.rs::lock";

    // 1. Agent Alpha acquires Resource A
    let _ = registry.acquire(fqn_1, "agent_alpha", 10_000);

    // 2. Agent Beta acquires Resource B
    let _ = registry.acquire(fqn_2, "agent_beta", 10_000);

    // 3. Agent Alpha waits for Resource B (held by Beta) -> Ok
    let wait_res_a = registry.register_wait("agent_alpha", fqn_2);
    assert!(wait_res_a.is_ok());

    // 4. Agent Beta waits for Resource A (held by Alpha) -> Forms circular wait [Alpha -> Beta -> Alpha]
    let wait_res_b = registry.register_wait("agent_beta", fqn_1);
    assert!(wait_res_b.is_err(), "Expected deadlock cycle to be detected and broken");

    let resolution = wait_res_b.unwrap_err();
    assert!(resolution.cycle.contains(&"agent_alpha".to_string()));
    assert!(resolution.cycle.contains(&"agent_beta".to_string()));
    assert!(!resolution.broken_lease_id.is_empty());

    // 5. Verify that the evicted lease was removed to break the deadlock
    let active_lease = registry.get_active_lease(&resolution.broken_fqn);
    assert!(active_lease.is_none(), "Evicted lease must be released");
}

#[test]
fn test_wildcard_prefix_and_matching_helpers() {
    assert!(LeaseBroker::is_wildcard("src/auth/*"));
    assert!(!LeaseBroker::is_wildcard("src/auth/login.rs"));

    assert_eq!(LeaseBroker::wildcard_prefix("src/auth/*"), "src/auth/");
    assert_eq!(LeaseBroker::wildcard_prefix("crate::billing::*"), "crate::billing::");

    assert!(LeaseBroker::matches_hierarchical("src/auth/*", "src/auth/jwt.rs::verify"));
    assert!(LeaseBroker::matches_hierarchical("crate::billing::*", "crate::billing::invoice"));
    assert!(!LeaseBroker::matches_hierarchical("src/auth/*", "src/billing/stripe.rs"));
}
