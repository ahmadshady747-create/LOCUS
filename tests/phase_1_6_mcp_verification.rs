//! Phase 1.6 Verification: 28 Sovereign Native MCP Tools & JSON-RPC 2.0 Dispatcher.

use locus_engine::mcp::handle_json_rpc_message;
use serde_json::{json, Value};

#[test]
fn test_mcp_tools_list_contains_all_28_tools() {
    let req = r#"{
        "jsonrpc": "2.0",
        "id": "list-req-1",
        "method": "tools/list"
    }"#;

    let resp_str = handle_json_rpc_message(req).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let tools = resp["result"]["tools"].as_array().expect("Tools array");

    assert_eq!(tools.len(), 28, "Expected exactly 28 registered MCP tools");

    let expected_tools = [
        "check_safety",
        "skeletonize",
        "patch_symbol",
        "index_graph",
        "synthesize_contract",
        "extract_intent_slice",
        "verify_contract",
        "resolve_symbol",
        "get_blast_radius",
        "find_references",
        "prepare_context",
        "verified_patch",
        "begin_tx",
        "stage_tx",
        "commit_tx",
        "rollback_tx",
        "auto_remediate",
        "acquire_symbol_lease",
        "release_symbol_lease",
        "renew_symbol_lease",
        "trace_taint_flow",
        "hybrid_search",
        "query_cst",
        "audit_taint_path",
        "acquire_subtree_lease",
        "verify_occ_token",
        "morph_ast",
        "simd_vector_search",
    ];

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing tool in MCP registry: {}",
            expected
        );
    }
}

#[test]
fn test_mcp_call_query_cst() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": "cst-1",
        "method": "tools/call",
        "params": {
            "name": "query_cst",
            "arguments": {
                "code": "pub fn add(a: i32, b: i32) -> i32 {\n    // lossless comment\n    a + b\n}",
                "language": "rust",
                "offset": 12
            }
        }
    });

    let resp_str = handle_json_rpc_message(&req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    let payload: Value = serde_json::from_str(text).expect("Parsed payload");

    assert_eq!(payload["status"], "success");
    assert_eq!(payload["trivia_roundtrip_intact"], true);
    assert!(payload["token_at_offset"].is_object());
}

#[test]
fn test_mcp_call_audit_taint_path() {
    let code_snippet = r#"
    export async function handleRequest(req, res) {
        const client_target = req.headers["x-user-input"];
        const clean = DOMPurify.sanitize(client_target);
        await db.execute(clean);
    }
    "#;

    let req = json!({
        "jsonrpc": "2.0",
        "id": "taint-1",
        "method": "tools/call",
        "params": {
            "name": "audit_taint_path",
            "arguments": {
                "file_path": "src/auth/handler.ts",
                "code": code_snippet,
                "symbol": "handleRequest"
            }
        }
    });

    let resp_str = handle_json_rpc_message(&req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    let payload: Value = serde_json::from_str(text).expect("Parsed payload");

    assert!(payload.is_array());
    let reports = payload.as_array().unwrap();
    assert!(!reports.is_empty(), "Expected taint report for untrusted input flow");
    assert_eq!(reports[0]["is_sanitized"], true);
    assert!(reports[0]["certificate"].is_object());
}

#[test]
fn test_mcp_call_acquire_subtree_lease() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": "lease-1",
        "method": "tools/call",
        "params": {
            "name": "acquire_subtree_lease",
            "arguments": {
                "pattern": "crate::mcp_tests::*",
                "agent_id": "agent_test_runner",
                "ttl_ms": 30000
            }
        }
    });

    let resp_str = handle_json_rpc_message(&req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");

    assert!(text.contains("Acquired"));
    assert!(text.contains("crate::mcp_tests::*"));
}

#[test]
fn test_mcp_call_verify_occ_token() {
    let fqn = "crate::billing::counter";

    // 1. Verify initial OCC version 1
    let verify_req = json!({
        "jsonrpc": "2.0",
        "id": "occ-1",
        "method": "tools/call",
        "params": {
            "name": "verify_occ_token",
            "arguments": {
                "fqn": fqn,
                "expected_version": 1,
                "commit": false
            }
        }
    });

    let resp_str = handle_json_rpc_message(&verify_req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    assert!(text.contains("\"version\":1"));

    // 2. Commit OCC advancement to version 2
    let commit_req = json!({
        "jsonrpc": "2.0",
        "id": "occ-2",
        "method": "tools/call",
        "params": {
            "name": "verify_occ_token",
            "arguments": {
                "fqn": fqn,
                "expected_version": 1,
                "commit": true
            }
        }
    });

    let commit_resp_str = handle_json_rpc_message(&commit_req.to_string()).expect("Response expected");
    let commit_resp: Value = serde_json::from_str(&commit_resp_str).expect("Valid JSON");
    let commit_text = commit_resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    assert!(commit_text.contains("\"version\":2"));
}

#[test]
fn test_mcp_call_morph_ast() {
    let broken_snippet = r#"
        export function Card() {
            const name = user.profile.address.city;
            return <div className="user"><span>User City</span></div>;
        }
    "#;

    let req = json!({
        "jsonrpc": "2.0",
        "id": "morph-1",
        "method": "tools/call",
        "params": {
            "name": "morph_ast",
            "arguments": {
                "code": broken_snippet
            }
        }
    });

    let resp_str = handle_json_rpc_message(&req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    let payload: Value = serde_json::from_str(text).expect("Parsed payload");

    assert_eq!(payload["success"], true);
    assert_eq!(payload["passed_verification"], true);
    let remediated = payload["remediated_code"].as_str().unwrap();
    assert!(remediated.contains("user?.profile?.address?.city"));
}

#[test]
fn test_mcp_call_simd_vector_search() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": "simd-1",
        "method": "tools/call",
        "params": {
            "name": "simd_vector_search",
            "arguments": {
                "query_text": "authenticate user token",
                "corpus": [
                    {"id": 101, "text": "authenticate user access token"},
                    {"id": 102, "text": "render billing invoice card"}
                ],
                "top_k": 2
            }
        }
    });

    let resp_str = handle_json_rpc_message(&req.to_string()).expect("Response expected");
    let resp: Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    let text = resp["result"]["content"][0]["text"].as_str().expect("Text payload");
    let payload: Value = serde_json::from_str(text).expect("Parsed payload");

    assert_eq!(payload["status"], "success");
    assert_eq!(payload["simd_accelerated"], true);
    let hits = payload["hits"].as_array().expect("Hits array");
    assert!(!hits.is_empty());
    assert_eq!(hits[0]["id"], 101);
}
