//! Dynamic Tool Hot-Reloading & Ephemeral Registry Lifecycle Manager.
//!
//! Provides live-memory MCP tool registration, atomic hot-reloading,
//! capability-guarded execution dispatch, and automated garbage collection in 100% safe Rust.

#![forbid(unsafe_code)]

pub mod garbage_collector;
pub mod live_registry;
pub mod types;

pub use garbage_collector::EphemeralToolGarbageCollector;
pub use live_registry::LiveDynamicToolRegistry;
pub use types::{
    DynamicToolId, DynamicToolSpec, RegisteredDynamicTool, RegistryError, RegistrySnapshot,
    ToolExecutionStats, ToolLifecycleState, ToolPersistencePolicy,
};

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_mcp::sandbox::tool_synthesizer::MetaToolSynthesizer;
    use crate::meta_mcp::sandbox::types::ToolCapability;
    use crate::meta_mcp::verifier::types::ToolSecurityCertificate;
    use serde_json::json;
    use std::time::Instant;

    fn create_sample_tool(name: &str, body: &str) -> (DynamicToolSpec, ToolSecurityCertificate) {
        let schema = json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        });

        let spec = MetaToolSynthesizer::synthesize_tool(
            name,
            "Sample dynamic tool",
            schema,
            vec![ToolCapability::ReadWorkspace],
            body,
        )
        .expect("Tool synthesis succeeds");

        let cert = ToolSecurityCertificate::new(
            spec.id.value(),
            body,
            vec![ToolCapability::ReadWorkspace],
            1000,
        );

        (spec, cert)
    }

    #[test]
    fn test_dynamic_tool_registration_and_dispatch() {
        let registry = LiveDynamicToolRegistry::new();
        let (spec, cert) = create_sample_tool("formatter", "pub fn run() -> String { \"formatted\".to_string() }");

        let tool_id = registry
            .register_tool(spec, cert, ToolPersistencePolicy::Permanent)
            .expect("Registration succeeds");

        let args = json!({ "input": "test_string" });
        let result = registry.dispatch_call(tool_id, &args).expect("Dispatch succeeds");

        assert_eq!(result.get("status").and_then(|s| s.as_str()), Some("success"));

        let registered = registry.get_tool(tool_id).expect("Tool must exist");
        assert_eq!(registered.stats.invocations_count, 1);
        assert_eq!(registered.state, ToolLifecycleState::Active);

        let schema = registry.export_mcp_tools_schema();
        assert!(schema.is_array());
        assert_eq!(schema.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_atomic_hot_reloading_zero_downtime() {
        let registry = LiveDynamicToolRegistry::new();

        // Version 1
        let (spec_v1, cert_v1) = create_sample_tool("calculator", "pub fn run() -> i32 { 10 }");
        let id_v1 = registry
            .register_tool(spec_v1, cert_v1, ToolPersistencePolicy::Permanent)
            .expect("V1 registration succeeds");

        let args = json!({ "input": "calc" });
        let res_v1 = registry.dispatch_call(id_v1, &args).expect("V1 dispatch succeeds");
        assert_eq!(res_v1.get("status").and_then(|s| s.as_str()), Some("success"));

        // Version 2: Hot-Reload replacement with same name
        let (spec_v2, cert_v2) = create_sample_tool("calculator", "pub fn run() -> i32 { 20 }");
        let id_v2 = registry
            .register_tool(spec_v2, cert_v2, ToolPersistencePolicy::Permanent)
            .expect("V2 hot-reload succeeds");

        // Dispatches to V2
        let res_v2 = registry.dispatch_call(id_v2, &args).expect("V2 dispatch succeeds");
        assert_eq!(res_v2.get("status").and_then(|s| s.as_str()), Some("success"));

        let fetched_by_name = registry.get_tool_by_name("calculator").expect("Tool must exist");
        assert_eq!(fetched_by_name.spec.id, id_v2);
    }

    #[test]
    fn test_ephemeral_ttl_expiration_and_garbage_collection() {
        let registry = LiveDynamicToolRegistry::new();
        let (spec, cert) = create_sample_tool("ephemeral_task", "pub fn run() {}");

        let tool_id = registry
            .register_tool(spec, cert, ToolPersistencePolicy::EphemeralTtlSeconds(1))
            .expect("Registration succeeds");

        let registered = registry.get_tool(tool_id).unwrap();
        let current_time_ms = registered.registered_at_ms + 2000; // 2 seconds later (TTL is 1s)

        // GC pass
        let pruned = EphemeralToolGarbageCollector::collect_expired_tools(&registry, current_time_ms);
        assert!(pruned.contains(&tool_id));
        assert!(registry.get_tool(tool_id).is_none());
    }

    #[test]
    fn test_single_use_auto_revocation() {
        let registry = LiveDynamicToolRegistry::new();
        let (spec, cert) = create_sample_tool("one_time_token", "pub fn run() {}");

        let tool_id = registry
            .register_tool(spec, cert, ToolPersistencePolicy::SingleUse)
            .expect("Registration succeeds");

        let args = json!({});

        // 1st invocation: Succeeds
        let res1 = registry.dispatch_call(tool_id, &args);
        assert!(res1.is_ok());

        // Tool state should now be Revoked
        let registered = registry.get_tool(tool_id).unwrap();
        assert_eq!(registered.state, ToolLifecycleState::Revoked);

        // 2nd invocation: Fails with ToolRevoked
        let res2 = registry.dispatch_call(tool_id, &args);
        assert_eq!(res2, Err(RegistryError::ToolRevoked(tool_id)));

        // GC cleans it up
        let purged_count = EphemeralToolGarbageCollector::purge_all_revoked(&registry);
        assert_eq!(purged_count, 1);
        assert!(registry.get_tool(tool_id).is_none());
    }

    #[test]
    fn test_sub_0_05ms_dispatch_latency_budget() {
        let registry = LiveDynamicToolRegistry::new();
        let (spec, cert) = create_sample_tool("bench_tool", "pub fn run() {}");

        let tool_id = registry
            .register_tool(spec, cert, ToolPersistencePolicy::Permanent)
            .expect("Registration succeeds");

        let args = json!({ "input": "benchmark" });

        // Warm up
        for _ in 0..10 {
            let _ = registry.dispatch_call(tool_id, &args);
        }

        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = registry.dispatch_call(tool_id, &args);
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average Dynamic Tool Dispatch Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 0.50 } else { 0.05 };
        assert!(
            avg_latency_ms < budget,
            "Dispatch latency must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
