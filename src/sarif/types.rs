//! OASIS SARIF v2.1.0 Data Models and Type Definitions.
//!
//! Provides 100% specification-compliant SARIF structures, severity levels,
//! physical/logical locations, code flow traces, and error types in 100% safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub const SARIF_SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/cos02/schemas/sarif-schema-2.1.0.json";
pub const SARIF_VERSION: &str = "2.1.0";

// ---------------------------------------------------------------------------
// SARIF Severity Levels & Messages
// ---------------------------------------------------------------------------

/// Severity level for a SARIF result or rule configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SarifLevel {
    Error,
    Warning,
    Note,
    None,
}

impl fmt::Display for SarifLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Note => write!(f, "note"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Formatted message object within SARIF records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

impl Message {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            markdown: None,
        }
    }

    pub fn with_markdown(text: impl Into<String>, markdown: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            markdown: Some(markdown.into()),
        }
    }
}

/// Multiformat message string used in rule definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiformatMessageString {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

impl MultiformatMessageString {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            markdown: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool & Rule Descriptors
// ---------------------------------------------------------------------------

/// Default configuration for a rule descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportingConfiguration {
    pub level: SarifLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<f32>,
}

/// Metadata describing a deterministic inspection or verification rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingDescriptor {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<MultiformatMessageString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_description: Option<MultiformatMessageString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_configuration: Option<ReportingConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

/// Driver details of the analysis tool (e.g. locus-engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolComponentDriver {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
    pub rules: Vec<ReportingDescriptor>,
}

/// Tool container referencing driver and extension components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolComponent {
    pub driver: ToolComponentDriver,
}

// ---------------------------------------------------------------------------
// Locations & Code Spans
// ---------------------------------------------------------------------------

/// Artifact URI reference for a file in the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLocation {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri_base_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

impl ArtifactLocation {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            uri_base_id: None,
            index: None,
        }
    }
}

/// Snippet content within a code region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactContent {
    pub text: String,
}

/// Text region spanning lines, columns, and character offsets.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<ArtifactContent>,
}

impl Region {
    pub fn from_line_col(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line: Some(start_line),
            start_column: Some(start_col),
            end_line: Some(end_line),
            end_column: Some(end_col),
            char_offset: None,
            char_length: None,
            snippet: None,
        }
    }

    pub fn from_span(char_offset: usize, char_length: usize, snippet: Option<String>) -> Self {
        Self {
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
            char_offset: Some(char_offset),
            char_length: Some(char_length),
            snippet: snippet.map(|s| ArtifactContent { text: s }),
        }
    }
}

/// Physical file and region location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_region: Option<Region>,
}

/// Logical construct identifier (e.g. function or struct name).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalLocation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fully_qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Full location container with physical and logical metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_location: Option<PhysicalLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_locations: Option<Vec<LogicalLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

impl Location {
    pub fn from_physical(phys: PhysicalLocation) -> Self {
        Self {
            physical_location: Some(phys),
            logical_locations: None,
            message: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Code Flow & Taint Paths
// ---------------------------------------------------------------------------

/// Step in an execution trace or taint path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadFlowLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nesting_level: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_order: Option<usize>,
}

/// Ordered sequence of thread flow locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadFlow {
    pub locations: Vec<ThreadFlowLocation>,
}

/// Cross-function or cross-file execution path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeFlow {
    pub thread_flows: Vec<ThreadFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

// ---------------------------------------------------------------------------
// SARIF Results & Run Container
// ---------------------------------------------------------------------------

/// Single finding or violation recorded by locus-engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_index: Option<usize>,
    pub level: SarifLevel,
    pub message: Message,
    pub locations: Vec<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_flows: Option<Vec<CodeFlow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_locations: Option<Vec<Location>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

/// Execution telemetry of an analysis run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    pub execution_successful: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_utc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_utc: Option<String>,
}

/// A complete inspection run containing tool metadata and result set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRun {
    pub tool: ToolComponent,
    pub results: Vec<SarifResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocations: Option<Vec<Invocation>>,
}

/// Top-level OASIS SARIF v2.1.0 document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

impl Default for SarifLog {
    fn default() -> Self {
        Self {
            schema: SARIF_SCHEMA_URI.to_string(),
            version: SARIF_VERSION.to_string(),
            runs: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Error Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum SarifError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid rule definition for ID '{0}': {1}")]
    InvalidRule(String, String),

    #[error("Location calculation error for file '{0}': {1}")]
    InvalidLocation(String, String),

    #[error("I/O error during SARIF export: {0}")]
    IoError(String),
}
