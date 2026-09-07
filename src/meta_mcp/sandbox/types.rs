//! Data types and models for the Meta-MCP Dynamic Tool Sandbox & Tool Synthesis Engine.
//!
//! Provides capability-based access control, resource limit contracts,
//! dynamic MCP tool specifications, and execution result schemas in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use crate::types::fnv1a_64;

// ---------------------------------------------------------------------------
// Identifiers & Capabilities
// ---------------------------------------------------------------------------

/// Deterministic 64-bit identifier for dynamic MCP tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DynamicToolId(pub u64);

impl DynamicToolId {
    /// Generates a deterministic `DynamicToolId` from tool name and version.
    pub fn new(name: &str, version: u32) -> Self {
        let combined = format!("{}:{}", name, version);
        Self(fnv1a_64(combined.as_bytes()))
    }

    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for DynamicToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mcp_tool_{:016x}", self.0)
    }
}

/// Fine-grained capability tokens for dynamic MCP execution sandboxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCapability {
    /// Read-only access to workspace AST and symbol graphs.
    ReadWorkspace,
    /// Permission to stage mutations into the workspace transaction.
    WriteWorkspace,
    /// Permission to execute polyglot CST/AST transformations.
    ExecuteAstTransform,
    /// Permission to inspect system metrics and memory pressure.
    SystemMetrics,
    /// Restricted, monitored network access token.
    NetworkRestricted,
}

impl fmt::Display for ToolCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ReadWorkspace => "ReadWorkspace",
            Self::WriteWorkspace => "WriteWorkspace",
            Self::ExecuteAstTransform => "ExecuteAstTransform",
            Self::SystemMetrics => "SystemMetrics",
            Self::NetworkRestricted => "NetworkRestricted",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// Resource Limits & Tool Specifications
// ---------------------------------------------------------------------------

/// Strict resource limits enforced per dynamic tool invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResourceLimits {
    /// Maximum allowable memory allocation in bytes (default: 16 MB).
    pub max_memory_bytes: usize,
    /// Maximum execution timeout in milliseconds (default: 500 ms).
    pub timeout_ms: u64,
    /// Maximum recursive or nested tool call depth (default: 8).
    pub max_call_depth: u32,
}

impl Default for ToolResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024, // 16 MB
            timeout_ms: 500,                    // 500 ms
            max_call_depth: 8,
        }
    }
}

/// Complete specification of a dynamically synthesized MCP tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicToolSpec {
    pub id: DynamicToolId,
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub input_schema: serde_json::Value,
    pub required_capabilities: Vec<ToolCapability>,
    pub resource_limits: ToolResourceLimits,
    pub implementation_body: String,
    pub compiled_body: String,
    pub is_verified: bool,
}

impl DynamicToolSpec {
    /// Constructs a new dynamic tool specification.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters_schema: serde_json::Value,
        required_capabilities: Vec<ToolCapability>,
        implementation_body: impl Into<String>,
    ) -> Self {
        let name_str = name.into();
        let id = DynamicToolId::new(&name_str, 1);
        let body_str = implementation_body.into();
        let schema_val = parameters_schema;
        Self {
            id,
            name: name_str,
            description: description.into(),
            parameters_schema: schema_val.clone(),
            input_schema: schema_val,
            required_capabilities,
            resource_limits: ToolResourceLimits::default(),
            implementation_body: body_str.clone(),
            compiled_body: body_str,
            is_verified: false,
        }
    }
}

/// Structured outcome of an isolated tool sandbox execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub tool_id: DynamicToolId,
    pub tool_name: String,
    pub output: serde_json::Value,
    pub execution_time_ms: f64,
    pub memory_consumed_bytes: usize,
}

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxError {
    #[error("Unauthorized capability violation: Tool '{0}' requested ungranted capability '{1}'")]
    UnauthorizedCapability(String, String),

    #[error("Resource limit exceeded: {limit} exceeded (requested: {requested}, max: {max})")]
    ResourceLimitExceeded {
        limit: String,
        requested: usize,
        max: usize,
    },

    #[error("Execution timeout: Tool '{tool_name}' exceeded maximum timeout of {timeout_ms}ms")]
    ExecutionTimeout {
        tool_name: String,
        timeout_ms: u64,
    },

    #[error("Invalid JSON Schema for parameters: {0}")]
    InvalidParametersSchema(String),

    #[error("Tool validation failed: {0}")]
    ToolValidationFailed(String),

    #[error("Dynamic tool not found with ID: {0}")]
    ToolNotFound(DynamicToolId),

    #[error("Unverified tool execution blocked: '{0}' has not passed AST pre-registration safety gate")]
    UnverifiedTool(String),

    #[error("Internal sandbox error: {0}")]
    InternalError(String),
}
