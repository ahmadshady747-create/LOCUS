//! Data types and domain models for Deterministic Contract Lockdown & Mid-Execution Addendum Engine.
//!
//! Provides strict immutable specifications, state transition machines,
//! delta addendums, transaction freeze snapshots, and resumption receipts.

#![forbid(unsafe_code)]

use crate::cst::polyglot::types::InvariantBitset;
use crate::intent::socratic::dialogue::types::ArchitecturalPattern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Contract Identifier (Deterministic FNV-1a Hash)
// ---------------------------------------------------------------------------

/// Deterministic 64-bit identifier for a specification contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContractId(pub u64);

impl ContractId {
    /// Generates a deterministic `ContractId` from raw intent text and target symbols using FNV-1a.
    pub fn from_intent(intent: &str, target_symbols: &[String]) -> Self {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for byte in intent.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for sym in target_symbols {
            for byte in sym.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        Self(hash)
    }

    /// Returns the raw 64-bit integer ID.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Contract Lock State
// ---------------------------------------------------------------------------

/// Finite State Machine governing the lifecycle of a `SpecContract`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractLockState {
    /// Contract is being drafted and may be freely configured.
    Draft,
    /// Contract has been sealed into an immutable baseline; ad-hoc mutations are forbidden.
    Locked,
    /// An unhandled edge case occurred mid-execution; the transaction is frozen.
    FrozenMidExecution,
    /// An approved `ContractDelta` has been integrated; ready for resumption.
    DeltaApplied,
    /// Finalized and sealed after successful multi-file transaction commit.
    Sealed,
}

impl ContractLockState {
    /// True if the contract is currently locked or sealed against unmanaged mutation.
    pub fn is_immutable(&self) -> bool {
        matches!(self, Self::Locked | Self::Sealed)
    }

    /// True if the contract is frozen waiting for Socratic addendum resolution.
    pub fn is_frozen(&self) -> bool {
        matches!(self, Self::FrozenMidExecution)
    }
}

impl fmt::Display for ContractLockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "DRAFT"),
            Self::Locked => write!(f, "LOCKED"),
            Self::FrozenMidExecution => write!(f, "FROZEN_MID_EXECUTION"),
            Self::DeltaApplied => write!(f, "DELTA_APPLIED"),
            Self::Sealed => write!(f, "SEALED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Specification Contract
// ---------------------------------------------------------------------------

/// Deterministic, immutable specification contract governing autonomous multi-agent execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecContract {
    /// Unique deterministic identifier.
    pub id: ContractId,
    /// Raw intent FNV-1a hash.
    pub intent_hash: u64,
    /// Canonical symbols allowed to be declared or modified.
    pub target_symbols: Vec<String>,
    /// File system paths and AST subtrees permitted for modification.
    pub allowed_subtrees: Vec<String>,
    /// Bitset of mandatory AST safety invariants enforced by `AstGuard`.
    pub enforced_invariants: InvariantBitset,
    /// Maximum allowable cyclomatic complexity per generated symbol.
    pub max_cyclomatic_complexity: u32,
    /// Architectural taste pattern locked by Socratic resolution.
    pub taste_pattern: ArchitecturalPattern,
    /// Current lifecycle state in the finite state machine.
    pub lock_state: ContractLockState,
    /// Monotonically increasing version number (starts at 1).
    pub version: u32,
    /// Creation epoch timestamp in milliseconds.
    pub creation_timestamp: u64,
    /// Socratic resolution hints and design invariants.
    pub resolution_hints: Vec<String>,
    /// Extensible metadata key-value store.
    pub metadata: HashMap<String, String>,
}

impl SpecContract {
    /// Creates a new draft specification contract.
    pub fn new_draft(
        intent: &str,
        target_symbols: Vec<String>,
        allowed_subtrees: Vec<String>,
        taste_pattern: ArchitecturalPattern,
    ) -> Self {
        let id = ContractId::from_intent(intent, &target_symbols);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id,
            intent_hash: id.0,
            target_symbols,
            allowed_subtrees,
            enforced_invariants: InvariantBitset::new(),
            max_cyclomatic_complexity: 15,
            taste_pattern,
            lock_state: ContractLockState::Draft,
            version: 1,
            creation_timestamp: timestamp,
            resolution_hints: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Checks if a file path is permitted by the contract's allowed subtrees.
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if self.allowed_subtrees.is_empty() {
            return true;
        }
        let normalized = path.replace('\\', "/");
        self.allowed_subtrees.iter().any(|allowed| {
            let norm_allowed = allowed.replace('\\', "/");
            normalized.starts_with(&norm_allowed) || norm_allowed == "*"
        })
    }

    /// Checks if a symbol is allowed by the contract.
    pub fn is_symbol_allowed(&self, symbol: &str) -> bool {
        if self.target_symbols.is_empty() {
            return true;
        }
        self.target_symbols.iter().any(|s| s == symbol)
    }
}

// ---------------------------------------------------------------------------
// Contract Delta (Mid-Execution Addendum)
// ---------------------------------------------------------------------------

