//! Lightweight Cross-File Taint & Data-Flow Dependency Graph.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use std::time::Instant;
use regex::Regex;

use crate::types::{RiskScore, TaintFlowReport, TaintKind, TaintSink, TaintSource};

static RE_TAINT_SOURCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:let|const|var)\s+([a-zA-Z0-9_$]+)\s*=\s*(?:req\.(?:params|query|body)|params\.|userInput|user_path|process\.env|import\.meta\.env)"#).unwrap()
});

static RE_SENSITIVE_SINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:fs\.(?:readFile|readFileSync|writeFile|unlink)|db\.(?:query|execute)|eval|new\s+Function|fetch|axios)\s*\([^)]*?\b([a-zA-Z0-9_$]+)\b"#).unwrap()
});

pub struct DataFlowTracker;

impl DataFlowTracker {
    /// Trace taint flow within source content or across related caller files.
    pub fn analyze_source(file_path: &str, symbol: &str, content: &str) -> Vec<TaintFlowReport> {
        let start = Instant::now();
        let mut reports = Vec::new();

        // 1. Identify taint sources
        let mut tainted_vars = Vec::new();
        for cap in RE_TAINT_SOURCE.captures_iter(content) {
            if let Some(var_match) = cap.get(1) {
                let var_name = var_match.as_str().to_string();
                tainted_vars.push(TaintSource {
                    file: file_path.to_string(),
                    symbol: symbol.to_string(),
                    variable: var_name,
                    kind: TaintKind::UnvalidatedInput,
                });
            }
        }

        // 2. Trace propagation to sensitive sinks
        for src in &tainted_vars {
            let mut sinks = Vec::new();
            for (line_idx, line) in content.lines().enumerate() {
                if line.contains(&src.variable) {
                    if let Some(sink_cap) = RE_SENSITIVE_SINK.captures(line) {
                        if let Some(arg_match) = sink_cap.get(1) {
                            if arg_match.as_str() == src.variable {
                                sinks.push(TaintSink {
                                    file: file_path.to_string(),
                                    symbol: symbol.to_string(),
                                    line: line_idx + 1,
                                    operation: line.trim().to_string(),
                                });
                            }
                        }
                    }
                }
            }

            if !sinks.is_empty() {
                let is_sanitized = content.contains("sanitize") || content.contains("validate") || content.contains("escape");
                let violation_risk = if is_sanitized { RiskScore::Low } else { RiskScore::High };
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

                reports.push(TaintFlowReport {
                    taint_id: format!("taint_{}_{}", src.symbol, src.variable),
                    source: src.clone(),
                    flow_path: vec![format!("{}:{}", file_path, src.variable)],
                    sinks,
                    is_sanitized,
                    violation_risk,
                    latency_ms: elapsed_ms,
                });
            }
        }

        reports
    }
}
