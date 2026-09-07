//! Intent Synthesis, Ambiguity Entropy & Socratic Dialogues.

#![forbid(unsafe_code)]

pub mod socratic;

pub use socratic::{
    AmbiguityEntropyCalculator, AmbiguityScore, AmbiguousTarget, ArchitecturalChoice,
    ArchitecturalPattern, ArchitecturalTasteEngine, ContractDelta, ContractError, ContractId,
    ContractLockState, DynamicAddendumEngine, EdgeCaseDetector, EdgeCaseKind, EdgeCaseViolation,
    EntropyBreakdown, EntropyThresholdConfig, FrozenTransactionState, IntentEntropyReport,
    IntentResolution, ResumeReceipt, SocraticQuestion, SocraticQuestionGenerator, SpecContract,
    SpecLockdownEngine, TasteScore, TradeoffMatrix, TradeoffRating,
};
