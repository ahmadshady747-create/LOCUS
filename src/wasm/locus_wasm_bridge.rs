//! WebAssembly (WASM) Bridge for Browser IDEs, Edge Environments & Web Extensions.

#![forbid(unsafe_code)]

use crate::diff::AstDiffEngine;
use crate::guard::AstGuard;
use crate::mcp::handle_json_rpc_message;
use crate::parser::AstQueryEngine;
use crate::remediate::AutoFixer;
use crate::slice::ContextSlicer;
use crate::types::Language;
use serde_json::json;

/// WASM-compatible bridge interface for in-browser AST verification.
pub struct LocusWasmBridge;

impl LocusWasmBridge {
    /// Verify source code safety invariants (returns JSON report).
    pub fn verify_code(code: &str) -> String {
        let report = AstGuard::verify(code);
        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Surgically skeletonize source code to compress context (>73% token reduction).
    pub fn skeletonize(code: &str, lang_str: &str) -> String {
        let lang = Language::from_extension(lang_str);
        AstDiffEngine::skeletonize(code, lang)
    }

    /// Extract focused AST context slice around a named symbol.
    pub fn slice_context(code: &str, symbol: &str, lang_str: &str) -> String {
        let lang = Language::from_extension(lang_str);
        let slice = ContextSlicer::slice_from_source(code, symbol, 2, lang);
        serde_json::to_string(&slice).unwrap_or_else(|_| "{}".to_string())
    }

    /// Deterministically remediate broken JSX tags, optional chaining, and conditional hooks.
    pub fn auto_remediate(code: &str) -> String {
        let res = AutoFixer::remediate(code);
        serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string())
    }

    /// Query structural AST patterns via S-expressions.
    pub fn query_ast(pattern: &str, code: &str) -> String {
        let matches = AstQueryEngine::query(pattern, code);
        serde_json::to_string(&matches).unwrap_or_else(|_| "[]".to_string())
    }

    /// Process a raw Model Context Protocol JSON-RPC message directly in-memory.
    pub fn process_mcp_message(raw_json: &str) -> String {
        handle_json_rpc_message(raw_json).unwrap_or_else(|| {
            json!({
                "jsonrpc": "2.0",
                "result": {}
            })
            .to_string()
        })
    }
}
