//! Deterministic Specification Lockdown Engine.
//!
//! Transforms ambiguous natural-language intent and Socratic dialogue resolutions into
//! mathematically verified, immutable `SpecContract` baselines.

#![forbid(unsafe_code)]

use crate::cst::polyglot::types::InvariantBitset;
use crate::intent::socratic::contract::types::{ContractError, ContractLockState, SpecContract};
use crate::intent::socratic::dialogue::types::{ArchitecturalPattern, IntentResolution};
use crate::intent::socratic::types::IntentEntropyReport;

/// High-throughput engine for synthesizing and locking deterministic specification contracts.
pub struct SpecLockdownEngine;

impl SpecLockdownEngine {
    /// Locks a contract from an `IntentResolution` produced by Socratic Dialogue.
    pub fn lock_from_resolution(
        intent: &str,
        resolution: &IntentResolution,
        entropy_report: &IntentEntropyReport,
        target_symbols: Vec<String>,
        allowed_subtrees: Vec<String>,
    ) -> Result<SpecContract, ContractError> {
        let mut contract = SpecContract::new_draft(
            intent,
            target_symbols,
            allowed_subtrees,
            resolution.applied_pattern,
        );

        // Populate resolution hints and design invariants
        contract.resolution_hints = resolution.synthesized_contract_hints.clone();
        
        // Enforce invariants based on resolution invariants
        let mut bitset = InvariantBitset::new();
        for inv_str in &resolution.enforced_invariants {
            match inv_str.as_str() {
                "AsyncMutexAcrossAwait" => bitset.set_violation(1),
                "DivisionByZero" => bitset.set_violation(2),
                "ArrayOutOfBounds" => bitset.set_violation(3),
                "UnsafeUnwrap" => bitset.set_violation(4),
                "NullDereference" => bitset.set_violation(6),
                "ConditionalHookCall" => bitset.set_violation(7),
                "ClientSecretLeak" => bitset.set_violation(8),
                "JsxTagMismatch" => bitset.set_violation(10),
                "ReactStateRace" => bitset.set_violation(13),
                "PathTraversal" => bitset.set_violation(16),
                "DynamicCodeEval" => bitset.set_violation(18),
                _ => {}
            }
        }

        // Lock down edge cases discovered in entropy report
        for edge_case in &entropy_report.underspecified_edge_cases {
            use crate::intent::socratic::types::EdgeCaseKind;
            match edge_case.kind {
                EdgeCaseKind::ConcurrencyGap => bitset.set_violation(1),
                EdgeCaseKind::NullabilityBoundaryGap => bitset.set_violation(6),
                EdgeCaseKind::InputValidationGap => bitset.set_violation(3),
                EdgeCaseKind::MissingErrorHandling => bitset.set_violation(4),
                EdgeCaseKind::ResourceCleanupGap => bitset.set_violation(24),
                EdgeCaseKind::SecurityContextGap => bitset.set_violation(8),
            }
        }
        contract.enforced_invariants = bitset;

        // Tune complexity budget based on architectural pattern
        contract.max_cyclomatic_complexity = match resolution.applied_pattern {
            ArchitecturalPattern::DataOriented | ArchitecturalPattern::FunctionalPipeline => 10,
            ArchitecturalPattern::CleanArchitecture | ArchitecturalPattern::Hexagonal => 15,
            ArchitecturalPattern::MicroKernel | ArchitecturalPattern::ActorMesh => 12,
        };

        // Transition from Draft to Locked
        contract.lock_state = ContractLockState::Locked;

        Ok(contract)
    }

    /// Creates and immediately locks a deterministic contract from direct explicit parameters.
    pub fn lock_direct(
        intent: &str,
        pattern: ArchitecturalPattern,
        symbols: Vec<String>,
        subtrees: Vec<String>,
        invariants: InvariantBitset,
        max_complexity: u32,
    ) -> SpecContract {
        let mut contract = SpecContract::new_draft(intent, symbols, subtrees, pattern);
        contract.enforced_invariants = invariants;
        contract.max_cyclomatic_complexity = max_complexity;
        contract.lock_state = ContractLockState::Locked;
        contract
    }

    /// Verifies strict compliance of generated code artifacts against the locked contract.
    pub fn verify_compliance(
        contract: &SpecContract,
        modified_symbols: &[String],
        target_file: &str,
        cyclomatic_complexity: u32,
        detected_violations: &InvariantBitset,
    ) -> Result<(), ContractError> {
        // 1. Check if contract is currently frozen or sealed
        if contract.lock_state == ContractLockState::FrozenMidExecution {
            return Err(ContractError::ContractFrozen(contract.id));
        }

        // 2. Check subtree boundary
        if !contract.is_path_allowed(target_file) {
            return Err(ContractError::UnauthorizedSubtreeMutation {
                path: target_file.to_string(),
            });
        }

        // 3. Check symbol boundary
        for sym in modified_symbols {
            if !contract.is_symbol_allowed(sym) {
                return Err(ContractError::UnauthorizedSymbolGeneration {
                    symbol: sym.clone(),
                });
            }
        }

        // 4. Check cyclomatic complexity
        if cyclomatic_complexity > contract.max_cyclomatic_complexity {
            return Err(ContractError::ComplexityBudgetExceeded {
                max: contract.max_cyclomatic_complexity,
                actual: cyclomatic_complexity,
            });
        }

        // 5. Check invariant compliance
        // If the contract enforces an invariant, detected_violations must NOT contain it
        let common = InvariantBitset(contract.enforced_invariants.0 & detected_violations.0);
        if !common.is_clean() {
            return Err(ContractError::InvariantViolation(format!(
                "Generated code violated {} contract-enforced invariant(s)",
                common.violation_count()
            )));
        }

        Ok(())
    }

    /// Permanently seals a contract after successful multi-file transaction commit.
    pub fn seal(contract: &mut SpecContract) -> Result<(), ContractError> {
        if contract.lock_state == ContractLockState::FrozenMidExecution {
            return Err(ContractError::ContractFrozen(contract.id));
        }
        contract.lock_state = ContractLockState::Sealed;
        Ok(())
    }
}
