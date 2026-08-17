//! Swappable Core Slots & Plugin Architecture for LOCUS.
//!
//! Provides abstract trait interfaces (`ContextSlot`, `SandboxSlot`), runtime dynamic engine switching,
//! zero-panic local tool runner, Windows Shebang resolver, Circuit Breaker, and lifecycle hook dispatching.

pub mod drivers;
pub mod hooks;
pub mod local_tools;
pub mod registry;
pub mod slots;
pub mod traits;
pub mod types;

pub use drivers::{InMemoryBM25Driver, MockIsolationDriver, NativeProcessDriver, RipgrepDriver};
pub use hooks::{HookDispatcher, HookEvent, HookResult, RegisteredHook};
pub use local_tools::{
    discover_local_tools, parse_script_headers, CircuitBreakerManager, CircuitState,
    LocalToolManifest, LocalToolRunner, PluginError, ShebangResolver, ToolExecutionOutput,
    ToolParameter,
};
pub use registry::{AddonManifest, GitAddonInstaller, InstalledAddon, RegistryError, RegistryStore};
pub use slots::SlotsEngine;
pub use traits::{ContextSlot, SandboxSlot};
pub use types::{
    ContextSearchResult, ExecutionResult, SlotDescriptor, SlotError, SlotType, SlotsConfig,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_slots_engine_defaults() {
        let engine = SlotsEngine::new();
        let config = engine.get_config();

        assert_eq!(config.active_context_driver, "bm25");
        assert_eq!(config.active_sandbox_driver, "native");
        assert_eq!(config.descriptors.len(), 4);
    }

    #[tokio::test]
    async fn test_runtime_context_slot_switching() {
        let engine = SlotsEngine::new();

        let driver1 = engine.get_active_context_driver().expect("Active driver");
        assert_eq!(driver1.driver_id(), "bm25");
        let res1 = driver1.search("struct Token", 5).await.expect("Search");
        assert!(!res1.is_empty());

        let updated_cfg = engine
            .set_active_driver(SlotType::Context, "ripgrep")
            .expect("Switch to ripgrep");
        assert_eq!(updated_cfg.active_context_driver, "ripgrep");

        let driver2 = engine.get_active_context_driver().expect("Active driver");
        assert_eq!(driver2.driver_id(), "ripgrep");
    }

    #[tokio::test]
    async fn test_runtime_sandbox_slot_switching() {
        let engine = SlotsEngine::new();

        engine
            .set_active_driver(SlotType::Sandbox, "mock")
            .expect("Switch to mock");
        let driver = engine.get_active_sandbox_driver().expect("Active driver");
        assert_eq!(driver.driver_id(), "mock");

        let result = driver.execute("echo 'hello'", "").await.expect("Execution");
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("[MOCK ISOLATION DRY-RUN]"));
    }

    #[test]
    fn test_shebang_resolver_windows_and_unix() {
        let script = Path::new("scripts/test_script.py");
        let (prog, args) = ShebangResolver::resolve_interpreter_and_args("#!/usr/bin/env python3", script);
        #[cfg(target_os = "windows")]
        assert_eq!(prog, "python");
        assert!(args.first().unwrap().contains("test_script.py"));

        let node_script = Path::new("tools/build.js");
        let (node_prog, node_args) = ShebangResolver::resolve_interpreter_and_args("#!/usr/bin/env node", node_script);
        #[cfg(target_os = "windows")]
        assert_eq!(node_prog, "node");
        assert!(node_args.first().unwrap().contains("build.js"));
    }

    #[test]
    fn test_script_header_parsing() {
        let content = r#"#!/usr/bin/env python3
# @name: Format SQL Query
# @description: Prettifies raw SQL statements into standard format
# @timeout: 8
# @param: query required The raw SQL string to format
# @param: dialect optional Target database dialect

import sys
print("Formatted SQL")
"#;
        let manifest = parse_script_headers(content, Path::new("format_sql.py"), false);
        assert_eq!(manifest.name, "Format SQL Query");
        assert_eq!(manifest.description, "Prettifies raw SQL statements into standard format");
        assert_eq!(manifest.timeout_secs, 8);
        assert_eq!(manifest.parameters.len(), 2);
        assert_eq!(manifest.parameters[0].name, "query");
        assert!(manifest.parameters[0].required);
        assert_eq!(manifest.parameters[1].name, "dialect");
        assert!(!manifest.parameters[1].required);
    }

    #[test]
    fn test_circuit_breaker_trips_after_3_failures() {
        let cb = CircuitBreakerManager::new();
        let tool_id = "test_flaky_tool";

        assert!(cb.check_allowed(tool_id).is_ok());

        cb.record_failure(tool_id, "Failure 1");
        assert!(cb.check_allowed(tool_id).is_ok());

        cb.record_failure(tool_id, "Failure 2");
        assert!(cb.check_allowed(tool_id).is_ok());

        // 3rd failure trips the circuit (OPEN)
        cb.record_failure(tool_id, "Failure 3");
        let err = cb.check_allowed(tool_id);
        assert!(err.is_err());
        match err.unwrap_err() {
            PluginError::CircuitOpen(id, count) => {
                assert_eq!(id, tool_id);
                assert_eq!(count, 3);
            }
            _ => panic!("Expected CircuitOpen error"),
        }

        // Reset restores permission
        assert!(cb.reset(tool_id));
        assert!(cb.check_allowed(tool_id).is_ok());
    }

    #[tokio::test]
    async fn test_hook_dispatcher_dispatch() {
        let dispatcher = HookDispatcher::new();
        dispatcher.register(HookEvent::PromptReceived, "test_echo", "echo hook_fired");
        dispatcher.dispatch(HookEvent::PromptReceived, "test payload");
        // Dispatch is non-blocking and fire-and-forget
    }
}
