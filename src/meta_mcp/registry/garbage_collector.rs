//! Ephemeral Tool Garbage Collector & Resource Reclamation Engine.
//!
//! Scans the dynamic tool registry, prunes TTL-expired ephemeral tools and
//! exhausted single-use tools, freeing memory allocations in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::meta_mcp::registry::live_registry::LiveDynamicToolRegistry;
use crate::meta_mcp::registry::types::{DynamicToolId, ToolLifecycleState};

/// Autonomous garbage collector for ephemeral and revoked dynamic Meta-MCP tools.
#[derive(Debug, Default, Clone, Copy)]
pub struct EphemeralToolGarbageCollector;

impl EphemeralToolGarbageCollector {
    pub fn new() -> Self {
        Self
    }

    /// Scans registry and purges tools that have exceeded their TTL or are marked Revoked/Expired.
    pub fn collect_expired_tools(
        registry: &LiveDynamicToolRegistry,
        current_timestamp_ms: u64,
    ) -> Vec<DynamicToolId> {
        let tools_lock = registry.tools_ref();
        let name_map_lock = registry.name_map_ref();

        let mut tools = tools_lock.write();
        let mut name_map = name_map_lock.write();

        let mut pruned_ids = Vec::new();

        // Identify tools to prune
        for (&tool_id, tool) in tools.iter() {
            let should_prune = tool.state == ToolLifecycleState::Expired
                || tool.state == ToolLifecycleState::Revoked
                || tool.is_expired(current_timestamp_ms);

            if should_prune {
                pruned_ids.push((tool_id, tool.spec.name.clone()));
            }
        }

        // Prune from collections
        for (id, name) in &pruned_ids {
            tools.remove(id);
            if let Some(&mapped_id) = name_map.get(name) {
                if mapped_id == *id {
                    name_map.remove(name);
                }
            }
        }

        pruned_ids.into_iter().map(|(id, _)| id).collect()
    }

    /// Purges all explicitly revoked tools immediately.
    pub fn purge_all_revoked(registry: &LiveDynamicToolRegistry) -> usize {
        let tools_lock = registry.tools_ref();
        let name_map_lock = registry.name_map_ref();

        let mut tools = tools_lock.write();
        let mut name_map = name_map_lock.write();

        let mut pruned_count = 0;
        let mut to_remove = Vec::new();

        for (&tool_id, tool) in tools.iter() {
            if tool.state == ToolLifecycleState::Revoked {
                to_remove.push((tool_id, tool.spec.name.clone()));
            }
        }

        for (id, name) in to_remove {
            tools.remove(&id);
            name_map.remove(&name);
            pruned_count += 1;
        }

        pruned_count
    }
}