/// Structured, transactional diff applied to a frozen contract mid-execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractDelta {
    /// Unique delta identifier.
    pub delta_id: u64,
    /// Target contract ID to which this delta applies.
    pub target_contract_id: ContractId,
    /// Additional symbols authorized by the developer's Socratic answer.
    pub added_symbols: Vec<String>,
    /// Additional file paths / AST subtrees authorized.
    pub added_subtrees: Vec<String>,
    /// Modified or additional invariant bitset constraints.
    pub modified_invariants: InvariantBitset,
    /// Developer's explicit resolution hint explaining the amendment.
    pub user_resolution_hint: String,
    /// Optional updated architectural pattern if the resolution shifted the paradigm.
    pub applied_pattern: Option<ArchitecturalPattern>,
    /// Amendment timestamp in milliseconds.
    pub timestamp: u64,
}

impl ContractDelta {
    /// Constructs a new contract delta with auto-generated timestamp.
    pub fn new(
        target_contract_id: ContractId,
        added_symbols: Vec<String>,
        added_subtrees: Vec<String>,
        user_resolution_hint: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let delta_id = target_contract_id.0 ^ timestamp ^ 0xa5a5a5a5a5a5a5a5;

        Self {
            delta_id,
            target_contract_id,
            added_symbols,
            added_subtrees,
            modified_invariants: InvariantBitset::new(),
            user_resolution_hint,
            applied_pattern: None,
            timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// Frozen Transaction State
// ---------------------------------------------------------------------------

/// Zero-loss snapshot of an in-flight workspace transaction frozen during edge-case resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenTransactionState {
    /// Unique transaction identifier (e.g. `tx_workspace_98234`).
    pub tx_id: String,
    /// Associated contract ID.
    pub contract_id: ContractId,
    /// Number of memory buffers staged in `stage_tx`.
    pub staged_buffers_count: usize,
    /// List of file paths currently held in staged buffers.
    pub staged_files: Vec<String>,
    /// Specific ambiguity or missing invariant trigger that caused the freeze.
    pub freeze_reason: String,
    /// Epoch timestamp when the freeze occurred.
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Resume Receipt
// ---------------------------------------------------------------------------

/// Verifiable receipt confirming successful mid-execution resumption within latency budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumeReceipt {
    /// Associated transaction identifier.
    pub tx_id: String,
    /// Updated contract ID.
    pub contract_id: ContractId,
    /// New version of the resumed contract.
    pub resumed_version: u32,
    /// Elapsed duration in microseconds for the entire freeze->delta->resume transition.
    pub duration_micros: u64,
    /// Human-readable status confirmation.
    pub status: String,
}

// ---------------------------------------------------------------------------
// Contract Error
// ---------------------------------------------------------------------------

/// Typed domain errors for contract lockdown and addendum operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractError {
    /// Attempted direct modification of a locked contract without a delta.
    ContractAlreadyLocked(ContractId),
    /// Attempted to apply a delta to a contract that is not in `FrozenMidExecution` state.
    ContractNotFrozen(ContractId),
    /// Attempted execution against a contract currently frozen.
    ContractFrozen(ContractId),
    /// Contract is permanently sealed after commit; no further deltas allowed.
    ContractSealed(ContractId),
    /// Delta target ID mismatch.
    InvalidDeltaTarget { expected: ContractId, found: ContractId },
    /// Referenced transaction not found in the frozen transaction registry.
    TransactionNotFound(String),
    /// Generated code violated mandatory invariant bitset rules.
    InvariantViolation(String),
    /// Cyclomatic complexity exceeded the contract limit.
    ComplexityBudgetExceeded { max: u32, actual: u32 },
    /// AST mutation targeted a path outside the allowed subtrees.
    UnauthorizedSubtreeMutation { path: String },
    /// Unauthorized symbol generated outside contract specification.
    UnauthorizedSymbolGeneration { symbol: String },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContractAlreadyLocked(id) => write!(f, "Contract {} is locked; ad-hoc mutations forbidden without ContractDelta", id),
            Self::ContractNotFrozen(id) => write!(f, "Contract {} is not frozen; cannot apply delta in current state", id),
            Self::ContractFrozen(id) => write!(f, "Contract {} is currently frozen waiting for Socratic resolution", id),
            Self::ContractSealed(id) => write!(f, "Contract {} is permanently sealed; transaction committed", id),
            Self::InvalidDeltaTarget { expected, found } => write!(f, "Delta target mismatch: expected {}, found {}", expected, found),
            Self::TransactionNotFound(tx) => write!(f, "Transaction '{}' not found in frozen store", tx),
            Self::InvariantViolation(reason) => write!(f, "Invariant violation: {}", reason),
            Self::ComplexityBudgetExceeded { max, actual } => write!(f, "Cyclomatic complexity {} exceeded maximum budget {}", actual, max),
            Self::UnauthorizedSubtreeMutation { path } => write!(f, "Mutation in '{}' forbidden by allowed subtrees", path),
            Self::UnauthorizedSymbolGeneration { symbol } => write!(f, "Symbol '{}' is not authorized by contract", symbol),
        }
    }
}

impl std::error::Error for ContractError {}
