//! Tauri IPC commands for Directive-Bound Bidirectional Formal Verifier.

use locus_agents::verifier::BidirectionalVerifier;
use locus_core::{CodeContract, VerificationVerdict};

#[tauri::command]
pub fn verifier_prove_contract(
    _file_path: String,
    patch_content: String,
    contract: Option<CodeContract>,
) -> Result<VerificationVerdict, String> {
    let verifier = BidirectionalVerifier::new();

    let actual_contract = match contract {
        Some(c) if !c.requires.is_empty() || !c.ensures.is_empty() || !c.invariants.is_empty() => c,
        _ => BidirectionalVerifier::extract_baseline_contract(&patch_content, None),
    };

    Ok(verifier.verify(&patch_content, &actual_contract))
}

#[tauri::command]
pub fn verifier_get_active_invariants(_workspace_root: String) -> Result<Vec<String>, String> {
    Ok(vec![
        "ArrayBound: 0 <= index < len".to_string(),
        "DivisionSafety: divisor != 0".to_string(),
        "MemorySafety: ptr != null".to_string(),
        "FrameRule: pure functions preserve unreferenced state".to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_ipc_command_safe() {
        let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let res = verifier_prove_contract("src/lib.rs".to_string(), code.to_string(), None);
        assert!(res.is_ok());
        let verdict = res.unwrap();
        assert!(verdict.is_bidirectionally_verified);
    }
}
