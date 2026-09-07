//! Comprehensive Integration & Stress Test Suite for the Dynamic Proof Verifier Subsystem (Task 10.3).
//!
//! Validates end-to-end AST proof evaluation, cryptographic certificate minting,
//! MCP Tool #30 (`verify_proof`) JSON-RPC bridge execution, transactional proof gating,
//! and high-throughput stress budgets in 100% pure safe Rust (#![forbid(unsafe_code)]).

#![forbid(unsafe_code)]

use locus_engine::cst::parse_to_green;
use locus_engine::cst::polyglot::{GreenTreeRoot, LanguageId};
use locus_engine::mcp::handle_json_rpc_message;
use locus_engine::proof_verifier::{
    ProofAssertion, ProofClaim, ProofVerifierEngine, PROOF_VERIFIER_VERSION,
};
use locus_engine::tx::WorkspaceTransaction;
use locus_engine::types::Language;
use serde_json::Value;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Scenario 1: Multi-Pass Valid Claim & Cryptographic Minting
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_multi_pass_valid_claim_and_cryptographic_minting() {
    let source_code = r#"
        /// Sovereign Vault Access Gate
        pub async fn authorize_vault_access(client_token: &str, target_vault: &str) -> Result<String, VaultError> {
            let sanitized_token = DOMPurify.sanitize(client_token);
            let sanitized_vault = clean_path(target_vault);
            let audit_log = format!("ACCESS_GRANTED:{}->{}", sanitized_token, sanitized_vault);
            Ok(audit_log)
        }
    "#;

    let mut claim = ProofClaim::new(
        "claim-vault-auth-001",
        "src/vault/auth.rs",
        "authorize_vault_access",
        "DeterministicVaultSanitizationInvariant",
        1725151000000,
    );

    // 1. Node Exists: Function Declaration
    claim.add_assertion(ProofAssertion::node_exists(
        "FunctionDeclaration",
        Some("authorize_vault_access"),
    ));

    // 2. Property Equals: Visibility is pub
    claim.add_assertion(ProofAssertion::property_equals(
        "authorize_vault_access",
        "visibility",
        "pub",
    ));

    // 3. Property Equals: Async keyword present
    claim.add_assertion(ProofAssertion::property_equals(
        "authorize_vault_access",
        "async",
        "true",
    ));

    // 4. Hierarchical Nesting: Function contains Block
    claim.add_assertion(ProofAssertion::contains_sub_node(
        "FunctionDeclaration",
        "Block",
    ));

    // 5. Data Flow Relation with Mandatory Sanitizer
    claim.add_assertion(ProofAssertion::data_flow_relation(
        "client_token",
        "audit_log",
        true,
    ));

    // 6. Range Intersection Disjoint Spans
    claim.add_assertion(ProofAssertion::range_intersection_empty((10, 80), (100, 250)));

    // Execute full verification pipeline
    let result = ProofVerifierEngine::verify_source(source_code, Some("rust"), &claim);

    assert!(
        result.is_proven(),
        "Valid multi-pass claim must evaluate to Proven"
    );
    assert_eq!(result.steps_evaluated, 6);
    assert!(result.verdict.is_proven());

    // Verify minted certificate
    let cert = result.certificate.expect("ProofCertificate must be minted on Proven");
    assert!(cert.certificate_id.starts_with("cert-"));
    assert!(cert.sha256_signature.starts_with("LOCUS-PROOF-v2:"));
    assert_eq!(cert.verifier_version, PROOF_VERIFIER_VERSION);
    assert_eq!(cert.claim_id, "claim-vault-auth-001");

    // Cryptographic self-verification
    let green = parse_to_green(source_code);
    let tree = GreenTreeRoot::new(green, LanguageId::Rust);
    let ast_hash = format!("{:016x}", tree.content_hash());

    assert!(
        cert.verify_signature("authorize_vault_access", "DeterministicVaultSanitizationInvariant", &ast_hash),
        "Minted certificate cryptographic signature must verify against matching parameters"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2: Instant Refutation on Hallucinated & Unsafe Claims
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_instant_refutation_on_hallucinated_and_unsafe_claims() {
    let source_code = r#"
        fn private_internal_transfer(raw_amount: i64) -> i64 {
            let unverified_sink = raw_amount;
            unverified_sink
        }
    "#;

    // Test A: Refute Missing (Hallucinated) Node
    {
        let mut claim_missing_fn = ProofClaim::new(
            "claim-hallucination-01",
            "src/transfer.rs",
            "execute_wire_transfer",
            "MustExist",
            1725151100000,
        );
        claim_missing_fn.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("execute_wire_transfer"),
        ));

        let res = ProofVerifierEngine::verify_source(source_code, Some("rust"), &claim_missing_fn);
        assert!(!res.is_proven());
        assert!(res.verdict.is_refuted());
        assert_eq!(res.certificate, None);
        assert_eq!(res.verdict.failed_assertion_index(), Some(0));
        let reason = res.verdict.failure_reason().unwrap();
        assert!(reason.contains("was not found"));
    }

    // Test B: Refute Visibility Mismatch (Expected pub, Found default/private)
    {
        let mut claim_visibility = ProofClaim::new(
            "claim-visibility-01",
            "src/transfer.rs",
            "private_internal_transfer",
            "PublicExportInvariant",
            1725151200000,
        );
        claim_visibility.add_assertion(ProofAssertion::property_equals(
            "private_internal_transfer",
            "visibility",
            "pub",
        ));

        let res = ProofVerifierEngine::verify_source(source_code, Some("rust"), &claim_visibility);
        assert!(!res.is_proven());
        assert!(res.verdict.is_refuted());
        assert_eq!(res.certificate, None);
        assert_eq!(res.verdict.failed_assertion_index(), Some(0));
        let reason = res.verdict.failure_reason().unwrap();
        assert!(reason.contains("mismatch"));
    }

    // Test C: Refute Unsanitized Data Flow Violation
    {
        let mut claim_taint = ProofClaim::new(
            "claim-taint-01",
            "src/transfer.rs",
            "private_internal_transfer",
            "SanitizedTransferInvariant",
            1725151300000,
        );
        claim_taint.add_assertion(ProofAssertion::data_flow_relation(
            "raw_amount",
            "unverified_sink",
            true, // Requires active sanitizer
        ));

        let res = ProofVerifierEngine::verify_source(source_code, Some("rust"), &claim_taint);
        assert!(!res.is_proven());
        assert!(res.verdict.is_refuted());
        assert_eq!(res.certificate, None);
        assert_eq!(res.verdict.failed_assertion_index(), Some(0));
        let reason = res.verdict.failure_reason().unwrap();
        assert!(reason.contains("sanitizer was NOT passed"));
    }

    // Test D: Refute Overlapping Token Ranges
    {
        let mut claim_overlap = ProofClaim::new(
            "claim-overlap-01",
            "src/transfer.rs",
            "private_internal_transfer",
            "MemoryDisjointness",
            1725151400000,
        );
        claim_overlap.add_assertion(ProofAssertion::range_intersection_empty((10, 50), (30, 70)));

        let res = ProofVerifierEngine::verify_source(source_code, Some("rust"), &claim_overlap);
        assert!(!res.is_proven());
        assert!(res.verdict.is_refuted());
        assert_eq!(res.certificate, None);
        assert_eq!(res.verdict.failed_assertion_index(), Some(0));
        let reason = res.verdict.failure_reason().unwrap();
        assert!(reason.contains("overlaps with"));
    }
}

