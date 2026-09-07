//! Meta-MCP Subsystem — Dynamic Runtime Tool Synthesis, Sandbox Isolation & Capability Gate.

#![forbid(unsafe_code)]

pub mod registry;
pub mod sandbox;
pub mod verifier;

pub use registry::{
    EphemeralToolGarbageCollector, LiveDynamicToolRegistry, RegisteredDynamicTool, RegistryError,
    RegistrySnapshot, ToolExecutionStats, ToolLifecycleState, ToolPersistencePolicy,
};
pub use sandbox::{
    DynamicToolId, DynamicToolRegistry, DynamicToolSpec, MetaToolSynthesizer, SandboxError,
    ToolCapability, ToolExecutionResult, ToolResourceLimits, ToolSandboxRunner,
};
pub use verifier::{
    compute_sha256, SecurityPolicyRule, SecurityViolation, SecurityViolationKind,
    ToolAstAnalyzer, ToolSecurityAuditor, ToolSecurityCertificate, ToolSyntheticTestRunner,
    ToolVerificationReport,
};
