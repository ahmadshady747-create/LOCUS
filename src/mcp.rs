//! Model Context Protocol (MCP) Server for locus-engine over stdio (JSON-RPC 2.0).

use std::fs;
use std::io::{BufRead, Write};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::diff::AstDiffEngine;
use crate::graph::SymbolGraph;
use crate::guard::AstGuard;
use crate::types::Language;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    #[serde(default)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// Dispatches a single raw JSON-RPC string message and returns the response JSON string if appropriate.
pub fn handle_json_rpc_message(raw_json: &str) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(raw_json) {
        Ok(r) => r,
        Err(err) => {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {
                    "code": -32700,
                    "message": format!("Parse error: {}", err)
                }
            });
            return Some(err_resp.to_string());
        }
    };

    let id = req.id.clone().unwrap_or(Value::Null);
    let is_notification = req.id.is_none();

    match req.method.as_str() {
        "initialize" => {
            let resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "locus-engine",
                        "version": "0.1.0"
                    }
                }
            });
            Some(resp.to_string())
        }

        "notifications/initialized" | "initialized" => {
            if is_notification {
                None
            } else {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                });
                Some(resp.to_string())
            }
        }

        "ping" => {
            let resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            });
            Some(resp.to_string())
        }

        "tools/list" => {
            let tools = json!({
                "tools": [
                    {
                        "name": "check_safety",
                        "description": "Deterministic 6-pass invariant AST safety check (<0.05ms) catching delimiter balance, async mutex across await, div-by-zero, array bounds, unsafe unwraps, and ReDoS.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to the source file to verify on disk"
                                },
                                "code": {
                                    "type": "string",
                                    "description": "Raw code snippet to verify (optional if path is provided)"
                                }
                            }
                        }
                    },
                    {
                        "name": "skeletonize",
                        "description": "Surgically extracts an AST skeleton (type signatures only, stripped bodies) providing >50-80% context token reduction.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Raw source code to skeletonize"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "python"],
                                    "description": "Programming language of the code"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "patch_symbol",
                        "description": "Surgically replaces a named AST symbol with new code within a source file.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "Original source code"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Name of the function, struct, or class to replace"
                                },
                                "new_code": {
                                    "type": "string",
                                    "description": "New implementation for the symbol"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "python"],
                                    "description": "Programming language of the source"
                                }
                            },
                            "required": ["source", "symbol", "new_code"]
                        }
                    },
                    {
                        "name": "index_graph",
                        "description": "Recursively indexes a directory tree into a cross-file SymbolGraph and computes token savings.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Directory path to index"
                                }
                            },
                            "required": ["path"]
                        }
                    }
                ]
            });

            let resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tools
            });
            Some(resp.to_string())
        }

        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let tool_result = execute_tool(tool_name, &arguments);
            let resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result
            });
            Some(resp.to_string())
        }

        _ => {
            if is_notification {
                None
            } else {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", req.method)
                    }
                });
                Some(err_resp.to_string())
            }
        }
    }
}