// ---------------------------------------------------------------------------
// Scenario 3: MCP Tool #30 JSON-RPC 2.0 Bridge Verification
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_mcp_tool_json_rpc_bridge_workflow() {
    // 1. Verify `tools/list` exposes `verify_proof`
    let list_req = r#"{"jsonrpc":"2.0","id":100,"method":"tools/list"}"#;
    let list_resp = handle_json_rpc_message(list_req).expect("Response expected");
    let parsed_list: Value = serde_json::from_str(&list_resp).unwrap();
    let tools = parsed_list["result"]["tools"].as_array().expect("Tools list");
    let verify_tool = tools
        .iter()
        .find(|t| t["name"] == "verify_proof")
        .expect("verify_proof must be registered in MCP tools");
    assert!(verify_tool["description"].as_str().unwrap().contains("dynamic AST & semantic proof verification"));

    // 2. Execute `tools/call` for `verify_proof` with Proven Outcome
    let call_proven_req = r#"{
        "jsonrpc": "2.0",
        "id": 101,
        "method": "tools/call",
        "params": {
            "name": "verify_proof",
            "arguments": {
                "code": "pub async fn dispatch_event(evt: &str) -> bool { let clean = DOMPurify.sanitize(evt); clean.len() > 0 }",
                "language": "rust",
                "claim": {
                    "claim_id": "claim-mcp-e2e-01",
                    "target_file": "src/events.rs",
                    "target_symbol": "dispatch_event",
                    "invariant_name": "SanitizedEventInvariant",
                    "assertions": [
                        {
                            "NodeExists": {
                                "node_kind": "FunctionDeclaration",
                                "identifier": "dispatch_event"
                            }
                        },
                        {
                            "PropertyEquals": {
                                "node_id": "dispatch_event",
                                "property": "visibility",
                                "expected_value": "pub"
                            }
                        },
                        {
                            "PropertyEquals": {
                                "node_id": "dispatch_event",
                                "property": "async",
                                "expected_value": "true"
                            }
                        }
                    ],
                    "created_at_ms": 1725151500000
                }
            }
        }
    }"#;

    let resp_proven = handle_json_rpc_message(call_proven_req).expect("Response expected");
    let parsed_proven: Value = serde_json::from_str(&resp_proven).unwrap();
    assert_eq!(parsed_proven["id"], 101);
    let content_text = parsed_proven["result"]["content"][0]["text"].as_str().unwrap();

    let res_obj: Value = serde_json::from_str(content_text).expect("Valid JSON result payload");
    assert_eq!(res_obj["verdict"], "Proven");
    assert!(res_obj["certificate"].is_object());
    assert!(res_obj["certificate"]["sha256_signature"].as_str().unwrap().starts_with("LOCUS-PROOF-v2:"));
    assert_eq!(res_obj["steps_evaluated"], 3);

    // 3. Execute `tools/call` for `verify_proof` with Refuted Outcome
    let call_refuted_req = r#"{
        "jsonrpc": "2.0",
        "id": 102,
        "method": "tools/call",
        "params": {
            "name": "verify_proof",
            "arguments": {
                "code": "fn internal_non_async() {}",
                "language": "rust",
                "claim_id": "claim-mcp-e2e-02",
                "target_symbol": "internal_non_async",
                "invariant_name": "AsyncCheck",
                "assertions": [
                    {
                        "PropertyEquals": {
                            "node_id": "internal_non_async",
                            "property": "async",
                            "expected_value": "true"
                        }
                    }
                ]
            }
        }
    }"#;

    let resp_refuted = handle_json_rpc_message(call_refuted_req).expect("Response expected");
    let parsed_refuted: Value = serde_json::from_str(&resp_refuted).unwrap();
    assert_eq!(parsed_refuted["id"], 102);
    let refuted_text = parsed_refuted["result"]["content"][0]["text"].as_str().unwrap();

    let res_refuted_obj: Value = serde_json::from_str(refuted_text).expect("Valid JSON result");
    assert!(res_refuted_obj["verdict"]["Refuted"].is_object());
    assert_eq!(res_refuted_obj["certificate"], Value::Null);
}

