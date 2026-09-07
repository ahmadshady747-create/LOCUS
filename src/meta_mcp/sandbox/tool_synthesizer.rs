//! Meta-Tool Synthesizer & Dynamic Tool Registry.
//!
//! Synthesizes MCP tools on-the-fly, validates JSON parameter schemas,
//! executes AST safety pre-registration gates, and manages the active dynamic tool registry.

#![forbid(unsafe_code)]

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::guard::AstGuard;
use crate::meta_mcp::sandbox::types::{
    DynamicToolId, DynamicToolSpec, SandboxError, ToolCapability,
};

pub struct MetaToolSynthesizer;

impl MetaToolSynthesizer {
    /// Synthesizes a new dynamic MCP tool with JSON Schema validation and pre-registration AST safety checks.
    pub fn synthesize_tool(
        name: &str,
        description: &str,
        parameters_schema: serde_json::Value,
        required_capabilities: Vec<ToolCapability>,
        implementation_body: &str,
    ) -> Result<DynamicToolSpec, SandboxError> {
        // 1. Validate JSON Parameter Schema
        if !parameters_schema.is_object() {
            return Err(SandboxError::InvalidParametersSchema(
                "Parameter schema must be a valid JSON Schema object".to_string(),
            ));
        }

        // 2. Pre-Registration AST Safety Gate
        // Verify that the dynamic implementation body contains zero unsafe blocks and zero delimiter imbalances
        let safety_report = AstGuard::verify(implementation_body);
        if !safety_report.passed {
            return Err(SandboxError::ToolValidationFailed(format!(
                "AST Invariant verification rejected tool body with {} violations: {}",
                safety_report.violations.len(),
                safety_report.violations.join("; ")
            )));
        }

        let mut spec = DynamicToolSpec::new(
            name,
            description,
            parameters_schema,
            required_capabilities,
            implementation_body,
        );

        spec.is_verified = true;

        Ok(spec)
    }
}

/// Thread-safe registry for dynamically synthesized MCP tools.
#[derive(Debug, Default)]
pub struct DynamicToolRegistry {
    tools: RwLock<HashMap<DynamicToolId, DynamicToolSpec>>,
    name_index: RwLock<HashMap<String, DynamicToolId>>,
}

impl DynamicToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a verified dynamic tool into the central registry.
    pub fn register_tool(&self, spec: DynamicToolSpec) -> Result<DynamicToolId, SandboxError> {
        if !spec.is_verified {
            return Err(SandboxError::UnverifiedTool(spec.name.clone()));
        }

        let tool_id = spec.id;
        let tool_name = spec.name.clone();

        let mut tools_lock = self.tools.write();
        let mut name_lock = self.name_index.write();

        name_lock.insert(tool_name, tool_id);
        tools_lock.insert(tool_id, spec);

        Ok(tool_id)
    }

    /// Retrieves a dynamic tool specification by its ID.
    pub fn get_tool(&self, id: DynamicToolId) -> Option<DynamicToolSpec> {
        self.tools.read().get(&id).cloned()
    }

    /// Retrieves a dynamic tool specification by its name.
    pub fn get_tool_by_name(&self, name: &str) -> Option<DynamicToolSpec> {
        let name_lock = self.name_index.read();
        let tool_id = name_lock.get(name)?;
        self.tools.read().get(tool_id).cloned()
    }

    /// Lists all currently registered dynamic tools.
    pub fn list_tools(&self) -> Vec<DynamicToolSpec> {
        self.tools.read().values().cloned().collect()
    }

    /// Returns the total count of registered dynamic tools.
    pub fn total_registered(&self) -> usize {
        self.tools.read().len()
    }
}
