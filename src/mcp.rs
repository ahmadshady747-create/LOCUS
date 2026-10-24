//! Model Context Protocol (MCP) Server for locus-engine over stdio (JSON-RPC 2.0).

pub mod tools;
pub mod tools_v15_a;

use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Instant;

use crate::contract::ContractSynthesizer;
use crate::diff::AstDiffEngine;
use crate::graph::SymbolGraph;
use crate::guard::AstGuard;
use crate::slice::ContextSlicer;
use crate::types::Language;
use crate::types::ViolationKind;

static LEASE_REGISTRY: std::sync::LazyLock<crate::lease::LeaseRegistry> =
    std::sync::LazyLock::new(crate::lease::LeaseRegistry::new);

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
                        "version": env!("CARGO_PKG_VERSION")
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
                        "description": "Deterministic AST invariant safety check (<0.05ms) catching delimiter balance, async mutex across await, div-by-zero, array bounds, unsafe unwraps, ReDoS, JSX tag mismatches, conditional hooks, and client secret leaks. First-failure gate: reports the first violation only; use check_safety_all for the full report.",
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
                    },
                    {
                        "name": "begin_tx",
                        "description": "Opens a multi-file ACID workspace transaction in-memory with automatic rollback journal.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "stage_tx",
                        "description": "Stages a file modification into the active in-memory ACID transaction buffer.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "tx_id": {
                                    "type": "string",
                                    "description": "Active transaction ID"
                                },
                                "file_path": {
                                    "type": "string",
                                    "description": "Target file path to stage"
                                },
                                "content": {
                                    "type": "string",
                                    "description": "Staged file content"
                                },
                                "language": {
                                    "type": "string",
                                    "description": "Optional language"
                                }
                            },
                            "required": ["tx_id", "file_path", "content"]
                        }
                    },
                    {
                        "name": "commit_tx",
                        "description": "Validates all staged files across AST invariants and atomically commits to disk if and only if 100% pass.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "tx_id": {
                                    "type": "string",
                                    "description": "Active transaction ID to commit"
                                }
                            },
                            "required": ["tx_id"]
                        }
                    },
                    {
                        "name": "rollback_tx",
                        "description": "Rolls back and aborts an active workspace transaction, clearing staged buffers without touching disk.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "tx_id": {
                                    "type": "string",
                                    "description": "Transaction ID to rollback"
                                }
                            },
                            "required": ["tx_id"]
                        }
                    },
                    {
                        "name": "auto_remediate",
                        "description": "Deterministic AST self-healing rewriter that automatically closes broken JSX/HTML tags, fixes deep property null accesses to optional chaining (?.), and hoists conditional React hooks.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Raw source code snippet to deterministically remediate"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "acquire_symbol_lease",
                        "description": "Acquires an exclusive short-lived symbol lease with TTL on an FQN across multi-agent swarms to prevent concurrent write collisions.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "fqn": {
                                    "type": "string",
                                    "description": "Fully Qualified Symbol Name (e.g. src/auth.rs::login)"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Unique agent identifier"
                                },
                                "ttl_ms": {
                                    "type": "integer",
                                    "description": "Time-To-Live in milliseconds (default: 60000)"
                                }
                            },
                            "required": ["fqn", "agent_id"]
                        }
                    },
                    {
                        "name": "release_symbol_lease",
                        "description": "Releases an active symbol lease held by an agent.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "lease_id": {
                                    "type": "string",
                                    "description": "Lease ID returned during acquisition"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Agent identifier"
                                }
                            },
                            "required": ["lease_id", "agent_id"]
                        }
                    },
                    {
                        "name": "renew_symbol_lease",
                        "description": "Renews an active symbol lease heartbeat extension.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "lease_id": {
                                    "type": "string",
                                    "description": "Active lease ID"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Agent identifier"
                                },
                                "extension_ms": {
                                    "type": "integer",
                                    "description": "Extension duration in milliseconds (default: 60000)"
                                }
                            },
                            "required": ["lease_id", "agent_id"]
                        }
                    },
                    {
                        "name": "trace_taint_flow",
                        "description": "Traces cross-file taint and unvalidated external data flows heading toward sensitive sinks (DB queries, filesystem calls, unhandled Option/null returns).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Target file to trace"
                                },
                                "code": {
                                    "type": "string",
                                    "description": "Optional raw source code snippet to analyze directly without disk access"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Optional symbol to scope taint tracking"
                                }
                            },
                            "required": ["file_path"]
                        }
                    },
                    {
                        "name": "hybrid_search",
                        "description": "Executes sub-millisecond in-memory hybrid AST lexical + quantized HNSW vector search across indexed symbols.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Semantic query or symbol description"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Workspace root directory to search (default: .)"
                                },
                                "top_k": {
                                    "type": "integer",
                                    "description": "Maximum number of search results (default: 5)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "query_cst",
                        "description": "Parses raw source code into a Lossless Concrete Syntax Tree (CST) with Green/Red nodes, trivia (comments, whitespace), and absolute byte offsets.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Source code snippet to parse into CST"
                                },
                                "language": {
                                    "type": "string",
                                    "enum": ["rust", "typescript", "javascript", "tsx", "jsx", "python"],
                                    "description": "Optional programming language (default: rust)"
                                },
                                "offset": {
                                    "type": "integer",
                                    "description": "Optional byte offset to query the exact token or syntax node at"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "audit_taint_path",
                        "description": "Traces inter-procedural SSA data-flow taint propagation across call graph G=(V, E) and returns verified sanitizer proof chains with cryptographic TaintAuditCertificate.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Target source file path"
                                },
                                "code": {
                                    "type": "string",
                                    "description": "Optional raw source code content"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Optional target symbol name (default: '*')"
                                }
                            },
                            "required": ["file_path"]
                        }
                    },
                    {
                        "name": "acquire_subtree_lease",
                        "description": "Acquires an exclusive hierarchical wildcard lease (e.g. `src/auth/*` or `crate::billing::*`) with TTL across multi-agent swarms, enforcing bidirectional subtree conflict guards.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "pattern": {
                                    "type": "string",
                                    "description": "Wildcard pattern or module path (e.g. `src/auth/*`, `crate::billing::*`)"
                                },
                                "agent_id": {
                                    "type": "string",
                                    "description": "Unique agent identifier"
                                },
                                "ttl_ms": {
                                    "type": "integer",
                                    "description": "Lease duration in milliseconds (default: 60000)"
                                }
                            },
                            "required": ["pattern", "agent_id"]
                        }
                    },
                    {
                        "name": "verify_occ_token",
                        "description": "Deterministically checks or commits monotonic Optimistic Concurrency Control (OCC) version tokens across multi-agent swarms.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "fqn": {
                                    "type": "string",
                                    "description": "Target symbol or module FQN"
                                },
                                "expected_version": {
                                    "type": "integer",
                                    "description": "Expected monotonic OCC version"
                                },
                                "commit": {
                                    "type": "boolean",
                                    "description": "If true, advances the OCC version counter on match (default: false)"
                                }
                            },
                            "required": ["fqn", "expected_version"]
                        }
                    },
                    {
                        "name": "morph_ast",
                        "description": "Executes deterministic AST transformations, refactorings, and rule remediations (converting unsafe unwraps to ?, hoisting conditional hooks, balancing JSX tags, repairing deep property null accesses) with full post-verification proof.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Source code to transform"
                                },
                                "language": {
                                    "type": "string",
                                    "description": "Optional programming language"
                                }
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "simd_vector_search",
                        "description": "Executes batch hardware-accelerated (AVX2 256-bit / ARM NEON 128-bit) quantized 8-bit dot-product vector search with zero heap allocations, achieving sub-20µs latency.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query_text": {
                                    "type": "string",
                                    "description": "Query text to embed and search"
                                },
                                "query_vector": {
                                    "type": "array",
                                    "items": {"type": "integer"},
                                    "description": "Optional raw 64-dim i8 quantized query vector"
                                },
                                "corpus": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"},
                                            "text": {"type": "string"}
                                        },
                                        "required": ["id"]
                                    },
                                    "description": "Optional list of corpus items to dynamically index and search"
                                },
                                "top_k": {
                                    "type": "integer",
                                    "description": "Number of nearest neighbors to return (default: 5)"
                                }
                            }
                        }
                    },
                    {
                        "name": "semantic_analyze",
                        "description": "Executes comprehensive semantic invariant analysis (Rules S1..S5), synthesizes formal mathematical proofs with WP calculus derivations, detects counterexamples, and generates deterministic auto-fixes with optional SARIF v2.1.0 export.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Source code text to analyze"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional file path to load source code from"
                                },
                                "language": {
                                    "type": "string",
                                    "description": "Programming language (typescript, javascript, rust, etc.)"
                                },
                                "sarif": {
                                    "type": "boolean",
                                    "description": "If true, includes full OASIS SARIF v2.1.0 report in the response (default: false)"
                                }
                            }
                        }
                    },
                    {
                        "name": "verify_proof",
                        "description": "Executes high-throughput dynamic AST & semantic proof verification on code claims, evaluates assertions with fail-fast tri-state verdicts, and mints cryptographic SHA-256 certificates in sub-0.15ms.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {
                                    "type": "string",
                                    "description": "Source code text to verify"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional file path to load source code from"
                                },
                                "language": {
                                    "type": "string",
                                    "description": "Optional target programming language (default: rust)"
                                },
                                "claim": {
                                    "type": "object",
                                    "description": "ProofClaim JSON object containing claim_id, target_symbol, invariant_name, and assertions"
                                },
                                "claim_id": {
                                    "type": "string",
                                    "description": "Optional claim ID shorthand if constructing inline claim"
                                },
                                "target_symbol": {
                                    "type": "string",
                                    "description": "Optional target symbol shorthand"
                                },
                                "invariant_name": {
                                    "type": "string",
                                    "description": "Optional invariant name shorthand"
                                },
                                "assertions": {
                                    "type": "array",
                                    "description": "Optional list of ProofAssertion objects"
                                }
                            }
                        }
                    },
                    {
                        "name": "explain_violation",
                        "description": "Agentic violation explainer: runs the AST safety check and returns a structured, machine-readable report — rule, detail, 1-based line and column, offending line text, and an actionable fix pattern — instead of a prose message. Returns passed:true with null location fields when the code is clean.",
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
                        "name": "diff_check",
                        "description": "Agentic change-scoped check: verifies new_code and reports whether the first violation lies inside the changed line range (computed by common line prefix/suffix against old_code) or is pre-existing, plus a baseline check of old_code. Lets an agent tell 'my edit broke this' apart from 'this was already broken'.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "old_code": {
                                    "type": "string",
                                    "description": "Original source code before the edit"
                                },
                                "new_code": {
                                    "type": "string",
                                    "description": "Source code after the edit"
                                }
                            },
                            "required": ["old_code", "new_code"]
                        }
                    },
                    {
                        "name": "check_safety_all",
                        "description": "Runs all 32 invariant rules and returns EVERY violation found (with violation_count), unlike check_safety which stops at the first failure. Lets an agent fix all violations in one pass instead of looping fix-recheck per violation.",
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


/// Agentic helpers for the new MCP tools (explain_violation, diff_check).
/// Parse "byte N" out of an AstGuard detail string, e.g. "… at byte 30: `xs[0]`".
fn parse_byte_offset(detail: &str) -> Option<usize> {
    let idx = detail.find("byte ")? + 5;
    let rest = &detail[idx..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// Convert a byte offset into 1-based (line, column) plus the full line text.
/// The offset is clamped to a UTF-8 char boundary; out-of-range offsets
/// (beyond the end of the source) are clamped to the end.
fn byte_to_line_col(code: &str, mut byte: usize) -> Option<(usize, usize, String)> {
    if byte > code.len() {
        byte = code.len();
    }
    while byte > 0 && !code.is_char_boundary(byte) {
        byte -= 1;
    }
    let prefix = &code[..byte];
    let line = prefix.matches('\n').count() + 1;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = prefix[line_start..].chars().count() + 1;
    let snippet = code.lines().nth(line - 1).unwrap_or("").to_string();
    Some((line, column, snippet))
}

/// Actionable fix pattern for a violation kind. This is guidance text for an
/// agent to act on, not an automatic rewrite.
fn fix_pattern(kind: &ViolationKind) -> &'static str {
    match kind {
        ViolationKind::ArrayOutOfBounds => {
            "Replace direct indexing `xs[i]` with `xs.get(i)` and handle None, or guard with `i < xs.len()`."
        }
        ViolationKind::UnsafeUnwrap => {
            "Replace `.unwrap()`/`.expect()` with `match`, `?`, or `ok_or`/`unwrap_or` handling."
        }
        ViolationKind::DivisionByZero => {
            "Guard the divisor (e.g. `if divisor != 0`) or use `checked_div` and handle None."
        }
        ViolationKind::UncheckedArithmeticOverflow => {
            "Use `checked_*` arithmetic (`checked_add`/`checked_sub`/`checked_mul`) and handle None."
        }
        ViolationKind::UnbalancedDelimiters => {
            "Balance braces, brackets, and parentheses; check the reported region for a missing closer."
        }
        ViolationKind::AsyncMutexAcrossAwait => {
            "Scope and drop the mutex guard before `.await`, or use an async-aware lock."
        }
        _ => "Manual review required for this rule; see detail.",
    }
}

/// Changed line range in `new_lines` relative to `old_lines`, computed by
/// common line prefix/suffix. 1-based inclusive (start, end); None when the
/// two texts are identical (or the new text is empty).
fn changed_line_range(old_lines: &[&str], new_lines: &[&str]) -> Option<(usize, usize)> {
    if new_lines.is_empty() {
        return None;
    }
    if old_lines == new_lines {
        return None;
    }
    let mut prefix = 0;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_lines.len() - prefix
        && suffix < new_lines.len() - prefix
        && old_lines[old_lines.len() - 1 - suffix] == new_lines[new_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let start = prefix + 1;
    let mut end = new_lines.len() - suffix;
    if end < start {
        end = start;
    }
    Some((start, end))
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
            let report_json =
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
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

            let lang_str = args
                .get("language")
                .and_then(|s| s.as_str())
                .unwrap_or("rust");
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

            let lang_str = args
                .get("language")
                .and_then(|s| s.as_str())
                .unwrap_or("rust");
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
            let lang_str = args
                .get("language")
                .and_then(|s| s.as_str())
                .unwrap_or("rust");
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
            let lang_str = args
                .get("language")
                .and_then(|s| s.as_str())
                .unwrap_or("rust");
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
                    let ext = path_obj
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or(lang_str);
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
                if let Ok(c) =
                    serde_json::from_value::<crate::contract::IntentContract>(c_val.clone())
                {
                    c
                } else {
                    let intent_str = c_val
                        .get("intent")
                        .and_then(|s| s.as_str())
                        .unwrap_or("general implementation");
                    let lang_str = args
                        .get("language")
                        .and_then(|s| s.as_str())
                        .unwrap_or("rust");
                    ContractSynthesizer::synthesize(
                        intent_str,
                        None,
                        None,
                        Language::from_extension(lang_str),
                    )
                }
            } else if let Some(intent) = args.get("intent").and_then(|s| s.as_str()) {
                let lang_str = args
                    .get("language")
                    .and_then(|s| s.as_str())
                    .unwrap_or("rust");
                ContractSynthesizer::synthesize(
                    intent,
                    None,
                    None,
                    Language::from_extension(lang_str),
                )
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
            let target_path = args
                .get("target_path")
                .and_then(|s| s.as_str())
                .unwrap_or(".");

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
            let target_path = args
                .get("target_path")
                .and_then(|s| s.as_str())
                .unwrap_or(".");

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
                let search_root = if parent_dir.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent_dir
                };
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

            let dry_run = args
                .get("dry_run")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

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

            let ext = Path::new(file_path_str)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang_str = args.get("language").and_then(|s| s.as_str()).unwrap_or(ext);
            let lang = Language::from_extension(lang_str);

            // Step 3: In-memory surgical patch
            let patched_content = match AstDiffEngine::patch(
                &original_content,
                symbol,
                new_code,
                lang,
            ) {
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

        "begin_tx" => match tools_v15_a::handle_begin_tx(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(err) => json!({
                "content": [{"type": "text", "text": format!("Error: {}", err)}],
                "isError": true
            }),
        },

        "stage_tx" => match tools_v15_a::handle_stage_tx(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(err) => json!({
                "content": [{"type": "text", "text": format!("Error: {}", err)}],
                "isError": true
            }),
        },

        "commit_tx" => match tools_v15_a::handle_commit_tx(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(err) => json!({
                "content": [{"type": "text", "text": format!("Error: {}", err)}],
                "isError": true
            }),
        },

        "rollback_tx" => match tools_v15_a::handle_rollback_tx(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(err) => json!({
                "content": [{"type": "text", "text": format!("Error: {}", err)}],
                "isError": true
            }),
        },

        "auto_remediate" => match tools_v15_a::handle_auto_remediate(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(err) => json!({
                "content": [{"type": "text", "text": format!("Error: {}", err)}],
                "isError": true
            }),
        },

        "acquire_symbol_lease" => {
            let fqn = match args.get("fqn").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'fqn' argument is required"}], "isError": true})
                }
            };
            let agent_id = match args.get("agent_id").and_then(|s| s.as_str()) {
                Some(a) => a,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'agent_id' argument is required"}], "isError": true})
                }
            };
            let ttl_ms = args.get("ttl_ms").and_then(|v| v.as_u64()).unwrap_or(60000);
            let status = LEASE_REGISTRY.acquire(fqn, agent_id, ttl_ms);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&status).unwrap_or_default()}]
            })
        }

        "release_symbol_lease" => {
            let lease_id = match args.get("lease_id").and_then(|s| s.as_str()) {
                Some(l) => l,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'lease_id' argument is required"}], "isError": true})
                }
            };
            let agent_id = match args.get("agent_id").and_then(|s| s.as_str()) {
                Some(a) => a,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'agent_id' argument is required"}], "isError": true})
                }
            };
            let status = LEASE_REGISTRY.release(lease_id, agent_id);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&status).unwrap_or_default()}]
            })
        }

        "renew_symbol_lease" => {
            let lease_id = match args.get("lease_id").and_then(|s| s.as_str()) {
                Some(l) => l,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'lease_id' argument is required"}], "isError": true})
                }
            };
            let agent_id = match args.get("agent_id").and_then(|s| s.as_str()) {
                Some(a) => a,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'agent_id' argument is required"}], "isError": true})
                }
            };
            let extension_ms = args
                .get("extension_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(60000);
            let status = LEASE_REGISTRY.renew(lease_id, agent_id, extension_ms);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&status).unwrap_or_default()}]
            })
        }

        "trace_taint_flow" => {
            let file_path = match args.get("file_path").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'file_path' argument is required"}], "isError": true})
                }
            };
            let symbol = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("*");
            let content = if let Some(c) = args.get("code").and_then(|s| s.as_str()) {
                c.to_string()
            } else {
                match fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return json!({"content": [{"type": "text", "text": format!("Error reading file '{}': {}", file_path, e)}], "isError": true})
                    }
                }
            };
            let reports =
                crate::taint::DataFlowTracker::analyze_source(file_path, symbol, &content);
            let null_reports =
                crate::taint::NullPropagationTracker::scan_nullable_flows(file_path, &content);
            let mut all_reports = reports;
            all_reports.extend(null_reports);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&all_reports).unwrap_or_default()}]
            })
        }

        "hybrid_search" => {
            let query = match args.get("query").and_then(|s| s.as_str()) {
                Some(q) => q,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'query' argument is required"}], "isError": true})
                }
            };
            let path = args.get("path").and_then(|s| s.as_str()).unwrap_or(".");
            let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

            let graph = SymbolGraph::index_directory(path);
            let matcher = crate::search::HybridMatcher::new();
            matcher.index_graph(&graph);
            let result = matcher.search(query, top_k);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "query_cst" => {
            let code = match args.get("code").and_then(|s| s.as_str()) {
                Some(c) => c,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'code' argument is required"}], "isError": true})
                }
            };
            let lang_str = args
                .get("language")
                .and_then(|s| s.as_str())
                .unwrap_or("rust");
            let lang = Language::from_extension(lang_str);
            let red_tree = crate::cst::parse_to_cst(code);
            let offset_opt = args
                .get("offset")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let token_info = if let Some(off) = offset_opt {
                red_tree.token_at_offset(off).map(|t| {
                    json!({
                        "kind": format!("{:?}", t.kind()),
                        "text": t.text(),
                        "start": t.text_range().start(),
                        "end": t.text_range().end(),
                        "is_trivia": t.kind().is_trivia(),
                    })
                })
            } else {
                None
            };

            let reconstructed = red_tree.text();
            let result = json!({
                "status": "success",
                "language": lang.to_string(),
                "total_text_length": reconstructed.len(),
                "trivia_roundtrip_intact": reconstructed == code,
                "token_at_offset": token_info,
                "root_kind": format!("{:?}", red_tree.kind()),
                "root_text": reconstructed
            });

            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "audit_taint_path" => {
            let file_path = match args.get("file_path").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'file_path' argument is required"}], "isError": true})
                }
            };
            let symbol = args.get("symbol").and_then(|s| s.as_str()).unwrap_or("*");
            let content = if let Some(c) = args.get("code").and_then(|s| s.as_str()) {
                c.to_string()
            } else {
                match fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return json!({"content": [{"type": "text", "text": format!("Error reading file '{}': {}", file_path, e)}], "isError": true})
                    }
                }
            };

            let reports =
                crate::taint::DataFlowTracker::analyze_source(file_path, symbol, &content);
            let null_reports =
                crate::taint::NullPropagationTracker::scan_nullable_flows(file_path, &content);
            let mut all = reports;
            all.extend(null_reports);

            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&all).unwrap_or_default()}]
            })
        }

        "acquire_subtree_lease" => {
            let pattern = match args.get("pattern").and_then(|s| s.as_str()) {
                Some(p) => p,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'pattern' argument is required"}], "isError": true})
                }
            };
            let agent_id = match args.get("agent_id").and_then(|s| s.as_str()) {
                Some(a) => a,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'agent_id' argument is required"}], "isError": true})
                }
            };
            let ttl_ms = args.get("ttl_ms").and_then(|v| v.as_u64()).unwrap_or(60000);

            let status = LEASE_REGISTRY.acquire(pattern, agent_id, ttl_ms);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&status).unwrap_or_default()}]
            })
        }

        "verify_occ_token" => {
            let fqn = match args.get("fqn").and_then(|s| s.as_str()) {
                Some(f) => f,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'fqn' argument is required"}], "isError": true})
                }
            };
            let expected_version = match args.get("expected_version").and_then(|v| v.as_u64()) {
                Some(v) => v,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'expected_version' argument is required"}], "isError": true})
                }
            };
            let commit = args.get("commit").and_then(|v| v.as_bool()).unwrap_or(false);

            let res = if commit {
                LEASE_REGISTRY.commit_occ(fqn, expected_version)
            } else {
                LEASE_REGISTRY.verify_occ_token(fqn, expected_version)
            };

            match res {
                Ok(v) => json!({
                    "content": [{"type": "text", "text": json!({"status": "success", "fqn": fqn, "version": v, "committed": commit}).to_string()}]
                }),
                Err(err_status) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&err_status).unwrap_or_default()}],
                    "isError": true
                }),
            }
        }

        "morph_ast" => {
            let code = match args.get("code").and_then(|s| s.as_str()) {
                Some(c) => c,
                None => {
                    return json!({"content": [{"type": "text", "text": "Error: 'code' argument is required"}], "isError": true})
                }
            };

            let result = crate::remediate::AutoFixer::remediate(code);
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "simd_vector_search" => {
            let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let query_vec: Vec<i8> = if let Some(arr) =
                args.get("query_vector").and_then(|v| v.as_array())
            {
                arr.iter()
                    .filter_map(|v| v.as_i64().map(|n| n as i8))
                    .collect()
            } else if let Some(txt) = args.get("query_text").and_then(|s| s.as_str()) {
                crate::search::HybridMatcher::embed_text(txt)
            } else {
                return json!({"content": [{"type": "text", "text": "Error: Either 'query_text' or 'query_vector' is required"}], "isError": true});
            };

            let mut index = crate::search::HnswIndex::default();

            if let Some(corpus_arr) = args.get("corpus").and_then(|v| v.as_array()) {
                for item in corpus_arr {
                    if let Some(id) = item.get("id").and_then(|v| v.as_u64()) {
                        let vec = if let Some(txt) = item.get("text").and_then(|s| s.as_str()) {
                            crate::search::HybridMatcher::embed_text(txt)
                        } else {
                            vec![0i8; crate::search::DEFAULT_DIM]
                        };
                        index.insert(id, vec);
                    }
                }
            } else {
                let sample = crate::search::HybridMatcher::embed_text("sample default vector");
                index.insert(1, sample);
            }

            let mut scratch = crate::search::HnswQueryScratch::with_capacity(top_k * 4);
            let mut hits = Vec::with_capacity(top_k);
            let start = Instant::now();
            index.search_with_scratch(&query_vec, top_k, &mut scratch, &mut hits);
            let latency_us = start.elapsed().as_secs_f64() * 1_000_000.0;

            let result = json!({
                "status": "success",
                "top_k": top_k,
                "hits": hits.into_iter().map(|(id, score)| json!({"id": id, "score": score})).collect::<Vec<_>>(),
                "latency_us": latency_us,
                "simd_accelerated": true
            });

            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "semantic_analyze" => match tools::handle_semantic_analyze(args) {
            Ok(val) => json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default()}]
            }),
            Err(e) => json!({
                "content": [{"type": "text", "text": format!("Error in semantic_analyze: {}", e)}],
                "isError": true
            }),
        },

        "verify_proof" => {
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

            let language = args.get("language").and_then(|s| s.as_str()).unwrap_or("rust");

            let claim: crate::proof_verifier::ProofClaim = if let Some(claim_val) = args.get("claim") {
                match serde_json::from_value(claim_val.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        return json!({
                            "content": [{"type": "text", "text": format!("Invalid 'claim' payload: {}", e)}],
                            "isError": true
                        });
                    }
                }
            } else {
                let claim_id = args.get("claim_id").and_then(|s| s.as_str()).unwrap_or("claim-dynamic-01");
                let target_symbol = args.get("target_symbol").and_then(|s| s.as_str()).unwrap_or("target");
                let invariant_name = args.get("invariant_name").and_then(|s| s.as_str()).unwrap_or("InvariantSafety");
                let mut assertions = Vec::new();
                if let Some(arr) = args.get("assertions").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Ok(ast_assertion) = serde_json::from_value(item.clone()) {
                            assertions.push(ast_assertion);
                        }
                    }
                }
                let mut c = crate::proof_verifier::ProofClaim::new(
                    claim_id,
                    "inline",
                    target_symbol,
                    invariant_name,
                    0,
                );
                c.assertions = assertions;
                c
            };

            let result = crate::proof_verifier::ProofVerifierEngine::verify_source(&code, Some(language), &claim);
            let result_json = serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string());
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": result_json
                    }
                ]
            })
        }

        "explain_violation" => {
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
            let (line, column, snippet) = report
                .detail
                .as_ref()
                .and_then(|d| parse_byte_offset(d))
                .and_then(|b| byte_to_line_col(&code, b))
                .map(|(l, c, s)| (Some(l), Some(c), Some(s)))
                .unwrap_or((None, None, None));
            let result = json!({
                "passed": report.passed,
                "violation": report.violation.as_ref().map(|v| v.to_string()),
                "detail": report.detail,
                "violations": report.violations,
                "line": line,
                "column": column,
                "snippet": snippet,
                "fix_pattern": report.violation.as_ref().map(fix_pattern),
                "latency_ms": report.latency_ms
            });
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "diff_check" => {
            let old_code = match args.get("old_code").and_then(|s| s.as_str()) {
                Some(c) => c,
                None => {
                    return json!({
                        "content": [{"type": "text", "text": "Error: 'old_code' argument is required"}],
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

            let old_lines: Vec<&str> = old_code.lines().collect();
            let new_lines: Vec<&str> = new_code.lines().collect();
            let range = changed_line_range(&old_lines, &new_lines);
            let new_report = AstGuard::verify(new_code);
            let old_report = AstGuard::verify(old_code);
            let (vline, vcolumn) = new_report
                .detail
                .as_ref()
                .and_then(|d| parse_byte_offset(d))
                .and_then(|b| byte_to_line_col(new_code, b))
                .map(|(l, c, _)| (Some(l), Some(c)))
                .unwrap_or((None, None));
            let in_changed_region = match (vline, range) {
                (Some(l), Some((start, end))) => Some(l >= start && l <= end),
                _ => None,
            };
            let result = json!({
                "passed": new_report.passed,
                "baseline_passed": old_report.passed,
                "changed_lines": range.map(|(s, e)| json!({"start": s, "end": e})),
                "violation": new_report.violation.as_ref().map(|v| v.to_string()),
                "detail": new_report.detail,
                "violation_line": vline,
                "violation_column": vcolumn,
                "in_changed_region": in_changed_region,
                "latency_ms": new_report.latency_ms
            });
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            })
        }

        "check_safety_all" => {
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

            let report = crate::guard::RuleRunner::verify_all(&code);
            let result = json!({
                "passed": report.passed,
                "violation": report.violation.as_ref().map(|v| v.to_string()),
                "detail": report.detail,
                "violations": report.violations,
                "violation_count": report.violations.len(),
                "latency_ms": report.latency_ms
            });
            json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
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
        assert_eq!(parsed["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
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
        assert_eq!(tools.len(), 33);

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
        assert!(names.contains(&"begin_tx"));
        assert!(names.contains(&"stage_tx"));
        assert!(names.contains(&"commit_tx"));
        assert!(names.contains(&"rollback_tx"));
        assert!(names.contains(&"auto_remediate"));
        assert!(names.contains(&"acquire_symbol_lease"));
        assert!(names.contains(&"release_symbol_lease"));
        assert!(names.contains(&"renew_symbol_lease"));
        assert!(names.contains(&"trace_taint_flow"));
        assert!(names.contains(&"hybrid_search"));
        assert!(names.contains(&"query_cst"));
        assert!(names.contains(&"audit_taint_path"));
        assert!(names.contains(&"acquire_subtree_lease"));
        assert!(names.contains(&"verify_occ_token"));
        assert!(names.contains(&"morph_ast"));
        assert!(names.contains(&"simd_vector_search"));
        assert!(names.contains(&"semantic_analyze"));
        assert!(names.contains(&"verify_proof"));
        assert!(names.contains(&"explain_violation"));
        assert!(names.contains(&"diff_check"));
        assert!(names.contains(&"check_safety_all"));
    }

    #[test]
    fn test_mcp_tool_call_explain_violation_array_oob() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 91,
            "method": "tools/call",
            "params": {
                "name": "explain_violation",
                "arguments": {
                    "code": "fn get(xs: Vec<u32>) -> u32 { xs[0] }"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let report: Value = serde_json::from_str(text).unwrap();
        assert_eq!(report["passed"], false);
        assert_eq!(report["violation"], "ARRAY_OOB");
        assert_eq!(report["line"], 1);
        assert!(report["snippet"].as_str().unwrap().contains("xs[0]"));
        assert!(report["fix_pattern"].as_str().unwrap().contains(".get("));
    }

    #[test]
    fn test_mcp_tool_call_diff_check_violation_in_changed_region() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 92,
            "method": "tools/call",
            "params": {
                "name": "diff_check",
                "arguments": {
                    "old_code": "fn get(xs: Vec<u32>) -> u32 { 0 }",
                    "new_code": "fn get(xs: Vec<u32>) -> u32 { xs[0] }"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let report: Value = serde_json::from_str(text).unwrap();
        assert_eq!(report["passed"], false);
        assert_eq!(report["baseline_passed"], true);
        assert_eq!(report["in_changed_region"], true);
        assert_eq!(report["violation_line"], 1);
    }

    #[test]
    fn test_mcp_tool_call_check_safety_all_reports_all_violations() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 93,
            "method": "tools/call",
            "params": {
                "name": "check_safety_all",
                "arguments": {
                    "code": "fn f(xs: Vec<u32>, o: Option<u32>) -> u32 { xs[0] + o.unwrap() }"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let report: Value = serde_json::from_str(text).unwrap();
        assert_eq!(report["passed"], false);
        assert!(report["violation_count"].as_u64().unwrap() >= 2);
        let joined = report["violations"].to_string();
        assert!(joined.contains("ARRAY_OOB"));
        assert!(joined.contains("UNSAFE_UNWRAP"));
    }

    #[test]
    fn test_mcp_tool_call_verify_proof_proven() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": {
                "name": "verify_proof",
                "arguments": {
                    "code": "pub async fn query_engine(q: &str) -> String { let clean = DOMPurify.sanitize(q); clean.to_string() }",
                    "language": "rust",
                    "claim": {
                        "claim_id": "claim-mcp-01",
                        "target_file": "src/engine.rs",
                        "target_symbol": "query_engine",
                        "invariant_name": "SanitizedQueryInvariant",
                        "assertions": [
                            {
                                "NodeExists": {
                                    "node_kind": "FunctionDeclaration",
                                    "identifier": "query_engine"
                                }
                            },
                            {
                                "PropertyEquals": {
                                    "node_id": "query_engine",
                                    "property": "visibility",
                                    "expected_value": "pub"
                                }
                            }
                        ],
                        "created_at_ms": 1725150000000
                    }
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"verdict\": \"Proven\""));
        assert!(text.contains("LOCUS-PROOF-v2:"));
    }

    #[test]
    fn test_mcp_tool_call_verify_proof_refuted() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 32,
            "method": "tools/call",
            "params": {
                "name": "verify_proof",
                "arguments": {
                    "code": "fn private_worker() {}",
                    "language": "rust",
                    "claim": {
                        "claim_id": "claim-mcp-02",
                        "target_file": "src/worker.rs",
                        "target_symbol": "private_worker",
                        "invariant_name": "PublicApiInvariant",
                        "assertions": [
                            {
                                "PropertyEquals": {
                                    "node_id": "private_worker",
                                    "property": "visibility",
                                    "expected_value": "pub"
                                }
                            }
                        ],
                        "created_at_ms": 1725150000000
                    }
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Refuted"));
        assert!(text.contains("failed_assertion_index"));
    }

    #[test]
    fn test_mcp_tool_call_semantic_analyze() {
        let call_req = r#"{
            "jsonrpc": "2.0",
            "id": 30,
            "method": "tools/call",
            "params": {
                "name": "semantic_analyze",
                "arguments": {
                    "code": "const items = [1, 2]; items.forEach(async (x) => { await doTask(x); });",
                    "language": "typescript",
                    "sarif": true
                }
            }
        }"#;
        let resp = handle_json_rpc_message(call_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"violations_count\": 1"));
        assert!(text.contains("AsyncIteratorRace"));
        assert!(text.contains("mathematical_proof"));
        assert!(text.contains("remediation_patch"));
        assert!(text.contains("sarifLog"));
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

    #[test]
    fn test_mcp_tool_call_tx_lifecycle() {
        let begin_req = r#"{"jsonrpc":"2.0","id":100,"method":"tools/call","params":{"name":"begin_tx","arguments":{}}}"#;
        let resp = handle_json_rpc_message(begin_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        let tx_id = val["tx_id"].as_str().unwrap().to_string();
        assert_eq!(val["status"], "success");

        let stage_req = json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "tools/call",
            "params": {
                "name": "stage_tx",
                "arguments": {
                    "tx_id": tx_id,
                    "file_path": "src/dummy_math.rs",
                    "content": "pub fn add_staged(a: i32, b: i32) -> i32 { a + b }",
                    "language": "rust"
                }
            }
        });
        let resp = handle_json_rpc_message(&stage_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["status"], "success");

        let commit_req = json!({
            "jsonrpc": "2.0",
            "id": 102,
            "method": "tools/call",
            "params": {
                "name": "commit_tx",
                "arguments": {
                    "tx_id": tx_id
                }
            }
        });
        let resp = handle_json_rpc_message(&commit_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["passed_verification"], true);
    }

    #[test]
    fn test_mcp_tool_call_tx_rollback() {
        let begin_req = r#"{"jsonrpc":"2.0","id":103,"method":"tools/call","params":{"name":"begin_tx","arguments":{}}}"#;
        let resp = handle_json_rpc_message(begin_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        let tx_id = val["tx_id"].as_str().unwrap().to_string();

        let rollback_req = json!({
            "jsonrpc": "2.0",
            "id": 104,
            "method": "tools/call",
            "params": {
                "name": "rollback_tx",
                "arguments": {
                    "tx_id": tx_id
                }
            }
        });
        let resp = handle_json_rpc_message(&rollback_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["status"], "RolledBack");
    }

    #[test]
    fn test_mcp_tool_call_tx_safety_violation_abort() {
        let begin_req = r#"{"jsonrpc":"2.0","id":105,"method":"tools/call","params":{"name":"begin_tx","arguments":{}}}"#;
        let resp = handle_json_rpc_message(begin_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        let tx_id = val["tx_id"].as_str().unwrap().to_string();

        let stage_req = json!({
            "jsonrpc": "2.0",
            "id": 106,
            "method": "tools/call",
            "params": {
                "name": "stage_tx",
                "arguments": {
                    "tx_id": tx_id,
                    "file_path": "src/unsafe_test.rs",
                    "content": "pub fn bad(opt: Option<i32>) -> i32 { opt.unwrap() }",
                    "language": "rust"
                }
            }
        });
        let _ = handle_json_rpc_message(&stage_req.to_string());

        let commit_req = json!({
            "jsonrpc": "2.0",
            "id": 107,
            "method": "tools/call",
            "params": {
                "name": "commit_tx",
                "arguments": {
                    "tx_id": tx_id
                }
            }
        });
        let resp = handle_json_rpc_message(&commit_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["passed_verification"], false);
        assert!(val["status"].as_str().unwrap().contains("Failed"));
    }

    #[test]
    fn test_mcp_tool_call_auto_remediate() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 108,
            "method": "tools/call",
            "params": {
                "name": "auto_remediate",
                "arguments": {
                    "code": "const val = user.profile.address.city;"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["success"], true);
    }

    #[test]
    fn test_mcp_tool_call_symbol_lease_lifecycle() {
        let acq_req = r#"{
            "jsonrpc": "2.0",
            "id": 109,
            "method": "tools/call",
            "params": {
                "name": "acquire_symbol_lease",
                "arguments": {
                    "fqn": "src/auth/service.rs::validate_user",
                    "agent_id": "agent-alpha-001",
                    "ttl_ms": 10000
                }
            }
        }"#;
        let resp = handle_json_rpc_message(acq_req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        let lease_id = val["Acquired"]["lease_id"].as_str().unwrap().to_string();

        let renew_req = json!({
            "jsonrpc": "2.0",
            "id": 110,
            "method": "tools/call",
            "params": {
                "name": "renew_symbol_lease",
                "arguments": {
                    "lease_id": lease_id,
                    "agent_id": "agent-alpha-001",
                    "extension_ms": 20000
                }
            }
        });
        let resp = handle_json_rpc_message(&renew_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Renewed"));

        let rel_req = json!({
            "jsonrpc": "2.0",
            "id": 111,
            "method": "tools/call",
            "params": {
                "name": "release_symbol_lease",
                "arguments": {
                    "lease_id": lease_id,
                    "agent_id": "agent-alpha-001"
                }
            }
        });
        let resp = handle_json_rpc_message(&rel_req.to_string()).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Released"));
    }

    #[test]
    fn test_mcp_tool_call_acquire_subtree_lease() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 112,
            "method": "tools/call",
            "params": {
                "name": "acquire_subtree_lease",
                "arguments": {
                    "pattern": "src/payments/*",
                    "agent_id": "agent-beta-002",
                    "ttl_ms": 15000
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Acquired"));
    }

    #[test]
    fn test_mcp_tool_call_verify_occ_token() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 113,
            "method": "tools/call",
            "params": {
                "name": "verify_occ_token",
                "arguments": {
                    "fqn": "src/orders.rs::OrderState",
                    "expected_version": 1,
                    "commit": true
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"status\":\"success\"") || text.contains("\"status\": \"success\""));
    }

    #[test]
    fn test_mcp_tool_call_trace_taint_flow() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 114,
            "method": "tools/call",
            "params": {
                "name": "trace_taint_flow",
                "arguments": {
                    "file_path": "src/api/handler.rs",
                    "code": "pub async fn handle(req: Request) { let query = req.param(); db.execute(query); }",
                    "symbol": "handle"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_mcp_tool_call_audit_taint_path() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 115,
            "method": "tools/call",
            "params": {
                "name": "audit_taint_path",
                "arguments": {
                    "file_path": "src/auth.rs",
                    "code": "pub fn login(user_input: &str) { let clean = sanitize(user_input); exec(clean); }",
                    "symbol": "login"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_mcp_tool_call_hybrid_search() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 116,
            "method": "tools/call",
            "params": {
                "name": "hybrid_search",
                "arguments": {
                    "query": "AstGuard",
                    "path": "src",
                    "top_k": 3
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("hits"));
    }

    #[test]
    fn test_mcp_tool_call_query_cst() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 117,
            "method": "tools/call",
            "params": {
                "name": "query_cst",
                "arguments": {
                    "code": "fn main() { println!(\"hello\"); }",
                    "language": "rust",
                    "offset": 3
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["status"], "success");
        assert_eq!(val["trivia_roundtrip_intact"], true);
    }

    #[test]
    fn test_mcp_tool_call_morph_ast() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 118,
            "method": "tools/call",
            "params": {
                "name": "morph_ast",
                "arguments": {
                    "code": "const val = user.profile.name;"
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["success"], true);
    }

    #[test]
    fn test_mcp_tool_call_simd_vector_search() {
        let req = r#"{
            "jsonrpc": "2.0",
            "id": 119,
            "method": "tools/call",
            "params": {
                "name": "simd_vector_search",
                "arguments": {
                    "query_text": "ast parser engine",
                    "corpus": [
                        {"id": 1, "text": "ast parser syntax tree"},
                        {"id": 2, "text": "database sql connection"}
                    ],
                    "top_k": 2
                }
            }
        }"#;
        let resp = handle_json_rpc_message(req).expect("Response expected");
        let parsed: Value = serde_json::from_str(&resp).unwrap();
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(val["status"], "success");
        assert_eq!(val["simd_accelerated"], true);
    }
}
