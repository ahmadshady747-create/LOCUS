//! Inter-Procedural SSA Taint & Data-Flow Dependency Graph (v2).
//!
//! Provides inter-procedural taint analysis across functions and modules using
//! a directed Call Graph G = (V, E), Sanitizer Proof Chains, and SHA-256
//! TaintAuditCertificates.

#![forbid(unsafe_code)]

use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::LazyLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::cache::AstContextCache;
use crate::types::{
    RiskScore, SanitizerRule, TaintAuditCertificate, TaintFlowReport, TaintKind, TaintSink,
    TaintSource, TxStagedFile,
};

// ---------------------------------------------------------------------------
// Compiled Detection Patterns
// ---------------------------------------------------------------------------

static RE_TAINT_SOURCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:let|const|var|mut)\s+([a-zA-Z0-9_$]+)\s*=\s*(?:req(?:\?|\.)(?:params|query|body|headers)|params\.|query\.|userInput|user_input|user_path|file_param|raw_input|client_target|input_data|process\.env|import\.meta\.env)"#).unwrap()
});

static RE_SENSITIVE_SINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:fs\.(?:readFile|readFileSync|writeFile|writeFileSync|unlink|open)|std::fs::(?:read|write)|db\.(?:query|execute)|eval|new\s+Function|fetch|axios\.(?:get|post|put|delete)|child_process\.(?:exec|spawn)|Command::new)\s*\([^)]*?\b([a-zA-Z0-9_$]+)\b|(?:\.?innerHTML\s*=\s*|dangerouslySetInnerHTML\s*=\s*\{[^{}]*__html\s*:\s*)\b([a-zA-Z0-9_$]+)\b"#).unwrap()
});

static RE_FUNCTION_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(?:export\s+)?(?:pub\s+)?(?:async\s+)?(?:function|fn)\s+([a-zA-Z0-9_$]+)\s*\(([^)]*)\)"#).unwrap()
});

static RE_CALL_INVOCATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b([a-zA-Z0-9_$]+)\s*\(([^)]*)\)"#).unwrap()
});

static RE_ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:let|const|var|mut)?\s*([a-zA-Z0-9_$]+)\s*=\s*(?:await\s+)?([^;]+);"#).unwrap()
});

static RE_RETURN_STMT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\breturn\s+(?:await\s+)?([^;]+);"#).unwrap()
});

// Sanitizer regexes
static RE_SANITIZER_DOMPURIFY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:DOMPurify\.sanitize|escapeHtml|escape_html|htmlentities|encodeHTML)\s*\(\s*([a-zA-Z0-9_$]+)\s*\)"#).unwrap()
});

static RE_SANITIZER_SQL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:sanitize_sql|sqlx::query!|sqlx::query_as!|prepared_statement|db\.prepare|sql_param)\s*\(\s*([a-zA-Z0-9_$]+)"#).unwrap()
});

static RE_SANITIZER_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:encodeURIComponent|encodeURI|urlencoding::encode)\s*\(\s*([a-zA-Z0-9_$]+)\s*\)"#).unwrap()
});

static RE_SANITIZER_CRYPTO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:bcrypt_hash|argon2|hash_password|sha256|hmac|ConstantTimeEq|timingSafeEqual|constant_time_eq)\s*\(\s*([a-zA-Z0-9_$]+)"#).unwrap()
});

static RE_SANITIZER_SCHEMA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:zod\.parse|z\.string\(\)\.parse|validator\.[a-zA-Z0-9_$]+|validate_input|sanitize_input|validate_path|path\.normalize|clean_path)\s*\(\s*([a-zA-Z0-9_$]+)\s*\)"#).unwrap()
});

// ---------------------------------------------------------------------------
// Call Graph & SSA Node / Edge Representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Source(TaintKind),
    Parameter { func_name: String, index: usize },
    Argument { call_site: String, index: usize },
    LocalVar,
    Return { func_name: String },
    Sanitizer { name: String, rule: SanitizerRule },
    Sink { operation: String },
}

