//! MCP Tool #30: `semantic_analyze` Implementation.
//!
//! Executes full semantic invariant analysis (Rules S1..S5), synthesizes mathematical proof cards,
//! detects falsifying counterexamples, and formats auto-fix patches with optional OASIS SARIF v2.1.0 export.

#![forbid(unsafe_code)]

use serde_json::{json, Value};
use std::fs;

use crate::cst::polyglot::types::LanguageId;
use crate::semantic::analyzer::LocusSemanticAnalyzer;
use crate::semantic::sarif_bridge::SemanticSarifBridge;

/// Handles invocation of the `semantic_analyze` MCP tool.
pub fn handle_semantic_analyze(args: &Value) -> Result<Value, String> {
    let code = if let Some(c) = args.get("code").and_then(|s| s.as_str()) {
        c.to_string()
    } else if let Some(p) = args.get("path").and_then(|s| s.as_str()) {
        fs::read_to_string(p)
            .map_err(|e| format!("Failed to read source file at '{}': {}", p, e))?
    } else {
        return Err("Missing required argument: specify either 'code' or 'path'".to_string());
    };

    let lang_str = args
        .get("language")
        .and_then(|s| s.as_str())
        .unwrap_or("typescript");

    let lang = match lang_str.to_lowercase().as_str() {
        "rust" | "rs" => LanguageId::Rust,
        "typescript" | "ts" => LanguageId::TypeScript,
        "javascript" | "js" => LanguageId::JavaScript,
        "tsx" => LanguageId::Tsx,
        "jsx" => LanguageId::Jsx,
        "dart" => LanguageId::Dart,
        "python" | "py" => LanguageId::Python,
        "php" => LanguageId::Php,
        "java" => LanguageId::Java,
        "go" => LanguageId::Go,
        "c" => LanguageId::C,
        "cpp" | "c++" => LanguageId::Cpp,
        _ => LanguageId::from_extension(lang_str),
    };

    let file_uri = args
        .get("path")
        .and_then(|s| s.as_str())
        .unwrap_or("snippet.ts");

    let analyzer = LocusSemanticAnalyzer::new();
    let report = analyzer.analyze_source(&code, lang);

    let include_sarif = args
        .get("sarif")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut response = json!({
        "status": if report.passed { "passed" } else { "violations_detected" },
        "passed": report.passed,
        "total_rules_evaluated": report.total_rules_evaluated,
        "violations_count": report.violations_count,
        "proofs": report.proofs,
        "execution_latency_ms": report.execution_latency_ms,
    });

    if include_sarif {
        let sarif_log = SemanticSarifBridge::to_sarif_log(&report, file_uri);
        response["sarifLog"] = serde_json::to_value(&sarif_log).unwrap_or(json!({}));
    }

    Ok(response)
}
