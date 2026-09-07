//! Live Dynamic Tool Registry & Hot-Reloading Engine.
//!
//! Provides atomic zero-downtime hot-reloading of MCP dynamic tools,
//! capability-guarded execution dispatch, and real-time performance tracking in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::meta_mcp::registry::types::{
    DynamicToolId, DynamicToolSpec, RegisteredDynamicTool, RegistryError, RegistrySnapshot,
    ToolLifecycleState, ToolPersistencePolicy,
};
use crate::meta_mcp::sandbox::sandbox_runner::ToolSandboxRunner;
use crate::meta_mcp::verifier::types::ToolSecurityCertificate;
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Central in-memory registry managing dynamic Meta-MCP tools and hot-reloading lifecycle.
#[derive(Debug, Clone)]
pub struct LiveDynamicToolRegistry {
    tools: Arc<RwLock<HashMap<DynamicToolId, RegisteredDynamicTool>>>,
    name_to_id: Arc<RwLock<HashMap<String, DynamicToolId>>>,
    version: Arc<AtomicU64>,
}

impl Default for LiveDynamicToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveDynamicToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            name_to_id: Arc::new(RwLock::new(HashMap::new())),
            version: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Registers or hot-reloads a verified dynamic tool in the live registry.
    pub fn register_tool(
        &self,
        spec: DynamicToolSpec,
        certificate: ToolSecurityCertificate,
        persistence: ToolPersistencePolicy,
    ) -> Result<DynamicToolId, RegistryError> {
        // 1. Verify certificate match
        if certificate.tool_id != spec.id.value() {
            return Err(RegistryError::InvalidCertificate(
                spec.id,
                format!(
                    "Certificate tool ID ({}) does not match spec ID ({})",
                    certificate.tool_id,
                    spec.id.value()
                ),
            ));
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let tool_id = spec.id;
        let tool_name = spec.name.clone();
        let registered_tool = RegisteredDynamicTool::new(spec, certificate, persistence, now_ms);

        let mut tools = self.tools.write();
        let mut name_map = self.name_to_id.write();

        // 2. Atomic Hot-Reload: If a tool with the same name exists, replace the mapping seamlessly
        if let Some(&old_id) = name_map.get(&tool_name) {
            if old_id != tool_id {
                tools.remove(&old_id);
            }
        }

        tools.insert(tool_id, registered_tool);
        name_map.insert(tool_name, tool_id);
        self.version.fetch_add(1, Ordering::SeqCst);

        Ok(tool_id)
    }

    /// Dispatches a JSON-RPC call to a registered tool, enforcing capability checks and updating stats.
    pub fn dispatch_call(
        &self,
        tool_id: DynamicToolId,
        args_json: &serde_json::Value,
    ) -> Result<serde_json::Value, RegistryError> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let start = Instant::now();

        // 1. Fetch tool spec and verify active status
        let (spec, granted_caps, persistence) = {
            let mut tools = self.tools.write();
            let tool = tools.get_mut(&tool_id).ok_or(RegistryError::ToolNotFound(tool_id))?;

            // Check revocation
            if tool.state == ToolLifecycleState::Revoked {
                return Err(RegistryError::ToolRevoked(tool_id));
            }

            // Check TTL expiration
            if tool.is_expired(now_ms) {
                tool.state = ToolLifecycleState::Expired;
                return Err(RegistryError::ToolExpired(tool_id));
            }

            if tool.state != ToolLifecycleState::Active {
                return Err(RegistryError::ToolNotFound(tool_id));
            }

            (
                tool.spec.clone(),
                tool.certificate.granted_capabilities.clone(),
                tool.persistence,
            )
        };

        // 2. Execute isolated in Sandbox
        let exec_result = ToolSandboxRunner::execute_isolated(&spec, args_json, &granted_caps);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let is_success = exec_result.is_ok();

        // 3. Update execution statistics & single-use lifecycle state
        {
            let mut tools = self.tools.write();
            if let Some(tool) = tools.get_mut(&tool_id) {
                tool.stats.record_invocation(elapsed_ms, is_success, now_ms);

                // Auto-revoke if SingleUse
                if is_success && persistence == ToolPersistencePolicy::SingleUse {
                    tool.state = ToolLifecycleState::Revoked;
                }
            }
        }

        match exec_result {
            Ok(res) => Ok(res.output),
            Err(e) => Err(RegistryError::SandboxFailure(e)),
        }
    }

    /// Revokes an active tool by ID.
    pub fn revoke_tool(&self, tool_id: DynamicToolId) -> Result<(), RegistryError> {
        let mut tools = self.tools.write();
        let tool = tools.get_mut(&tool_id).ok_or(RegistryError::ToolNotFound(tool_id))?;
        tool.state = ToolLifecycleState::Revoked;
        self.version.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Fetches a registered tool by its identifier.
    pub fn get_tool(&self, tool_id: DynamicToolId) -> Option<RegisteredDynamicTool> {
        self.tools.read().get(&tool_id).cloned()
    }

    /// Fetches a registered tool by its name.
    pub fn get_tool_by_name(&self, name: &str) -> Option<RegisteredDynamicTool> {
        let name_map = self.name_to_id.read();
        let tool_id = name_map.get(name)?;
        self.tools.read().get(tool_id).cloned()
    }

    /// Exports standard MCP tool list schema definition.
    pub fn export_mcp_tools_schema(&self) -> serde_json::Value {
        let tools = self.tools.read();
        let mut list = Vec::new();

        for tool in tools.values() {
            if tool.state == ToolLifecycleState::Active {
                list.push(json!({
                    "name": tool.spec.name,
                    "description": tool.spec.description,
                    "inputSchema": tool.spec.parameters_schema,
                }));
            }
        }

        json!(list)
    }

    /// Takes a point-in-time snapshot of active dynamic tools.
    pub fn snapshot(&self) -> RegistrySnapshot {
        let tools = self.tools.read();
        let mut metadata = Vec::with_capacity(tools.len());
        let mut active_count = 0;

        for tool in tools.values() {
            if tool.state == ToolLifecycleState::Active {
                active_count += 1;
            }
            metadata.push(json!({
                "id": tool.spec.id.0,
                "name": tool.spec.name,
                "state": tool.state.to_string(),
                "persistence": format!("{:?}", tool.persistence),
                "invocations": tool.stats.invocations_count,
                "avg_latency_ms": tool.stats.average_latency_ms,
            }));
        }

        RegistrySnapshot {
            total_active_tools: active_count,
            total_registered_tools: tools.len(),
            tools_metadata: metadata,
            snapshot_version: self.version.load(Ordering::Relaxed),
        }
    }

    /// Internal access to raw tools container for garbage collection.
    pub(crate) fn tools_ref(&self) -> Arc<RwLock<HashMap<DynamicToolId, RegisteredDynamicTool>>> {
        self.tools.clone()
    }

    /// Internal access to name map for garbage collection.
    pub(crate) fn name_map_ref(&self) -> Arc<RwLock<HashMap<String, DynamicToolId>>> {
        self.name_to_id.clone()
    }
}