// ---------------------------------------------------------------------------
// Scenario 4: Transactional Proof Gating Workflow
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_transactional_proof_gating_workflow() {
    let mut tx = WorkspaceTransaction::begin();

    let valid_code = r#"
        pub fn compute_hash(val: &[u8]) -> String {
            let clean = DOMPurify.sanitize(val);
            clean.to_string()
        }
    "#;

    tx.stage_file("src/crypto/hasher.rs", valid_code, Language::Rust)
        .expect("Staging succeeds");

    // Perform proof verification gating before transaction commit
    let mut claim = ProofClaim::new(
        "claim-tx-gate-01",
        "src/crypto/hasher.rs",
        "compute_hash",
        "NonEmptyPublicFunction",
        1725151600000,
    );
    claim.add_assertion(ProofAssertion::node_exists(
        "FunctionDeclaration",
        Some("compute_hash"),
    ));
    claim.add_assertion(ProofAssertion::property_equals(
        "compute_hash",
        "visibility",
        "pub",
    ));

    let proof_result = ProofVerifierEngine::verify_source(valid_code, Some("rust"), &claim);
    assert!(proof_result.is_proven(), "Gating proof must pass");

    // Transaction proceeds to validate and dry-run commit
    let (passed, violations) = tx.dry_run_verify();
    assert!(passed, "Clean transaction dry-run must pass");
    assert!(violations.is_empty(), "Violations must be empty");
}

