pub mod config;
pub mod diagnostics;
pub mod error;
pub mod types;
pub mod github_auth;
pub mod ambient;
pub mod airgap_sync;
pub mod terminal_catch;
pub mod compiler_probe;
pub mod contracts;
pub mod intent;
pub mod injector;
pub mod verifier_bridge;
pub mod chaos_simulator;

pub use config::LocusConfig;
pub use diagnostics::{
    sanitize_text, collect_system_info, format_summary,
    DiagnosticSnapshot, SystemInfo, WorkspaceInfo, AiEngineInfo,
    MeshInfo, AgentsInfo, SanitizedLogEntry,
};
pub use error::{LocusError, Result};
pub use types::*;
pub use github_auth::{
    DeviceCodeResponse, DeviceFlowPollStatus, GitHubAuthClient, GitHubAuthStatus, GitHubRepo,
    GitHubUser, DEFAULT_GITHUB_CLIENT_ID,
};
pub use ambient::{
    ActiveWindowContext, AmbientController, AmbientEngine, AmbientSnapshot, AmbientState,
    AmbientTelemetry, AppCategory,
};
pub use airgap_sync::{
    AirGapError, AirGapExporter, AirGapIngestProgress, AirGapReceiver, SyncChunk, SyncPayload,
};
pub use terminal_catch::{
    process_terminal_failure, strip_ansi, DiagnosticLocation, TerminalFailureReport,
};
pub use compiler_probe::{
    parse_cargo_json, parse_ruff_json, parse_tsc_output, CompilerProbeEngine, DiagnosticItem,
    DiagnosticSeverity, DiagnosticStore,
};
pub use contracts::{
    CodeContract, ConstraintExpression, Counterexample, VerificationVerdict,
};
pub use intent::OmniIntent;
pub use injector::{InjectionReport, SafeTextInjector};
pub use verifier_bridge::{QuickVerifierBridge, QuickVerifyReport};
pub use chaos_simulator::{ChaosSimulationReport, ScenarioMetrics};