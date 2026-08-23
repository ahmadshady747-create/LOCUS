//! Static Option / Null Propagation Tracker.

#![forbid(unsafe_code)]

use std::sync::LazyLock;
use std::time::Instant;
use regex::Regex;

use crate::types::{RiskScore, TaintFlowReport, TaintKind, TaintSink, TaintSource};

static RE_OPTION_RETURN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*\([^)]*\)\s*->\s*(?:Option<|Result<[^,]+,\s*[^>]+>)"#).unwrap()
});

static RE_NULLABLE_TS_RETURN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z0-9_$]+)\s*\([^)]*\)\s*:\s*[^;\n]+\|\s*(?:null|undefined)"#).unwrap()
});

pub struct NullPropagationTracker;

impl NullPropagationTracker {
    /// Detect unhandled Option/Null returns across call sites.
    pub fn scan_nullable_flows(file_path: &str, content: &str) -> Vec<TaintFlowReport> {
        let start = Instant::now();
        let mut reports = Vec::new();

        let mut nullable_funcs = Vec::new();
        for cap in RE_OPTION_RETURN.captures_iter(content) {
            if let Some(fn_match) = cap.get(1) {
                nullable_funcs.push(fn_match.as_str().to_string());
            }
        }
        for cap in RE_NULLABLE_TS_RETURN.captures_iter(content) {
            if let Some(fn_match) = cap.get(1) {
                nullable_funcs.push(fn_match.as_str().to_string());
            }
        }

        for fn_name in nullable_funcs {
            let call_pattern = format!("{}(", fn_name);
            for (idx, line) in content.lines().enumerate() {
                if line.contains(&call_pattern) && !line.contains("fn ") && !line.contains("function ") {
                    // Check if line contains unhandled direct unwrap or direct property access
                    let is_guarded = line.contains("if let") || line.contains("match ")
                        || line.contains("?.") || line.contains(".unwrap_or")
                        || line.contains("if (") || line.contains('?');

                    if !is_guarded {
                        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                        reports.push(TaintFlowReport {
                            taint_id: format!("null_prop_{}_{}", fn_name, idx),
                            source: TaintSource {
                                file: file_path.to_string(),
                                symbol: fn_name.clone(),
                                variable: fn_name.clone(),
                                kind: TaintKind::NullableReturn,
                            },
                            flow_path: vec![format!("{}:line_{}", file_path, idx + 1)],
                            sinks: vec![TaintSink {
                                file: file_path.to_string(),
                                symbol: fn_name.clone(),
                                line: idx + 1,
                                operation: line.trim().to_string(),
                            }],
                            is_sanitized: false,
                            violation_risk: RiskScore::Medium,
                            latency_ms: elapsed_ms,
                        });
                    }
                }
            }
        }

        reports
    }
}
