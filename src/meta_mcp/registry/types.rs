//! Data types and models for Dynamic Tool Registry and Ephemeral Lifecycle Governance.
//!
//! Enforces granular lifecycle states, persistence policies, execution statistics,
//! and registry snapshots in 100% pure safe Rust.

#![forbid(unsafe_code)]

pub use crate::meta_mcp::sandbox::types::{DynamicToolId, DynamicToolSpec, SandboxError};
use crate::meta_mcp::verifier::types::ToolSecurityCertificate;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Lifecycle States & Policies
// ---------------------------------------------------------------------------

/// Runtime lifecycle state of a dynamic Meta-MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolLifecycleState {
    /// Active and available for immediate JSON-RPC invocation.
    Active,
    /// Dormant (temporarily paused from execution dispatch).
    Dormant,
    /// Expired due to TTL timeout.
    Expired,
    /// Revoked due to single-use exhaustion, security policy violation, or explicit revocation.
    Revoked,
}

impl fmt::Display for ToolLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "ACTIVE"),
            Self::Dormant => write!(f, "DORMANT"),
            Self::Expired => write!(f, "EXPIRED"),
            Self::Revoked => write!(f, "REVOKED"),
        }
    }
}

/// Retention and garbage collection policy for dynamic tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolPersistencePolicy {
    /// Permanent tool retained until explicitly revoked.
    Permanent,
    /// Ephemeral tool with a time-to-live expiration window in seconds.
    EphemeralTtlSeconds(u64),
    /// Single-use tool automatically revoked upon first successful execution.
    SingleUse,
}

impl Default for ToolPersistencePolicy {
    fn default() -> Self {
        Self::Permanent
    }
}

// ---------------------------------------------------------------------------
// Tool Execution Statistics
// ---------------------------------------------------------------------------

/// Real-time performance and reliability metrics for a registered tool.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolExecutionStats {
    pub invocations_count: u64,
    pub total_latency_ms: f64,
    pub average_latency_ms: f64,
    pub failed_invocations: u64,
    pub last_invoked_at_ms: u64,
}

impl ToolExecutionStats {
    /// Records a new execution metric sample.
    pub fn record_invocation(&mut self, latency_ms: f64, success: bool, timestamp_ms: u64) {
        self.invocations_count += 1;
        self.total_latency_ms += latency_ms;
        self.average_latency_ms = self.total_latency_ms / (self.invocations_count as f64);
        self.last_invoked_at_ms = timestamp_ms;
        if !success {
            self.failed_invocations += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Registered Dynamic Tool
// ---------------------------------------------------------------------------

/// Complete in-memory registration container for an active dynamic MCP tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisteredDynamicTool {
    pub spec: DynamicToolSpec,
    pub certificate: ToolSecurityCertificate,
    pub persistence: ToolPersistencePolicy,
    pub state: ToolLifecycleState,
    pub stats: ToolExecutionStats,
    pub registered_at_ms: u64,
}

impl RegisteredDynamicTool {
    pub fn new(
        spec: DynamicToolSpec,
        certificate: ToolSecurityCertificate,
        persistence: ToolPersistencePolicy,
        registered_at_ms: u64,
    ) -> Self {
        Self {
            spec,
            certificate,
            persistence,
            state: ToolLifecycleState::Active,
            stats: ToolExecutionStats::default(),
            registered_at_ms,
        }
    }

    /// Returns true if the tool has exceeded its configured TTL.
    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        if let ToolPersistencePolicy::EphemeralTtlSeconds(ttl_secs) = self.persistence {
            let elapsed_ms = current_time_ms.saturating_sub(self.registered_at_ms);
            elapsed_ms > (ttl_secs * 1000)
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Registry Snapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of the dynamic tool registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub total_active_tools: usize,
    pub total_registered_tools: usize,
    pub tools_metadata: Vec<serde_json::Value>,
    pub snapshot_version: u64,
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegistryError {
    #[error("Dynamic tool with ID '{0}' not found in registry")]
    ToolNotFound(DynamicToolId),

    #[error("Dynamic tool '{0}' has been revoked")]
    ToolRevoked(DynamicToolId),

    #[error("Dynamic tool '{0}' has expired due to TTL timeout")]
    ToolExpired(DynamicToolId),

    #[error("Invalid security certificate for tool '{0}': {1}")]
    InvalidCertificate(DynamicToolId, String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Sandbox error during execution: {0}")]
    SandboxFailure(#[from] SandboxError),

    #[error("Internal registry error: {0}")]
    InternalError(String),
}
