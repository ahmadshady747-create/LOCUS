//! MCP Tool Bindings for Phase 1.5-A.
//!
//! Exposes ACID multi-file workspace transactions and deterministic auto-remediation
//! to Model Context Protocol (MCP) clients (Claude Code, Cursor, Windsurf, Antigravity).

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::LazyLock;
use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::remediate::AutoFixer;
use crate::tx::WorkspaceTransaction;
use crate::types::Language;

/// Global in-memory transaction registry for active MCP transactions.
static TX_REGISTRY: LazyLock<Mutex<HashMap<String, WorkspaceTransaction>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Handle `begin_tx` MCP tool invocation.
pub fn handle_begin_tx(_args: &Value) -> Result<Value, String> {
    let tx = WorkspaceTransaction::begin();
    let tx_id = tx.id.0.clone();

    let mut reg = TX_REGISTRY.lock();
    reg.insert(tx_id.clone(), tx);

    Ok(json!({
        "status": "success",
        "tx_id": tx_id,
        "message": "ACID Workspace Transaction opened in-memory."
    }))
}

/// Handle `stage_tx` MCP tool invocation.
pub fn handle_stage_tx(args: &Value) -> Result<Value, String> {
    let tx_id = args.get("tx_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'tx_id'".to_string())?;

    let file_path = args.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'file_path'".to_string())?;

    let content = args.get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'content'".to_string())?;

    let lang_str = args.get("language").and_then(|v| v.as_str()).unwrap_or("");
    let language = if lang_str.is_empty() {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        Language::from_extension(ext)
    } else {
        Language::from_extension(lang_str)
    };

    let mut reg = TX_REGISTRY.lock();
    let tx = reg.get_mut(tx_id)
        .ok_or_else(|| format!("Transaction '{}' not found or expired", tx_id))?;

    tx.stage_file(file_path, content, language)?;

    Ok(json!({
        "status": "success",
        "tx_id": tx_id,
        "staged_file": file_path,
        "language": language.to_string()
    }))
}

/// Handle `commit_tx` MCP tool invocation.
pub fn handle_commit_tx(args: &Value) -> Result<Value, String> {
    let tx_id = args.get("tx_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'tx_id'".to_string())?;

    let mut reg = TX_REGISTRY.lock();
    let mut tx = reg.remove(tx_id)
        .ok_or_else(|| format!("Transaction '{}' not found", tx_id))?;

    let report = tx.commit();

    Ok(json!({
        "tx_id": report.tx_id.0,
        "status": format!("{:?}", report.status),
        "passed_verification": report.passed_verification,
        "total_staged_files": report.total_staged_files,
        "committed_files": report.committed_files,
        "violations": report.violations,
        "latency_ms": report.latency_ms
    }))
}

/// Handle `rollback_tx` MCP tool invocation.
pub fn handle_rollback_tx(args: &Value) -> Result<Value, String> {
    let tx_id = args.get("tx_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'tx_id'".to_string())?;

    let mut reg = TX_REGISTRY.lock();
    let mut tx = reg.remove(tx_id)
        .ok_or_else(|| format!("Transaction '{}' not found", tx_id))?;

    let report = tx.rollback();

    Ok(json!({
        "tx_id": report.tx_id.0,
        "status": "RolledBack",
        "total_staged_files": report.total_staged_files,
        "latency_ms": report.latency_ms
    }))
}

/// Handle `auto_remediate` MCP tool invocation.
pub fn handle_auto_remediate(args: &Value) -> Result<Value, String> {
    let code = args.get("code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'code'".to_string())?;

    let result = AutoFixer::remediate(code);

    Ok(json!({
        "success": result.success,
        "remediated_code": result.remediated_code,
        "passed_verification": result.passed_verification,
        "edits_applied": result.edits_applied,
        "latency_ms": result.latency_ms
    }))
}
