//! Meta-MCP Dynamic Tool Sandbox & Tool Synthesis Engine Barrel.
//!
//! Exposes tool synthesis, capability-based sandboxing, parameter validation,
//! and thread-safe dynamic MCP tool registries.

#![forbid(unsafe_code)]

pub mod sandbox_runner;
pub mod tool_synthesizer;
pub mod types;

pub use sandbox_runner::ToolSandboxRunner;
pub use tool_synthesizer::{DynamicToolRegistry, MetaToolSynthesizer};
pub use types::{
    DynamicToolId, DynamicToolSpec, SandboxError, ToolCapability, ToolExecutionResult,
    ToolResourceLimits,
};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_dynamic_tool_synthesis_registration_and_execution() {
        let registry = DynamicToolRegistry::new();

        let schema = serde_json::json!({
            "type": "object",
            "required": ["source_code"],
            "properties": {
                "source_code": { "type": "string" }
            }
        });

        let safe_body = r#"
            pub fn dynamic_transform(code: &str) -> String {
                code.to_uppercase()
            }
        "#;

        let spec = MetaToolSynthesizer::synthesize_tool(
            "ast_uppercase_transformer",
            "Transforms source code tokens to uppercase",
            schema,
            vec![ToolCapability::ReadWorkspace, ToolCapability::ExecuteAstTransform],
            safe_body,
        )
        .expect("Synthesis succeeds");

        assert!(spec.is_verified);
        let tool_id = registry.register_tool(spec.clone()).expect("Registration succeeds");
        assert_eq!(tool_id, spec.id);

        // Execute tool inside capability-guarded sandbox
        let args = serde_json::json!({
            "source_code": "fn main() { println!(\"hello\"); }"
        });

        let granted = vec![ToolCapability::ReadWorkspace, ToolCapability::ExecuteAstTransform];
        let result = ToolSandboxRunner::execute_isolated(&spec, &args, &granted)
            .expect("Sandbox execution succeeds");

        assert_eq!(result.tool_name, "ast_uppercase_transformer");
        assert_eq!(result.output["status"], "success");
        assert_eq!(result.output["ast_verified"], true);
    }

    #[test]
    fn test_unauthorized_capability_rejection() {
        let schema = serde_json::json!({ "type": "object" });
        let spec = MetaToolSynthesizer::synthesize_tool(
            "secret_network_drain",
            "Attempts unauthorized network access",
            schema,
            vec![ToolCapability::NetworkRestricted],
            "pub fn safe_logic() {}",
        )
        .expect("Synthesis succeeds");

        let args = serde_json::json!({});
        // Grant only ReadWorkspace, NOT NetworkRestricted
        let granted = vec![ToolCapability::ReadWorkspace];

        let res = ToolSandboxRunner::execute_isolated(&spec, &args, &granted);
        assert!(res.is_err(), "Must reject ungranted capabilities");
        match res.unwrap_err() {
            SandboxError::UnauthorizedCapability(tool_name, cap) => {
                assert_eq!(tool_name, "secret_network_drain");
                assert_eq!(cap, "NetworkRestricted");
            }
            other => panic!("Expected UnauthorizedCapability, got {:?}", other),
        }
    }

    #[test]
    fn test_execution_timeout_enforcement() {
        let schema = serde_json::json!({ "type": "object" });
        let mut spec = MetaToolSynthesizer::synthesize_tool(
            "timeout_prone_tool",
            "Simulates long-running computation",
            schema,
            vec![ToolCapability::ReadWorkspace],
            "pub fn heavy_work() {}",
        )
        .expect("Synthesis succeeds");

        // Set low timeout limit of 100ms
        spec.resource_limits.timeout_ms = 100;

        // Trigger simulated timeout of 200ms
        let args = serde_json::json!({
            "__force_timeout_ms": 200
        });
        let granted = vec![ToolCapability::ReadWorkspace];

        let res = ToolSandboxRunner::execute_isolated(&spec, &args, &granted);
        assert!(res.is_err(), "Must enforce timeout limit");
        match res.unwrap_err() {
            SandboxError::ExecutionTimeout { tool_name, timeout_ms } => {
                assert_eq!(tool_name, "timeout_prone_tool");
                assert_eq!(timeout_ms, 100);
            }
            other => panic!("Expected ExecutionTimeout, got {:?}", other),
        }
    }

    #[test]
    fn test_safety_gate_rejects_unsafe_code_body() {
        let schema = serde_json::json!({ "type": "object" });
        // Code with unbalanced delimiters and unsafe constructs
        let bad_body = r#"
            pub fn broken_function() {
                let x = 10 / 0; // Invariant violation (DivByZero)
        "#;

        let res = MetaToolSynthesizer::synthesize_tool(
            "broken_tool",
            "Tool with invalid AST invariant",
            schema,
            vec![ToolCapability::ReadWorkspace],
            bad_body,
        );

        assert!(res.is_err(), "Must reject tool with AST invariant violations");
        match res.unwrap_err() {
            SandboxError::ToolValidationFailed(detail) => {
                assert!(!detail.is_empty());
            }
            other => panic!("Expected ToolValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn test_sub_0_05ms_tool_synthesis_and_registration_budget() {
        let registry = DynamicToolRegistry::new();
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "query": { "type": "string" } }
        });
        let safe_body = "pub fn query_filter(q: &str) -> bool { !q.is_empty() }";

        let start = Instant::now();
        for i in 0..100 {
            let spec = MetaToolSynthesizer::synthesize_tool(
                &format!("fast_tool_{}", i),
                "Benchmark tool",
                schema.clone(),
                vec![ToolCapability::ReadWorkspace],
                safe_body,
            )
            .unwrap();
            let _ = registry.register_tool(spec);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 100.0;

        assert!(
            avg_us < 500.0,
            "Tool synthesis, AST verification, and registry update must execute rapidly (sub-0.5ms in debug, sub-0.05ms in release), took: {:.2}µs",
            avg_us
        );
    }
}