// ---------------------------------------------------------------------------
// Scenario 5: Stress & Throughput Benchmark (1,000 Sequential Claims)
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_stress_and_throughput_1000_sequential_claims() {
    let base_sources = [
        "pub fn calc_a(x: i32) -> i32 { let y = DOMPurify.sanitize(x); y + 1 }",
        "pub async fn calc_b(msg: &str) -> usize { let s = clean_path(msg); s.len() }",
        "pub fn calc_c(arr: &[u8]) -> bool { let _ = arr; true }",
        "pub fn calc_d(flag: bool) -> bool { !flag }",
    ];

    let symbols = ["calc_a", "calc_b", "calc_c", "calc_d"];

    let total_iterations = 1000;
    let mut claims = Vec::with_capacity(total_iterations);

    for i in 0..total_iterations {
        let sym = symbols[i % symbols.len()];
        let mut claim = ProofClaim::new(
            format!("stress-claim-{}", i),
            format!("src/module_{}.rs", i % 10),
            sym,
            "ThroughputStressInvariant",
            1725152000000 + (i as u64),
        );
        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some(sym),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            sym,
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::range_intersection_empty(
            (0, 10),
            (20, 50),
        ));
        claims.push(claim);
    }

    // Warmup
    for i in 0..20 {
        let src = base_sources[i % base_sources.len()];
        let _ = ProofVerifierEngine::verify_source(src, Some("rust"), &claims[i]);
    }

    let start = Instant::now();
    let mut proven_count = 0;

    for i in 0..total_iterations {
        let src = base_sources[i % base_sources.len()];
        let res = ProofVerifierEngine::verify_source(src, Some("rust"), &claims[i]);
        if res.is_proven() {
            proven_count += 1;
        }
    }

    let total_elapsed = start.elapsed();
    let avg_latency_ms = (total_elapsed.as_secs_f64() * 1000.0) / (total_iterations as f64);
    let throughput_ops_sec = (total_iterations as f64) / total_elapsed.as_secs_f64();

    println!(
        "\n🚀 ProofVerifierEngine 1,000-Claim Stress Benchmark Results:\n   - Total Elapsed: {:.2?}\n   - Total Claims Verified: {} / {}\n   - Average Latency: {:.4} ms ({:.2} µs / proof)\n   - Sustained Throughput: {:.0} verifications / sec",
        total_elapsed,
        proven_count,
        total_iterations,
        avg_latency_ms,
        avg_latency_ms * 1000.0,
        throughput_ops_sec
    );

    assert_eq!(
        proven_count, total_iterations,
        "All 1,000 stress verification cycles must succeed"
    );

    let budget_ms = if cfg!(debug_assertions) { 0.50 } else { 0.15 };
    assert!(
        avg_latency_ms < budget_ms,
        "Average proof verification latency ({:.4} ms) must remain below the sovereign budget ({:.4} ms)",
        avg_latency_ms,
        budget_ms
    );
}