fn execute_tool(name: &str, args: &Value) -> Value {
    match name {
        "check_safety" => {
            let code = if let Some(c) = args.get("code").and_then(|s| s.as_str()) {
                c.to_string()
            } else if let Some(p) = args.get("path").and_then(|s| s.as_str()) {
                match fs::read_to_string(p) {
                    Ok(c) => c,
                    Err(e) => {
                        return json!({
                            "content": [{"type": "text", "text": format!("Error reading path '{}': {}", p, e)}],
                            "isError": true
                        });
                    }
                }
            } else {
                return json!({
                    "content": [{"type": "text", "text": "Error: Either 'code' or 'path' argument is required"}],
                    "isError": true
                });
            };

            let report = AstGuard::verify(&code);
            let report_json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": report_json
                    }
                ]
            })
        }

        "skeletonize" => {
            let code = match args.get("code").and_then(|s| s.as_str()) {
                Some(c) => c,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'code' argument is required"}],
                        "isError": true
                    });
                }
            };

            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
            let lang = Language::from_extension(lang_str);
            let skeleton = AstDiffEngine::skeletonize(code, lang);

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": skeleton
                    }
                ]
            })
        }

        "patch_symbol" => {
            let source = match args.get("source").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'source' argument is required"}],
                        "isError": true
                    });
                }
            };
            let symbol = match args.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'symbol' argument is required"}],
                        "isError": true
                    });
                }
            };
            let new_code = match args.get("new_code").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'new_code' argument is required"}],
                        "isError": true
                    });
                }
            };

            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
            let lang = Language::from_extension(lang_str);

            match AstDiffEngine::patch(source, symbol, new_code, lang) {
                Ok(patched) => {
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": patched
                            }
                        ]
                    })
                }
                Err(e) => {
                    json!({
                        "content": [{"type": "text", "text": format!("Patch failed: {}", e)}],
                        "isError": true
                    })
                }
            }
        }

        "index_graph" => {
            let path = match args.get("path").and_then(|s| s.as_str()) {
                Some(p) => p,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'path' argument is required"}],
                        "isError": true
                    });
                }
            };

            let graph = SymbolGraph::index_directory(path);
            let summary = json!({
                "indexed_path": path,
                "file_count": graph.file_to_symbols.len(),
                "symbol_count": graph.nodes.len(),
                "edge_count": graph.edges.len(),
                "symbols": graph.nodes.values().map(|n| {
                    json!({
                        "name": n.name,
                        "kind": n.kind.to_string(),
                        "file": n.file,
                        "signature": n.signature
                    })
                }).collect::<Vec<_>>()
            });

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&summary).unwrap_or_default()
                    }
                ]
            })
        }

        _ => {
            json!({
                "content": [{"type": "text", "text": format!("Unknown tool: {}", name)}],
                "isError": true
            })
        }
    }
}

/// Runs the standard input/output MCP JSON-RPC 2.0 loop.
pub fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    let reader = std::io::BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(resp) = handle_json_rpc_message(trimmed) {
            writeln!(stdout_lock, "{}", resp)?;
            stdout_lock.flush()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#;
        let resp = handle_json_rpc_message(init_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["serverInfo"]["name"], "locus-engine");
        assert_eq!(parsed["result"]["serverInfo"]["version"], "0.1.0");
    }

    #[test]
    fn test_mcp_ping() {
        let ping_req = r#"{"jsonrpc":"2.0","id":"ping-42","method":"ping"}"#;
        let resp = handle_json_rpc_message(ping_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["id"], "ping-42");
        assert_eq!(parsed["result"], json!({}));
    }

    #[test]
    fn test_mcp_tools_list() {
        let list_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let resp = handle_json_rpc_message(list_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let tools = parsed["result"]["tools"].as_array().expect("Tools array");
        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"check_safety"));
        assert!(names.contains(&"skeletonize"));
        assert!(names.contains(&"patch_symbol"));
        assert!(names.contains(&"index_graph"));
    }

    #[test]
    fn test_mcp_tool_call_check_safety_pass() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "check_safety",
                "arguments": {
                    "code": "pub fn add(a: i32, b: i32) -> i32 { a + b }"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"passed\": true"));
    }

    #[test]
    fn test_mcp_tool_call_skeletonize() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "skeletonize",
                "arguments": {
                    "code": "pub fn heavy_task() -> u64 {\n    let mut x = 0;\n    x + 1\n}",
                    "language": "rust"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("pub fn heavy_task() -> u64;"));
    }

    #[test]
    fn test_mcp_tool_call_patch_symbol() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "patch_symbol",
                "arguments": {
                    "source": "pub fn hello() -> &'static str { \"old\" }",
                    "symbol": "hello",
                    "new_code": "pub fn hello() -> &'static str { \"new\" }",
                    "language": "rust"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"new\""));
    }
}