#[derive(Debug, Clone)]
pub struct TaintNode {
    pub id: usize,
    pub file: String,
    pub symbol: String,
    pub variable: String,
    pub line: usize,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
    Assign,
    CallArgument,
    ReturnTransfer,
    SanitizingTransfer(SanitizerRule),
}

#[derive(Debug, Clone)]
pub struct TaintEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

/// Directed Call & Data-Flow Graph G = (V, E).
#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    pub nodes: Vec<TaintNode>,
    pub edges: Vec<TaintEdge>,
    adj: HashMap<usize, Vec<(usize, EdgeKind)>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(
        &mut self,
        file: &str,
        symbol: &str,
        variable: &str,
        line: usize,
        kind: NodeKind,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TaintNode {
            id,
            file: file.to_string(),
            symbol: symbol.to_string(),
            variable: variable.to_string(),
            line,
            kind,
        });
        id
    }

    pub fn add_edge(&mut self, from: usize, to: usize, kind: EdgeKind) {
        self.edges.push(TaintEdge {
            from,
            to,
            kind: kind.clone(),
        });
        self.adj.entry(from).or_default().push((to, kind));
    }

    pub fn neighbors(&self, u: usize) -> &[(usize, EdgeKind)] {
        self.adj.get(&u).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

// ---------------------------------------------------------------------------
// Inter-Procedural Data-Flow Tracker (v2)
// ---------------------------------------------------------------------------

pub struct DataFlowTracker;

impl DataFlowTracker {
    /// Trace taint flow within a single source content string.
    pub fn analyze_source(file_path: &str, symbol: &str, content: &str) -> Vec<TaintFlowReport> {
        let staged = vec![TxStagedFile {
            path: file_path.to_string(),
            original_content: None,
            staged_content: content.to_string(),
            language: crate::types::Language::from_extension(file_path),
        }];
        Self::analyze_owned_files(&staged)
            .into_iter()
            .filter(|r| r.source.file == file_path && (symbol == "*" || r.source.symbol == symbol))
            .collect()
    }

    /// Convenience wrapper for owned `TxStagedFile` slices.
    pub fn analyze_owned_files(staged_files: &[TxStagedFile]) -> Vec<TaintFlowReport> {
        let refs: Vec<&TxStagedFile> = staged_files.iter().collect();
        Self::analyze_workspace_files(&refs)
    }

    /// Inter-procedural SSA analysis across all files staged in a workspace transaction.
    pub fn analyze_workspace_files(staged_files: &[&TxStagedFile]) -> Vec<TaintFlowReport> {
        let start = Instant::now();
        let mut graph = CallGraph::new();
        let mut function_params: HashMap<String, (String, Vec<String>)> = HashMap::new(); // func_name -> (file, param_names)
        let mut func_param_nodes: HashMap<(String, String), usize> = HashMap::new(); // (func_name, param_name) -> node_id
        let mut var_nodes: HashMap<(String, String, String), usize> = HashMap::new(); // (file, symbol, var_name) -> node_id

        // Phase 1: Function signatures and parameter registration
        for file in staged_files {
            for cap in RE_FUNCTION_DEF.captures_iter(&file.staged_content) {
                if let (Some(fn_name), Some(raw_params)) = (cap.get(1), cap.get(2)) {
                    let name = fn_name.as_str().trim().to_string();
                    let params = raw_params
                        .as_str()
                        .split(',')
                        .map(|p| {
                            let clean = p.split(':').next().unwrap_or(p).trim();
                            clean
                                .trim_start_matches("mut ")
                                .trim_start_matches('&')
                                .trim()
                                .to_string()
                        })
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>();

                    for (idx, p) in params.iter().enumerate() {
                        let node_id = graph.add_node(
                            &file.path,
                            &name,
                            p,
                            0,
                            NodeKind::Parameter {
                                func_name: name.clone(),
                                index: idx,
                            },
                        );
                        func_param_nodes.insert((name.clone(), p.clone()), node_id);
                        var_nodes.insert((file.path.clone(), name.clone(), p.clone()), node_id);
                    }

                    function_params.insert(name, (file.path.clone(), params));
                }
            }
        }

        // Phase 2: Parse statements, sources, sanitizers, and assignments
        for file in staged_files {
            let mut current_symbol = "global".to_string();

            for (line_idx, line) in file.staged_content.lines().enumerate() {
                let line_num = line_idx + 1;

                if let Some(fn_cap) = RE_FUNCTION_DEF.captures(line) {
                    if let Some(fn_name) = fn_cap.get(1) {
                        current_symbol = fn_name.as_str().trim().to_string();
                    }
                }

                // 1. Identify Taint Sources
                if let Some(src_cap) = RE_TAINT_SOURCE.captures(line) {
                    if let Some(var_match) = src_cap.get(1) {
                        let var_name = var_match.as_str().to_string();
                        let src_node = graph.add_node(
                            &file.path,
                            &current_symbol,
                            &var_name,
                            line_num,
                            NodeKind::Source(TaintKind::UnvalidatedInput),
                        );
                        var_nodes.insert(
                            (file.path.clone(), current_symbol.clone(), var_name.clone()),
                            src_node,
                        );
                    }
                }

                // 2. Identify Sanitizer Transformations
                let mut detected_sanitizer = None;
                if let Some(cap) = RE_SANITIZER_DOMPURIFY.captures(line) {
                    if let Some(in_var) = cap.get(1) {
                        detected_sanitizer = Some((
                            "DOMPurify.sanitize".to_string(),
                            SanitizerRule::HtmlSanitization,
                            in_var.as_str().to_string(),
                        ));
                    }
                } else if let Some(cap) = RE_SANITIZER_SQL.captures(line) {
                    if let Some(in_var) = cap.get(1) {
                        detected_sanitizer = Some((
                            "sanitize_sql".to_string(),
                            SanitizerRule::SqlParamBinding,
                            in_var.as_str().to_string(),
                        ));
                    }
                } else if let Some(cap) = RE_SANITIZER_URL.captures(line) {
                    if let Some(in_var) = cap.get(1) {
                        detected_sanitizer = Some((
                            "encodeURIComponent".to_string(),
                            SanitizerRule::UrlEncoding,
                            in_var.as_str().to_string(),
                        ));
                    }
                } else if let Some(cap) = RE_SANITIZER_CRYPTO.captures(line) {
                    if let Some(in_var) = cap.get(1) {
                        detected_sanitizer = Some((
                            "ConstantTimeCrypto".to_string(),
                            SanitizerRule::CryptoHashing,
                            in_var.as_str().to_string(),
                        ));
                    }
                } else if let Some(cap) = RE_SANITIZER_SCHEMA.captures(line) {
                    if let Some(in_var) = cap.get(1) {
                        detected_sanitizer = Some((
                            "SchemaValidator".to_string(),
                            SanitizerRule::SchemaValidation,
                            in_var.as_str().to_string(),
                        ));
                    }
                }

                if let Some((san_name, rule, in_var)) = detected_sanitizer {
                    if let Some(assign_cap) = RE_ASSIGNMENT.captures(line) {
                        if let Some(out_var_match) = assign_cap.get(1) {
                            let out_var = out_var_match.as_str().to_string();
                            let san_node = graph.add_node(
                                &file.path,
                                &current_symbol,
                                &out_var,
                                line_num,
                                NodeKind::Sanitizer {
                                    name: san_name,
                                    rule: rule.clone(),
                                },
                            );
                            var_nodes.insert(
                                (file.path.clone(), current_symbol.clone(), out_var.clone()),
                                san_node,
                            );

                            if let Some(&in_node) = var_nodes.get(&(
                                file.path.clone(),
                                current_symbol.clone(),
                                in_var.clone(),
                            )) {
                                graph.add_edge(
                                    in_node,
                                    san_node,
                                    EdgeKind::SanitizingTransfer(rule),
                                );
                            }
                        }
                    }
                } else if let Some(assign_cap) = RE_ASSIGNMENT.captures(line) {
                    // Standard Local Variable Assignment: let target = expr;
                    if let (Some(target), Some(expr)) = (assign_cap.get(1), assign_cap.get(2)) {
                        let target_var = target.as_str().to_string();
                        let expr_str = expr.as_str().trim();

                        let target_node = *var_nodes
                            .entry((file.path.clone(), current_symbol.clone(), target_var.clone()))
                            .or_insert_with(|| {
                                graph.add_node(
                                    &file.path,
                                    &current_symbol,
                                    &target_var,
                                    line_num,
                                    NodeKind::LocalVar,
                                )
                            });

                        // Check if expr references another var
                        for (&(ref f, ref s, ref src_v), &src_node) in &var_nodes {
                            if f == &file.path
                                && s == &current_symbol
                                && src_v != &target_var
                                && expr_str.contains(src_v.as_str())
                            {
                                graph.add_edge(src_node, target_node, EdgeKind::Assign);
                            }
                        }
                    }
                }

                // 3. Inter-Procedural Call Invocations: callee(arg1, arg2)
                for call_cap in RE_CALL_INVOCATION.captures_iter(line) {
                    if let (Some(callee), Some(args_raw)) = (call_cap.get(1), call_cap.get(2)) {
                        let callee_name = callee.as_str().trim();
                        if let Some((_callee_file, params)) = function_params.get(callee_name) {
                            let passed_args = args_raw
                                .as_str()
                                .split(',')
                                .map(|a| a.trim().trim_start_matches('&').trim())
                                .collect::<Vec<_>>();

                            for (idx, arg) in passed_args.iter().enumerate() {
                                if idx < params.len() {
                                    let param_name = &params[idx];
                                    if let Some(&arg_node) = var_nodes.get(&(
                                        file.path.clone(),
                                        current_symbol.clone(),
                                        arg.to_string(),
                                    )) {
                                        if let Some(&param_node) = func_param_nodes
                                            .get(&(callee_name.to_string(), param_name.clone()))
                                        {
                                            graph.add_edge(
                                                arg_node,
                                                param_node,
                                                EdgeKind::CallArgument,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 4. Return statement transfer
                if let Some(ret_cap) = RE_RETURN_STMT.captures(line) {
                    if let Some(ret_expr) = ret_cap.get(1) {
                        let ret_var = ret_expr.as_str().trim().to_string();
                        let ret_node = graph.add_node(
                            &file.path,
                            &current_symbol,
                            &ret_var,
                            line_num,
                            NodeKind::Return {
                                func_name: current_symbol.clone(),
                            },
                        );

                        if let Some(&src_node) = var_nodes.get(&(
                            file.path.clone(),
                            current_symbol.clone(),
                            ret_var.clone(),
                        )) {
                            graph.add_edge(src_node, ret_node, EdgeKind::ReturnTransfer);
                        }
                    }
                }

                // 5. Sensitive Sinks
                if let Some(sink_cap) = RE_SENSITIVE_SINK.captures(line) {
                    let matched_arg = sink_cap.get(1).or_else(|| sink_cap.get(2));
                    if let Some(arg_match) = matched_arg {
                        let sink_arg = arg_match.as_str().to_string();
                        let sink_node = graph.add_node(
                            &file.path,
                            &current_symbol,
                            &sink_arg,
                            line_num,
                            NodeKind::Sink {
                                operation: line.trim().to_string(),
                            },
                        );

                        if let Some(&src_node) = var_nodes.get(&(
                            file.path.clone(),
                            current_symbol.clone(),
                            sink_arg.clone(),
                        )) {
                            graph.add_edge(src_node, sink_node, EdgeKind::Assign);
                        }
                    }
                }
            }
        }

        // Phase 3: Graph Traversal & Sanitizer Proof Chain Validation
        let mut reports = Vec::new();

        for source_node in &graph.nodes {
            if let NodeKind::Source(ref taint_kind) = source_node.kind {
                // BFS to find reachable sinks
                let mut queue = VecDeque::new();
                let mut visited = HashSet::new();
                queue.push_back((source_node.id, vec![source_node.id], false, None));
                visited.insert(source_node.id);

                while let Some((curr, path, has_sanitizer, san_info)) = queue.pop_front() {
                    let curr_node = &graph.nodes[curr];

                    if let NodeKind::Sink { ref operation } = curr_node.kind {
                        let flow_path_str = path
                            .iter()
                            .map(|&idx| {
                                let n = &graph.nodes[idx];
                                format!("{}:{}:{}", n.file, n.symbol, n.variable)
                            })
                            .collect::<Vec<_>>();

                        let is_sanitized = has_sanitizer;
                        let violation_risk = if is_sanitized {
                            RiskScore::Low
                        } else {
                            RiskScore::High
                        };

                        let certificate = if is_sanitized {
                            let (san_name, san_rule) = san_info.clone().unwrap_or((
                                "GenericSanitizer".to_string(),
                                SanitizerRule::Custom("sanitized".to_string()),
                            ));
                            Some(TaintAuditCertificate::generate(
                                &format!("taint_{}_{}", source_node.symbol, source_node.variable),
                                &source_node.variable,
                                &san_name,
                                san_rule,
                                operation,
                                flow_path_str.clone(),
                            ))
                        } else {
                            None
                        };

                        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

                        reports.push(TaintFlowReport {
                            taint_id: format!(
                                "taint_{}_{}",
                                source_node.symbol, source_node.variable
                            ),
                            source: TaintSource {
                                file: source_node.file.clone(),
                                symbol: source_node.symbol.clone(),
                                variable: source_node.variable.clone(),
                                kind: taint_kind.clone(),
                            },
                            flow_path: flow_path_str,
                            sinks: vec![TaintSink {
                                file: curr_node.file.clone(),
                                symbol: curr_node.symbol.clone(),
                                line: curr_node.line,
                                operation: operation.clone(),
                            }],
                            is_sanitized,
                            violation_risk,
                            certificate,
                            latency_ms,
                        });
                    }

                    for &(next, ref edge_kind) in graph.neighbors(curr) {
                        let next_node = &graph.nodes[next];
                        let next_sanitizer = has_sanitizer
                            || matches!(edge_kind, EdgeKind::SanitizingTransfer(_))
                            || matches!(next_node.kind, NodeKind::Sanitizer { .. });

                        let next_san_info = if let NodeKind::Sanitizer {
                            ref name,
                            ref rule,
                        } = next_node.kind
                        {
                            Some((name.clone(), rule.clone()))
                        } else if let EdgeKind::SanitizingTransfer(ref rule) = edge_kind {
                            Some(("SanitizerTransfer".to_string(), rule.clone()))
                        } else {
                            san_info.clone()
                        };

                        let mut next_path = path.clone();
                        next_path.push(next);
                        queue.push_back((next, next_path, next_sanitizer, next_san_info));
                    }
                }
            }
        }

        reports
    }
}

// ---------------------------------------------------------------------------
// Taint Audit Certificate Generator
// ---------------------------------------------------------------------------

impl TaintAuditCertificate {
    /// Generate a cryptographic SHA-256 certificate for a verified clean taint flow.
    pub fn generate(
        taint_id: &str,
        source_var: &str,
        sanitizer_name: &str,
        sanitizer_rule: SanitizerRule,
        sink_op: &str,
        proof_chain: Vec<String>,
    ) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let canonical_str = format!(
            "CERT:taint_id={}:src={}:sanitizer={}:rule={:?}:sink={}:chain={}:timestamp={}",
            taint_id,
            source_var,
            sanitizer_name,
            sanitizer_rule,
            sink_op,
            proof_chain.join("->"),
            now_ms
        );

        let digest = AstContextCache::sha256_digest(canonical_str.as_bytes());
        let sha256_hex = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let cert_id = format!("cert_{}", &sha256_hex[0..16]);

        Self {
            certificate_id: cert_id,
            taint_id: taint_id.to_string(),
            source_variable: source_var.to_string(),
            sanitizer_name: sanitizer_name.to_string(),
            sanitizer_rule,
            sink_operation: sink_op.to_string(),
            proof_chain,
            sha256_fingerprint: sha256_hex,
            issued_at_ms: now_ms,
        }
    }
}
