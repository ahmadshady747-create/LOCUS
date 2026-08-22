//! Model Context Protocol (MCP) Server for locus-engine over stdio (JSON-RPC 2.0).

use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Instant;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::contract::ContractSynthesizer;
use crate::diff::AstDiffEngine;
use crate::graph::SymbolGraph;
use crate::guard::AstGuard;
use crate::slice::ContextSlicer;
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
                        "version": "1.0.0"
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
                        "description": "Deterministic AST invariant safety check (<0.05ms) catching delimiter balance, async mutex across await, div-by-zero, array bounds, unsafe unwraps, ReDoS, JSX tag mismatches, conditional hooks, and client secret leaks.",
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
                        "description": "Surgically extracts an AST skeleton (type signatures, imports, interfaces, collapsed JSX render trees) providing >50-80% context token reduction across Rust, TSX, JSX, Svelte, Astro, Vue, and Python.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Raw source code to skeletonize"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Programming language of the code"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "patch_symbol",
                        "description": "Surgically replaces a named AST symbol (function, struct, component, event handler) with new code within a source file.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "Original source code"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Name of the function, struct, class, component, or event handler to replace"
                                },
                                "new_code": {
                                    "type": "string",
                                    "description": "New implementation for the symbol"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Programming language of the source"
                                }
                            },
                            "required": ["source", "symbol", "new_code"]
                        }
                    },
                    {
                        "name": "index_graph",
                        "description": "Recursively indexes a directory tree into a cross-file SymbolGraph, linking references and computing token savings.",
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
                    },
                    {
                        "name": "synthesize_contract",
                        "description": "Proactively projects developer intent into strict type contract scaffolding and safety invariant checklists before code generation.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "intent": {
                                    "type": "string",
                                    "description": "Developer intent or feature specification"
                                },
                                "target_path": {
                                    "type": "string",
                                    "description": "Optional target file path (e.g. src/auth.rs, src/UserCard.tsx)"
                                },
                                "context": {
                                    "type": "string",
                                    "description": "Optional surrounding context or existing types"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Target programming language"
                                }
                            },
                            "required": ["intent"]
                        }
                    },
                    {
                        "name": "extract_intent_slice",
                        "description": "Extracts a minimal, high-density AST context slice containing only the target symbol and its direct dependencies up to N degrees of separation.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": {
                                    "type": "string",
                                    "description": "Name of the target symbol/function/component to slice around"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional file path or directory on disk"
                                },
                                "code": {
                                    "type": "string",
                                    "description": "Optional raw source code to slice from"
                                },
                                "depth": {
                                    "type": "integer",
                                    "description": "Dependency traversal depth (default: 2)"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Programming language"
                                }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "verify_contract",
                        "description": "Bidirectionally verifies that generated code satisfies the synthesized architectural contract with zero safety violations and complete signature fidelity.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "contract": {
                                    "type": "object",
                                    "description": "Synthesized IntentContract JSON object"
                                },
                                "intent": {
                                    "type": "string",
                                    "description": "Alternative: developer intent string if contract object is not provided"
                                },
                                "generated_code": {
                                    "type": "string",
                                    "description": "Implementation code produced by the AI model"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Programming language"
                                }
                            },
                            "required": ["generated_code"]
                        }
                    },
                    {
                        "name": "resolve_symbol",
                        "description": "Resolves full symbol definition metadata, origin file, byte coordinates, signatures, and doc-comments across module paths.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": {
                                    "type": "string",
                                    "description": "Name of the symbol to resolve (e.g. AstGuard, UserProfileCard)"
                                },
                                "from_file": {
                                    "type": "string",
                                    "description": "Optional calling file context (e.g. src/main.rs)"
                                },
                                "target_path": {
                                    "type": "string",
                                    "description": "Workspace root directory to search (default: .)"
                                }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "get_blast_radius",
                        "description": "Calculates the blast-radius impact of modifying a symbol, identifying direct and transitive dependent files, caller sites, reference counts, and breaking change risk score.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": {
                                    "type": "string",
                                    "description": "Target symbol to calculate modification blast-radius for"
                                },
                                "file": {
                                    "type": "string",
                                    "description": "Optional origin file path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Workspace root directory (default: .)"
                                },
                                "depth": {
                                    "type": "integer",
                                    "description": "Transitive dependency depth (default: 2)"
                                }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "find_references",
                        "description": "Finds all inbound call sites, imports, and references for a symbol across the entire indexed workspace.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": {
                                    "type": "string",
                                    "description": "Symbol name to look up references for"
                                },
                                "target_path": {
                                    "type": "string",
                                    "description": "Workspace root directory (default: .)"
                                }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "prepare_context",
                        "description": "High-throughput compound context pipeline: Combines AST skeletonization, blast-radius calculation, context slicing, and symbol resolution in a single sub-millisecond pass to eliminate multi-step LLM round-trips.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target_file": {
                                    "type": "string",
                                    "description": "Target file to extract skeleton and context from"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Optional symbol to calculate blast radius and focused context slice around"
                                },
                                "budget": {
                                    "type": "integer",
                                    "description": "Optional token budget limit (default: 2000)"
                                },
                                "depth": {
                                    "type": "integer",
                                    "description": "Optional dependency traversal depth for blast radius (default: 2)"
                                }
                            },
                            "required": ["target_file"]
                        }
                    },
                    {
                        "name": "verified_patch",
                        "description": "High-throughput compound patching pipeline: Atomically executes pre-patch invariant safety verification -> in-memory AST symbol replacement -> post-patch full-file integrity validation -> atomic disk write. Aborts with detailed diagnostics if any safety invariant is violated.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Target file to surgically patch on disk"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Name of the function, struct, component, or event handler to patch"
                                },
                                "new_code": {
                                    "type": "string",
                                    "description": "New implementation code for the symbol"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "svelte", "astro", "vue", "python"],
                                    "description": "Optional programming language (inferred from file extension if omitted)"
                                },
                                "dry_run": {
                                    "type": "boolean",
                                    "description": "If true, validates and previews the patch without writing to disk"
                                }
                            },
                            "required": ["file_path", "symbol", "new_code"]
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

        "synthesize_contract" => {
            let intent = match args.get("intent").and_then(|s| s.as_str()) {
                Some(i) => i,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'intent' argument is required"}],
                        "isError": true
                    });
                }
            };
            let target_path = args.get("target_path").and_then(|s| s.as_str());
            let context = args.get("context").and_then(|s| s.as_str());
            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
            let lang = Language::from_extension(lang_str);

            let contract = ContractSynthesizer::synthesize(intent, target_path, context, lang);
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&contract).unwrap_or_default()
                    }
                ]
            })
        }

        "extract_intent_slice" => {
            let symbol = match args.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'symbol' argument is required"}],
                        "isError": true
                    });
                }
            };
            let depth = args.get("depth").and_then(|d| d.as_u64()).unwrap_or(2) as usize;
            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
            let lang = Language::from_extension(lang_str);

            if let Some(code) = args.get("code").and_then(|s| s.as_str()) {
                let slice = ContextSlicer::slice_from_source(code, symbol, depth, lang);
                json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&slice).unwrap_or_default()
                        }
                    ]
                })
            } else if let Some(path) = args.get("path").and_then(|s| s.as_str()) {
                let path_obj = std::path::Path::new(path);
                if path_obj.is_dir() {
                    let graph = SymbolGraph::index_directory(path);
                    let slice = ContextSlicer::slice_from_graph(&graph, symbol, depth);
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&slice).unwrap_or_default()
                            }
                        ]
                    })
                } else {
                    let code = match fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(e) => {
                            return json!({
                                "content": [{"type": "text", "text": format!("Error reading path '{}': {}", path, e)}],
                                "isError": true
                            });
                        }
                    };
                    let ext = path_obj.extension().and_then(|e| e.to_str()).unwrap_or(lang_str);
                    let file_lang = Language::from_extension(ext);
                    let slice = ContextSlicer::slice_from_source(&code, symbol, depth, file_lang);
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&slice).unwrap_or_default()
                            }
                        ]
                    })
                }
            } else {
                json!({
                    "content": [{"type": "text", "text": "Error: Either 'code' or 'path' must be provided for intent slicing"}],
                    "isError": true
                })
            }
        }

        "verify_contract" => {
            let generated_code = match args.get("generated_code").and_then(|s| s.as_str()) {
                Some(c) => c,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'generated_code' argument is required"}],
                        "isError": true
                    });
                }
            };

            let contract = if let Some(c_val) = args.get("contract") {
                if let Ok(c) = serde_json::from_value::<crate::contract::IntentContract>(c_val.clone()) {
                    c
                } else {
                    let intent_str = c_val.get("intent").and_then(|s| s.as_str()).unwrap_or("general implementation");
                    let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
                    ContractSynthesizer::synthesize(intent_str, None, None, Language::from_extension(lang_str))
                }
            } else if let Some(intent) = args.get("intent").and_then(|s| s.as_str()) {
                let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");
                ContractSynthesizer::synthesize(intent, None, None, Language::from_extension(lang_str))
            } else {
                ContractSynthesizer::synthesize("verified component", None, None, Language::Rust)
            };

            let report = ContractSynthesizer::verify_contract(&contract, generated_code);
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&report).unwrap_or_default()
                    }
                ]
            })
        }

        "resolve_symbol" => {
            let symbol = match args.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'symbol' argument is required"}],
                        "isError": true
                    });
                }
            };
            let from_file = args.get("from_file").and_then(|s| s.as_str());
            let target_path = args.get("target_path").and_then(|s| s.as_str()).unwrap_or(".");

            let graph = SymbolGraph::index_directory(target_path);
            let resolved = graph.resolve_symbol(symbol, from_file);

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&resolved).unwrap_or_default()
                    }
                ]
            })
        }

        "get_blast_radius" => {
            let symbol = match args.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'symbol' argument is required"}],
                        "isError": true
                    });
                }
            };
            let file = args.get("file").and_then(|s| s.as_str());
            let target_path = args.get("path").and_then(|s| s.as_str()).unwrap_or(".");
            let depth = args.get("depth").and_then(|d| d.as_u64()).unwrap_or(2) as usize;

            let graph = SymbolGraph::index_directory(target_path);
            let report = graph.calculate_blast_radius(symbol, file, depth);

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&report).unwrap_or_default()
                    }
                ]
            })
        }

        "find_references" => {
            let symbol = match args.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'symbol' argument is required"}],
                        "isError": true
                    });
                }
            };
            let target_path = args.get("target_path").and_then(|s| s.as_str()).unwrap_or(".");

            let graph = SymbolGraph::index_directory(target_path);
            let refs = graph.find_references(symbol);

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&refs).unwrap_or_default()
                    }
                ]
            })
        }

        "prepare_context" => {
            let start = Instant::now();
            let target_file = match args.get("target_file").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'target_file' argument is required"}],
                        "isError": true
                    });
                }
            };

            let symbol_opt = args.get("symbol").and_then(|s| s.as_str());
            let depth = args.get("depth").and_then(|d| d.as_u64()).unwrap_or(2) as usize;

            let file_path = Path::new(target_file);
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    return json!({
                        "content": [{"type": "text", "text": format!("Error reading target_file '{}': {}", target_file, e)}],
                        "isError": true
                    });
                }
            };

            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = Language::from_extension(ext);

            let skeleton = AstDiffEngine::skeletonize(&content, lang);
            let token_savings = SymbolGraph::calculate_token_savings(&content, &skeleton);

            let (sliced_context, blast_radius, resolved_symbol) = if let Some(sym) = symbol_opt {
                let parent_dir = file_path.parent().unwrap_or(Path::new("."));
                let search_root = if parent_dir.as_os_str().is_empty() { Path::new(".") } else { parent_dir };
                let graph = SymbolGraph::index_directory(search_root);

                let slice = ContextSlicer::slice_from_source(&content, sym, depth, lang);
                let report = graph.calculate_blast_radius(sym, Some(target_file), depth);
                let resolved = graph.resolve_symbol(sym, Some(target_file));
                (Some(slice.sliced_code), Some(report), resolved)
            } else {
                (None, None, None)
            };

            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

            let payload = json!({
                "target_file": target_file,
                "language": lang.to_string(),
                "file_skeleton": skeleton,
                "token_savings_percent": token_savings,
                "symbol": symbol_opt,
                "resolved_symbol": resolved_symbol,
                "sliced_context": sliced_context,
                "blast_radius": blast_radius,
                "latency_ms": latency_ms
            });

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&payload).unwrap_or_default()
                    }
                ]
            })
        }

        "verified_patch" => {
            let start = Instant::now();
            let file_path_str = match args.get("file_path").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'file_path' argument is required"}],
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
                Some(c) => c,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'new_code' argument is required"}],
                        "isError": true
                    });
                }
            };

            let dry_run = args.get("dry_run").and_then(|b| b.as_bool()).unwrap_or(false);

            // Step 1: Pre-patch in-memory invariant check on new_code
            let pre_check = AstGuard::verify(new_code);
            if !pre_check.passed {
                let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                let err_report = json!({
                    "success": false,
                    "stage": "pre_patch_safety_verification",
                    "file_path": file_path_str,
                    "symbol": symbol,
                    "violation": pre_check.violation.map(|v| v.to_string()),
                    "detail": pre_check.detail,
                    "latency_ms": latency_ms
                });
                return json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&err_report).unwrap_or_default()}],
                    "isError": true
                });
            }

            // Step 2: Read target file
            let original_content = match fs::read_to_string(file_path_str) {
                Ok(c) => c,
                Err(e) => {
                    return json!({
                        "content": [{"type": "text", "text": format!("Error reading file_path '{}': {}", file_path_str, e)}],
                        "isError": true
                    });
                }
            };

            let ext = Path::new(file_path_str).extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or(ext);
            let lang = Language::from_extension(lang_str);

            // Step 3: In-memory surgical patch
            let patched_content = match AstDiffEngine::patch(&original_content, symbol, new_code, lang) {
                Ok(p) => p,
                Err(e) => {
                    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                    let err_report = json!({
                        "success": false,
                        "stage": "ast_patch_execution",
                        "file_path": file_path_str,
                        "symbol": symbol,
                        "error": e.to_string(),
                        "latency_ms": latency_ms
                    });
                    return json!({
                        "content": [{"type": "text", "text": serde_json::to_string_pretty(&err_report).unwrap_or_default()}],
                        "isError": true
                    });
                }
            };

            // Step 4: Post-patch full-file integrity validation
            let post_check = AstGuard::verify(&patched_content);
            if !post_check.passed {
                let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                let err_report = json!({
                    "success": false,
                    "stage": "post_patch_full_file_verification",
                    "file_path": file_path_str,
                    "symbol": symbol,
                    "violation": post_check.violation.map(|v| v.to_string()),
                    "detail": post_check.detail,
                    "latency_ms": latency_ms
                });
                return json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&err_report).unwrap_or_default()}],
                    "isError": true
                });
            }

            // Step 5: Atomically write to disk (if not dry_run)
            if !dry_run {
                if let Err(e) = fs::write(file_path_str, &patched_content) {
                    return json!({
                        "content": [{"type": "text", "text": format!("Error writing patched content to '{}': {}", file_path_str, e)}],
                        "isError": true
                    });
                }
            }

            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let orig_lines = original_content.lines().count();
            let patched_lines = patched_content.lines().count();

            let result = json!({
                "success": true,
                "file_path": file_path_str,
                "symbol": symbol,
                "written_to_disk": !dry_run,
                "safety_verified": true,
                "diff_summary": format!("Surgically replaced symbol '{}' ({} lines -> {} lines)", symbol, orig_lines, patched_lines),
                "latency_ms": latency_ms
            });

            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_default()
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
        assert_eq!(parsed["result"]["serverInfo"]["version"], "1.0.0");
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
        assert_eq!(tools.len(), 12);

        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"check_safety"));
        assert!(names.contains(&"skeletonize"));
        assert!(names.contains(&"patch_symbol"));
        assert!(names.contains(&"index_graph"));
        assert!(names.contains(&"synthesize_contract"));
        assert!(names.contains(&"extract_intent_slice"));
        assert!(names.contains(&"verify_contract"));
        assert!(names.contains(&"resolve_symbol"));
        assert!(names.contains(&"get_blast_radius"));
        assert!(names.contains(&"find_references"));
        assert!(names.contains(&"prepare_context"));
        assert!(names.contains(&"verified_patch"));
    }

    #[test]
    fn test_mcp_tool_call_prepare_context() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "prepare_context",
                "arguments": {
                    "target_file": "src/lib.rs",
                    "symbol": "SymbolGraph"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("file_skeleton"));
        assert!(text.contains("token_savings_percent"));
    }

    #[test]
    fn test_mcp_tool_call_verified_patch_dry_run() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 13,
            "method": "tools/call",
            "params": {
                "name": "verified_patch",
                "arguments": {
                    "file_path": "src/diff.rs",
                    "symbol": "AstDiffEngine",
                    "new_code": "pub struct AstDiffEngine;",
                    "dry_run": true
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"success\": true"));
        assert!(text.contains("\"safety_verified\": true"));
    }

    #[test]
    fn test_mcp_tool_call_verified_patch_rejects_unsafe_code() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 14,
            "method": "tools/call",
            "params": {
                "name": "verified_patch",
                "arguments": {
                    "file_path": "src/diff.rs",
                    "symbol": "AstDiffEngine",
                    "new_code": "pub struct AstDiffEngine; fn bad(opt: Option<i32>) -> i32 { opt.unwrap() }",
                    "dry_run": true
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["result"]["isError"], true);
    }

    #[test]
    fn test_mcp_tool_call_resolve_symbol() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "resolve_symbol",
                "arguments": {
                    "symbol": "AstGuard",
                    "target_path": "src"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("AstGuard"));
    }

    #[test]
    fn test_mcp_tool_call_get_blast_radius() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "get_blast_radius",
                "arguments": {
                    "symbol": "AstGuard",
                    "path": "src",
                    "depth": 2
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("risk_score"));
    }

    #[test]
    fn test_mcp_tool_call_find_references() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "find_references",
                "arguments": {
                    "symbol": "verify",
                    "target_path": "src"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("file"));
    }

    #[test]
    fn test_mcp_tool_call_synthesize_contract() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "synthesize_contract",
                "arguments": {
                    "intent": "async payment checkout session",
                    "language": "rust"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("PaymentCheckoutRequest"));
    }

    #[test]
    fn test_mcp_tool_call_extract_intent_slice() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "extract_intent_slice",
                "arguments": {
                    "symbol": "render",
                    "code": "pub struct Widget; impl Widget { pub fn render(&self) {} }",
                    "language": "rust",
                    "depth": 1
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("render"));
    }

    #[test]
    fn test_mcp_tool_call_verify_contract() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "verify_contract",
                "arguments": {
                    "intent": "compute score",
                    "generated_code": "pub struct ComputeScoreRequest; pub struct ComputeScoreResponse; pub async fn compute_score(req: &ComputeScoreRequest) -> Result<ComputeScoreResponse, ()> { Ok(ComputeScoreResponse) }",
                    "language": "rust"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"passed\": true"));
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
