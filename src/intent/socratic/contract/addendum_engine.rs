//! Dynamic Mid-Execution Addendum Engine.
//!
//! Handles zero-loss transaction freezing (`stage_tx` freeze), Socratic delta amendment,
//! and sub-millisecond swarm resumption.

#![forbid(unsafe_code)]

use crate::intent::socratic::contract::types::{
    ContractDelta, ContractError, ContractLockState, FrozenTransactionState, ResumeReceipt, SpecContract,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// High-throughput engine managing mid-execution transaction freeze, delta merge, and resumption.
#[derive(Debug, Clone)]
pub struct DynamicAddendumEngine {
    frozen_store: Arc<RwLock<HashMap<String, FrozenTransactionState>>>,
}

impl Default for DynamicAddendumEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicAddendumEngine {
    /// Creates a new in-memory `DynamicAddendumEngine`.
    pub fn new() -> Self {
        Self {
            frozen_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Freezes an in-flight transaction mid-execution upon discovering an unhandled edge-case.
    pub fn freeze_transaction(
        &self,
        tx_id: &str,
        contract: &mut SpecContract,
        staged_files: Vec<String>,
        reason: &str,
    ) -> Result<FrozenTransactionState, ContractError> {
        // Only locked or delta-applied contracts can be frozen mid-execution
        if contract.lock_state == ContractLockState::Sealed {
            return Err(ContractError::ContractSealed(contract.id));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let frozen_state = FrozenTransactionState {
            tx_id: tx_id.to_string(),
            contract_id: contract.id,
            staged_buffers_count: staged_files.len(),
            staged_files,
            freeze_reason: reason.to_string(),
            timestamp,
        };

        // Transition contract state to FrozenMidExecution
        contract.lock_state = ContractLockState::FrozenMidExecution;

        // Store snapshot in in-memory registry
        if let Ok(mut store) = self.frozen_store.write() {
            store.insert(tx_id.to_string(), frozen_state.clone());
        }

        Ok(frozen_state)
    }

    /// Applies a structured Socratic `ContractDelta` to a frozen contract.
    pub fn apply_contract_delta(
        &self,
        contract: &mut SpecContract,
        delta: &ContractDelta,
    ) -> Result<SpecContract, ContractError> {
        // 1. Verify contract is in FrozenMidExecution state
        if contract.lock_state != ContractLockState::FrozenMidExecution {
            return Err(ContractError::ContractNotFrozen(contract.id));
        }

        // 2. Verify target contract ID matches
        if delta.target_contract_id != contract.id {
            return Err(ContractError::InvalidDeltaTarget {
                expected: contract.id,
                found: delta.target_contract_id,
            });
        }

        // 3. Atomically merge added symbols
        for sym in &delta.added_symbols {
            if !contract.target_symbols.contains(sym) {
                contract.target_symbols.push(sym.clone());
            }
        }

        // 4. Atomically merge added subtrees
        for tree in &delta.added_subtrees {
            if !contract.allowed_subtrees.contains(tree) {
                contract.allowed_subtrees.push(tree.clone());
            }
        }

        // 5. Merge invariant bitset
        contract.enforced_invariants.merge(delta.modified_invariants);

        // 6. Update architectural pattern if shifted
        if let Some(new_pattern) = delta.applied_pattern {
            contract.taste_pattern = new_pattern;
        }

        // 7. Add user resolution hint
        if !delta.user_resolution_hint.is_empty() {
            contract.resolution_hints.push(delta.user_resolution_hint.clone());
        }

        // 8. Increment version & transition to DeltaApplied
        contract.version += 1;
        contract.lock_state = ContractLockState::DeltaApplied;

        Ok(contract.clone())
    }

    /// Resumes a frozen transaction, restoring swarm execution within $< 1.0\,\text{ms}$.
    pub fn resume_transaction(
        &self,
        frozen_state: &FrozenTransactionState,
        updated_contract: &mut SpecContract,
    ) -> Result<ResumeReceipt, ContractError> {
        let start = Instant::now();

        // 1. Verify IDs
        if frozen_state.contract_id != updated_contract.id {
            return Err(ContractError::InvalidDeltaTarget {
                expected: frozen_state.contract_id,
                found: updated_contract.id,
            });
        }

        // 2. Verify contract state is DeltaApplied
        if updated_contract.lock_state != ContractLockState::DeltaApplied {
            return Err(ContractError::ContractNotFrozen(updated_contract.id));
        }

        // 3. Re-lock contract for ongoing execution
        updated_contract.lock_state = ContractLockState::Locked;

        // 4. Remove from frozen registry
        if let Ok(mut store) = self.frozen_store.write() {
            store.remove(&frozen_state.tx_id);
        }

        let elapsed_micros = start.elapsed().as_micros() as u64;

        Ok(ResumeReceipt {
            tx_id: frozen_state.tx_id.clone(),
            contract_id: updated_contract.id,
            resumed_version: updated_contract.version,
            duration_micros: elapsed_micros,
            status: "RESUMED_SUCCESSFULLY".to_string(),
        })
    }

    /// Retrieves an active frozen transaction by its ID.
    pub fn get_frozen_transaction(&self, tx_id: &str) -> Option<FrozenTransactionState> {
        if let Ok(store) = self.frozen_store.read() {
            store.get(tx_id).cloned()
        } else {
            None
        }
    }
}
