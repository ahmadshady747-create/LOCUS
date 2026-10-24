#![forbid(unsafe_code)]

//! locus-engine: Deterministic AST Verification, Semantic Symbol Graph & Surgical Patching Engine.

pub mod cache;
pub mod contract;
pub mod cst;
pub mod daemon;
pub mod diff;
pub mod formal;
pub mod graph;
pub mod guard;
pub mod hud;
pub mod intent;
pub mod lease;
pub mod mcp;
pub mod meta_mcp;
pub mod parser;
pub mod proof_verifier;
pub mod release;
pub mod remediate;
pub mod sarif;
pub mod scheduler;
pub mod search;
pub mod semantic;
pub mod slice;
pub mod swarm;
pub mod taint;
pub mod tx;
pub mod types;
pub mod ui_guard;
pub mod wal;
pub mod wasm;

// Primary public exports
pub use cache::{AstContextCache, CachedEntry};
pub use contract::{ContractSynthesizer, ContractVerificationReport, IntentContract};
pub use cst::{
    parse_to_cst, parse_to_green, to_lossless_text, CCppCstDriver, CfgEdge, CfgEdgeKind, CfgNode,
    CfgNodeKind, ControlFlowGraph, CstParseError, DartCstDriver, GenericPolyglotDriver, GoCstDriver,
    GreenElement, GreenNode, GreenNodeBuilder, GreenToken, GreenTreeRoot, InvariantBitset,
    JavaCstDriver, LanguageId, PhpCstDriver, PythonCstDriver, RustCstDriver, SyntaxElement,
    SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, TreeStats, TypeScriptCstDriver,
    UniversalCstDriver, UniversalCstDriverExt, UniversalDriverRegistry, GLOBAL_REGISTRY,
};
pub use daemon::{
    FrameType, IpcClient, IpcClientError, IpcCommand, IpcDaemonServer, IpcFrameCodec,
    IpcProtocolError, IpcResponse, IpcServerError,
};
pub use diff::{AstDiffEngine, DiffError};
pub use formal::{
    AstExpression, AstStatement, FormalError, HoareTriple, ProofVerdict, SemanticPredicate,
    SemanticSolver, WeakestPreconditionEngine,
};
pub use graph::SymbolGraph;
pub use guard::{AstGuard, InvariantsExtended, RuleMask, RuleRunner};
pub use hud::{TelemetryCollector, TelemetrySnapshot, TerminalHudRenderer};
pub use intent::{
    AmbiguityEntropyCalculator, AmbiguityScore, AmbiguousTarget, ArchitecturalChoice,
    ArchitecturalPattern, ArchitecturalTasteEngine, ContractDelta, ContractError, ContractId,
    ContractLockState, DynamicAddendumEngine, EdgeCaseDetector, EdgeCaseKind, EdgeCaseViolation,
    EntropyBreakdown, EntropyThresholdConfig, FrozenTransactionState, IntentEntropyReport,
    IntentResolution, ResumeReceipt, SocraticQuestion, SocraticQuestionGenerator, SpecContract,
    SpecLockdownEngine, TasteScore, TradeoffMatrix, TradeoffRating,
};
pub use lease::{LeaseBroker, LeaseRegistry};
pub use mcp::run_stdio_server;
pub use meta_mcp::{
    compute_sha256, DynamicToolId, DynamicToolRegistry, DynamicToolSpec,
    EphemeralToolGarbageCollector, LiveDynamicToolRegistry, MetaToolSynthesizer,
    RegisteredDynamicTool, RegistryError, RegistrySnapshot, SandboxError, SecurityPolicyRule,
    SecurityViolation, SecurityViolationKind, ToolAstAnalyzer, ToolCapability, ToolExecutionResult,
    ToolExecutionStats, ToolLifecycleState, ToolPersistencePolicy, ToolResourceLimits,
    ToolSandboxRunner, ToolSecurityAuditor, ToolSecurityCertificate, ToolSyntheticTestRunner,
    ToolVerificationReport,
};
pub use parser::{AstNode, AstQueryEngine, IncrementalParser, ParseDelta};
pub use proof_verifier::{
    compute_sha256 as compute_proof_sha256, AssertionEvaluator, ProofAssertion, ProofCertificate,
    ProofClaim, ProofVerificationResult, ProofVerifierEngine, PROOF_VERIFIER_VERSION,
};
pub use release::{AuditGateConfig, AuditVerdict, ReleaseAuditGate, ReleaseError};
pub use remediate::{AutoFixer, PatchStrategy};
pub use sarif::{
    get_canonical_rules, lookup_rule, rule_id_for_security_violation, rule_id_for_violation,
    ArtifactContent, ArtifactLocation, CodeFlow, Invocation, Location, LogicalLocation, Message,
    MultiformatMessageString, PhysicalLocation, Region, ReportingConfiguration,
    ReportingDescriptor, SarifError, SarifLevel, SarifLog, SarifReportGenerator, SarifResult,
    SarifRun, ThreadFlow, ThreadFlowLocation, ToolComponent, ToolComponentDriver, SARIF_SCHEMA_URI,
    SARIF_VERSION,
};
pub use scheduler::{
    ComputeBackend, ComputeBudgetManager, ComputeTask, FramePacingMode, FrameReport,
    HeterogeneousDispatcher, SchedulerError, TaskRouteDecision, WorkStealingPool,
};
pub use search::{
    ChunkId, DistanceMetric, FusedSearchResult, HnswIndex, HybridMatcher, HybridQuery,
    HybridSearchCoordinator, HybridSearchStats, LexicalCandidate, LexicalKeywordMatcher,
    MmapStoreConfig, MmapVectorReader, MmapVectorWriter, ReciprocalRankFusionEngine, RrfConfig,
    ScoredVector, SearchFilter, SimdBatchScanner, SimdEngineStats, SimdMathKernel, SymbolDocument,
    SymbolKindFilter, VectorDimension, VectorHeader, VectorId, VectorRecord, VectorStoreError,
};
pub use semantic::{
    all_semantic_rules, AsyncIteratorRule, CalendarArithmeticRule, CollectionKeyRule,
    DateCoercionRule, LocusSemanticAnalyzer, NumericSortRule, SemanticEngine, SemanticProof,
    SemanticReport, SemanticRule, SemanticSarifBridge, SemanticViolation, ViolationLocation,
    ViolationSeverity,
};
pub use slice::{ContextSlicer, IntentSlice};
pub use swarm::{
    AgentId, AgentRole, AgentShadowBuffer, BufferId, ChannelError, ChannelStats, ConsensusStatus,
    DagError, ExecutionDag, MessageId, OccError, OccLockState, OccValidationEngine,
    OccValidationResult, ShadowBufferManager, StagedMutation, SubtreeConflictPartitioner,
    SubtreeSpan, SwarmConsensusAggregator, SwarmConsensusReceipt, SwarmDagPlanner, SwarmMessage,
    SwarmMessageType, SwarmRingChannel, SwarmTaskNode, SwarmWorkspaceManager, TaskCompletionRecord,
    TaskId, TaskPriority, TaskStatus, VersionTag, DEFAULT_RING_CAPACITY, MAX_SWARM_BUFFERS,
};
pub use taint::{DataFlowTracker, NullPropagationTracker};
pub use tx::{ShadowBuffer, WorkspaceTransaction};
pub use types::*;
pub use ui_guard::{
    BoxConstraintBounds, CompiledUiArtifact, ComponentExportStyle, DesignTokens, FlutterLayoutSolver,
    FlutterUiCompiler, LayoutDirection, LayoutFlex, LayoutViolation, LayoutViolationKind,
    MockupApprovalState, MockupError, MockupId, MockupRenderExporter, TargetFramework, ThemeMode,
    TsxLayoutSolver, TsxUiCompiler, UiCompilationPipeline, UiCompilerConfig, UiCompilerError,
    UiInvariantChecker, UiInvariantReport, VisualElement, VisualLayoutSynthesizer, VisualMockupCard,
    VisualRole,
};
pub use wal::{
    crc32c, BinaryFrameCodec, BinaryWalWriter, CheckpointId, CheckpointIssuer, CheckpointSnapshot,
    CompactorConfig, CompactorError, CompactorPolicy, CompactorStats, FsyncPolicy,
    RecoveredTransaction, RecoveryMode, RecoveryReceipt, ScanResult, ScannedTx,
    TransactionRecoveryStatus, TxId, WalCheckpointPayload, WalConfig, WalCrashRecoveryManager,
    WalError, WalFrame, WalFrameHeader, WalFrameType, WalLogCompactor, WalLogScanner,
    WalMutationPayload, WalReplayEngine, WalVersion,
};
pub use wasm::LocusWasmBridge;
