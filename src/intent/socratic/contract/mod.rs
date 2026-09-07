//! Deterministic Contract Lockdown & Mid-Execution Addendum Engine.
//!
//! Provides strict immutable specifications, state transition machines,
//! delta addendums, transaction freeze snapshots, and sub-millisecond resumption.

#![forbid(unsafe_code)]

pub mod addendum_engine;
pub mod lockdown_engine;
pub mod types;

pub use addendum_engine::DynamicAddendumEngine;
pub use lockdown_engine::SpecLockdownEngine;
pub use types::{
    ContractDelta, ContractError, ContractId, ContractLockState, FrozenTransactionState,
    ResumeReceipt, SpecContract,
};

// ---------------------------------------------------------------------------
// Unit & Invariant Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::polyglot::types::InvariantBitset;
    use crate::intent::socratic::dialogue::types::{ArchitecturalPattern, IntentResolution, TasteScore};
    use crate::intent::socratic::entropy_calculator::AmbiguityEntropyCalculator;
    use crate::intent::socratic::types::{EdgeCaseKind, EdgeCaseViolation};
    use std::time::Instant;

    #[test]
    fn test_contract_id_deterministic_hashing() {
        let symbols1 = vec!["UserRepo".to_string(), "find_by_id".to_string()];
        let symbols2 = vec!["UserRepo".to_string(), "find_by_id".to_string()];
        let symbols_diff = vec!["OrderRepo".to_string()];

        let id1 = ContractId::from_intent("Create User repository with caching", &symbols1);
        let id2 = ContractId::from_intent("Create User repository with caching", &symbols2);
        let id3 = ContractId::from_intent("Create User repository with caching", &symbols_diff);

        assert_eq!(id1, id2, "Identical intent and symbols must yield identical ContractId");
        assert_ne!(id1, id3, "Different symbols must yield different ContractId");
    }

    #[test]
    fn test_lock_from_intent_resolution_and_invariants() {
        let intent = "Implement RateLimiter with sliding window counter in src/ratelimit.rs";
        let resolution = IntentResolution {
            question_id: "sq_ratelimit_1".to_string(),
            selected_choice_id: "choice_atomic_cas".to_string(),
            applied_pattern: ArchitecturalPattern::DataOriented,
            synthesized_contract_hints: vec![
                "pub struct SlidingWindowLimiter".to_string(),
                "pub fn check_rate(&self, key: &str) -> bool".to_string(),
            ],
            enforced_invariants: vec![
                "AsyncMutexAcrossAwait".to_string(),
                "DivisionByZero".to_string(),
            ],
            final_taste_score: TasteScore::new(0.92),
        };

        let mut entropy_report = AmbiguityEntropyCalculator::evaluate(intent, None, None, None);
        entropy_report.underspecified_edge_cases.push(EdgeCaseViolation {
            kind: EdgeCaseKind::ConcurrencyGap,
            description: "Potential race under high throughput concurrent token consumption".to_string(),
            suggested_guard: "Atomic CAS state transitions".to_string(),
        });

        let contract = SpecLockdownEngine::lock_from_resolution(
            intent,
            &resolution,
            &entropy_report,
            vec!["SlidingWindowLimiter".to_string(), "check_rate".to_string()],
            vec!["src/ratelimit.rs".to_string()],
        )
        .expect("Lockdown must succeed");

        assert_eq!(contract.lock_state, ContractLockState::Locked);
        assert_eq!(contract.version, 1);
        assert_eq!(contract.taste_pattern, ArchitecturalPattern::DataOriented);
        assert_eq!(contract.max_cyclomatic_complexity, 10);
        assert!(contract.is_path_allowed("src/ratelimit.rs"));
        assert!(!contract.is_path_allowed("src/other_module.rs"));
        assert!(contract.is_symbol_allowed("SlidingWindowLimiter"));
        assert!(!contract.is_symbol_allowed("RandomUnregisteredSymbol"));

        // Invariant bitset must contain AsyncMutexAcrossAwait (1) and DivisionByZero (2)
        assert!(contract.enforced_invariants.has_violation(1));
        assert!(contract.enforced_invariants.has_violation(2));
    }

    #[test]
    fn test_mid_execution_freeze_delta_and_resumption() {
        let engine = DynamicAddendumEngine::new();
        let intent = "Implement OrderService with payment processing in src/services/order.rs";

        let mut contract = SpecLockdownEngine::lock_direct(
            intent,
            ArchitecturalPattern::CleanArchitecture,
            vec!["OrderService".to_string(), "create_order".to_string()],
            vec!["src/services/order.rs".to_string()],
            InvariantBitset::new(),
            15,
        );

        let tx_id = "tx_workspace_49201";
        let staged_files = vec!["src/services/order.rs".to_string()];

        // 1. Freeze transaction mid-execution
        let frozen_state = engine
            .freeze_transaction(
                tx_id,
                &mut contract,
                staged_files.clone(),
                "Discovered unhandled idempotent retry edge-case",
            )
            .expect("Freeze must succeed");

        assert_eq!(contract.lock_state, ContractLockState::FrozenMidExecution);
        assert_eq!(frozen_state.staged_buffers_count, 1);
        assert_eq!(frozen_state.staged_files, staged_files);

        // 2. Applying delta with mismatched ID must fail
        let wrong_id_delta = ContractDelta::new(
            ContractId(0xdeadbeef),
            vec!["IdempotencyStore".to_string()],
            vec!["src/storage/idempotency.rs".to_string()],
            "Add IdempotencyStore".to_string(),
        );
        let err = engine.apply_contract_delta(&mut contract, &wrong_id_delta);
        assert!(err.is_err());

        // 3. Apply authorized contract delta
        let delta = ContractDelta::new(
            contract.id,
            vec!["IdempotencyStore".to_string(), "verify_idempotency_key".to_string()],
            vec!["src/storage/idempotency.rs".to_string()],
            "Developer clarified: use dedicated Redis-backed IdempotencyStore".to_string(),
        );

        let updated_contract = engine
            .apply_contract_delta(&mut contract, &delta)
            .expect("Delta application must succeed");

        assert_eq!(updated_contract.version, 2);
        assert_eq!(updated_contract.lock_state, ContractLockState::DeltaApplied);
        assert!(updated_contract.target_symbols.contains(&"IdempotencyStore".to_string()));
        assert!(updated_contract.allowed_subtrees.contains(&"src/storage/idempotency.rs".to_string()));
        assert_eq!(
            updated_contract.resolution_hints.last().unwrap(),
            "Developer clarified: use dedicated Redis-backed IdempotencyStore"
        );

        // 4. Resume transaction
        let receipt = engine
            .resume_transaction(&frozen_state, &mut contract)
            .expect("Resumption must succeed");

        assert_eq!(receipt.tx_id, tx_id);
        assert_eq!(receipt.resumed_version, 2);
        assert_eq!(receipt.status, "RESUMED_SUCCESSFULLY");
        assert_eq!(contract.lock_state, ContractLockState::Locked);
        assert!(engine.get_frozen_transaction(tx_id).is_none());
    }

    #[test]
    fn test_compliance_verification_accepts_valid_and_rejects_violations() {
        let mut invariants = InvariantBitset::new();
        invariants.set_violation(1); // AsyncMutexAcrossAwait
        invariants.set_violation(6); // NullDereference

        let contract = SpecLockdownEngine::lock_direct(
            "Build PaymentProcessor in src/payment.rs",
            ArchitecturalPattern::CleanArchitecture,
            vec!["PaymentProcessor".to_string(), "charge".to_string()],
            vec!["src/payment.rs".to_string()],
            invariants,
            12,
        );

        // 1. Valid code compliance
        let empty_violations = InvariantBitset::new();
        let ok = SpecLockdownEngine::verify_compliance(
            &contract,
            &["PaymentProcessor".to_string()],
            "src/payment.rs",
            5,
            &empty_violations,
        );
        assert!(ok.is_ok());

        // 2. Unauthorized subtree mutation
        let bad_path = SpecLockdownEngine::verify_compliance(
            &contract,
            &["PaymentProcessor".to_string()],
            "src/unauthorized_admin.rs",
            5,
            &empty_violations,
        );
        assert!(matches!(bad_path, Err(ContractError::UnauthorizedSubtreeMutation { .. })));

        // 3. Unauthorized symbol generation
        let bad_sym = SpecLockdownEngine::verify_compliance(
            &contract,
            &["BackdoorAdminExploit".to_string()],
            "src/payment.rs",
            5,
            &empty_violations,
        );
        assert!(matches!(bad_sym, Err(ContractError::UnauthorizedSymbolGeneration { .. })));

        // 4. Cyclomatic complexity exceeded
        let bad_complexity = SpecLockdownEngine::verify_compliance(
            &contract,
            &["PaymentProcessor".to_string()],
            "src/payment.rs",
            18,
            &empty_violations,
        );
        assert!(matches!(bad_complexity, Err(ContractError::ComplexityBudgetExceeded { max: 12, actual: 18 })));

        // 5. Invariant violation detected
        let mut detected_violations = InvariantBitset::new();
        detected_violations.set_violation(1); // violates AsyncMutexAcrossAwait
        let bad_invariant = SpecLockdownEngine::verify_compliance(
            &contract,
            &["PaymentProcessor".to_string()],
            "src/payment.rs",
            5,
            &detected_violations,
        );
        assert!(matches!(bad_invariant, Err(ContractError::InvariantViolation(..))));
    }

    #[test]
    fn test_rejection_of_unauthorized_state_mutations() {
        let engine = DynamicAddendumEngine::new();
        let mut contract = SpecLockdownEngine::lock_direct(
            "Task contract",
            ArchitecturalPattern::MicroKernel,
            vec!["Kernel".to_string()],
            vec!["src/kernel.rs".to_string()],
            InvariantBitset::new(),
            10,
        );

        // Cannot apply delta to locked contract without freezing first
        let delta = ContractDelta::new(contract.id, vec![], vec![], "hint".to_string());
        let err = engine.apply_contract_delta(&mut contract, &delta);
        assert!(matches!(err, Err(ContractError::ContractNotFrozen(..))));

        // Seal the contract
        SpecLockdownEngine::seal(&mut contract).expect("Seal succeeds");
        assert_eq!(contract.lock_state, ContractLockState::Sealed);

        // Cannot freeze a sealed contract
        let freeze_err = engine.freeze_transaction("tx1", &mut contract, vec![], "reason");
        assert!(matches!(freeze_err, Err(ContractError::ContractSealed(..))));
    }

    #[test]
    fn test_sub_millisecond_freeze_delta_resume_latency() {
        let engine = DynamicAddendumEngine::new();
        let contract = SpecLockdownEngine::lock_direct(
            "High frequency trading book in src/orderbook.rs",
            ArchitecturalPattern::DataOriented,
            vec!["OrderBook".to_string(), "insert_limit_order".to_string()],
            vec!["src/orderbook.rs".to_string()],
            InvariantBitset::new(),
            10,
        );

        let tx_id = "tx_hft_performance_test";
        let staged_files = vec!["src/orderbook.rs".to_string()];

        let iterations = 1000;
        let start = Instant::now();

        for i in 0..iterations {
            let mut c = contract.clone();
            let frozen = engine
                .freeze_transaction(tx_id, &mut c, staged_files.clone(), "Latency test")
                .expect("Freeze succeeds");

            let delta = ContractDelta::new(
                c.id,
                vec![format!("MatchingEngine_{}", i)],
                vec!["src/engine.rs".to_string()],
                "Latency iteration".to_string(),
            );

            let _ = engine.apply_contract_delta(&mut c, &delta).expect("Delta succeeds");
            let _ = engine.resume_transaction(&frozen, &mut c).expect("Resume succeeds");
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_cycle_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average Freeze + Delta + Resume Full Cycle Latency: {:.4}ms", avg_cycle_latency_ms);

        let budget = if cfg!(debug_assertions) { 0.50 } else { 0.05 };
        assert!(
            avg_cycle_latency_ms < budget,
            "Full cycle latency must be strictly sub-millisecond (budget {:.2}ms, got {:.4}ms)",
            budget,
            avg_cycle_latency_ms
        );
    }
}
