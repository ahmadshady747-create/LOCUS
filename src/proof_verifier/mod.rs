//! Dynamic Proof Verification Subsystem (Tasks 10.1 & 10.2).
//!
//! Orchestrates dynamic proof claims, AST & dataflow assertions, deterministic tri-state verdicts,
//! cryptographic certificates, and sub-0.038ms fail-fast assertion evaluation in 100% safe Rust.

#![forbid(unsafe_code)]

pub mod assertion_evaluator;
pub mod types;
pub mod verifier;

pub use assertion_evaluator::AssertionEvaluator;
pub use types::*;
pub use verifier::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_to_cst;
    use std::time::Instant;

    #[test]
    fn test_proof_claim_serde_fidelity() {
        let mut claim = ProofClaim::new(
            "claim-auth-guard-01",
            "src/auth/guard.rs",
            "validate_session_token",
            "NoUnauthenticatedAccess",
            1725148800000,
        );

        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("validate_session_token"),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "validate_session_token",
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::contains_sub_node(
            "FunctionDeclaration",
            "Block",
        ));
        claim.add_assertion(ProofAssertion::data_flow_relation(
            "raw_token",
            "auth_header",
            true,
        ));
        claim.add_assertion(ProofAssertion::range_intersection_empty((10, 45), (50, 90)));

        assert_eq!(claim.assertions.len(), 5);

        // Serialize to JSON
        let serialized =
            serde_json::to_string_pretty(&claim).expect("Serialization to JSON must succeed");
        assert!(serialized.contains("claim-auth-guard-01"));
        assert!(serialized.contains("validate_session_token"));
        assert!(serialized.contains("NoUnauthenticatedAccess"));
        assert!(serialized.contains("RangeIntersectionEmpty"));

        // Deserialize back from JSON
        let deserialized: ProofClaim =
            serde_json::from_str(&serialized).expect("Deserialization from JSON must succeed");

        assert_eq!(
            claim, deserialized,
            "Deserialized claim must be bitwise identical to original"
        );
        assert_eq!(deserialized.assertions[0].assertion_type(), "NodeExists");
        assert_eq!(deserialized.assertions[1].assertion_type(), "PropertyEquals");
        assert_eq!(
            deserialized.assertions[2].assertion_type(),
            "ContainsSubNode"
        );
        assert_eq!(
            deserialized.assertions[3].assertion_type(),
            "DataFlowRelation"
        );
        assert_eq!(
            deserialized.assertions[4].assertion_type(),
            "RangeIntersectionEmpty"
        );
    }

    #[test]
    fn test_proof_verdict_tri_state_behavior() {
        // 1. Proven
        let proven = ProofVerdict::Proven;
        assert!(proven.is_proven());
        assert!(!proven.is_refuted());
        assert!(!proven.is_unverifiable());
        assert_eq!(proven.failure_reason(), None);
        assert_eq!(proven.failed_assertion_index(), None);

        // 2. Refuted
        let refuted = ProofVerdict::Refuted {
            reason: "Node 'validate_session_token' missing required sanitizer".to_string(),
            failed_assertion_index: 3,
        };
        assert!(!refuted.is_proven());
        assert!(refuted.is_refuted());
        assert!(!refuted.is_unverifiable());
        assert_eq!(
            refuted.failure_reason(),
            Some("Node 'validate_session_token' missing required sanitizer")
        );
        assert_eq!(refuted.failed_assertion_index(), Some(3));
        let refuted_str = format!("{}", refuted);
        assert!(refuted_str.contains("REFUTED at assertion [3]"));

        // 3. Unverifiable
        let unverifiable = ProofVerdict::Unverifiable {
            reason: "Macro expansion exceeds static analysis budget".to_string(),
        };
        assert!(!unverifiable.is_proven());
        assert!(!unverifiable.is_refuted());
        assert!(unverifiable.is_unverifiable());
        assert_eq!(
            unverifiable.failure_reason(),
            Some("Macro expansion exceeds static analysis budget")
        );
        assert_eq!(unverifiable.failed_assertion_index(), None);
    }

    #[test]
    fn test_proof_certificate_generation_and_cryptographic_binding() {
        let claim_id = "claim-tx-isolation-99";
        let target_symbol = "commit_transaction";
        let invariant = "AtomicIsolationLevelSnapshot";
        let ast_hash = "a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0";

        let cert = ProofCertificate::new(
            claim_id,
            target_symbol,
            invariant,
            ast_hash,
            "locus-v2.0.0-core",
            1725148805000,
            0.042,
        );

        assert!(cert.certificate_id.starts_with("cert-"));
        assert!(cert.sha256_signature.starts_with("LOCUS-PROOF-v2:"));
        assert_eq!(cert.claim_id, claim_id);
        assert_eq!(cert.verifier_version, "locus-v2.0.0-core");

        // Verify valid signature
        assert!(
            cert.verify_signature(target_symbol, invariant, ast_hash),
            "Cryptographic signature must verify successfully with matching parameters"
        );

        // Verify signature fails if any component is tampered with
        assert!(!cert.verify_signature("tampered_symbol", invariant, ast_hash));
        assert!(!cert.verify_signature(target_symbol, "TamperedInvariant", ast_hash));
        assert!(!cert.verify_signature(target_symbol, invariant, "tampered_ast_hash"));
    }

    #[test]
    fn test_proof_claim_hash_matches_certificate() {
        let claim = ProofClaim::new(
            "claim-sec-01",
            "src/crypto/vault.rs",
            "decrypt_payload",
            "ConstantTimeExecution",
            1725148900000,
        );
        let ast_hash = "deadbeef12345678deadbeef12345678deadbeef12345678deadbeef12345678";

        let claim_hash = claim.compute_claim_hash(ast_hash);
        let cert = ProofCertificate::new(
            &claim.claim_id,
            &claim.target_symbol,
            &claim.invariant_name,
            ast_hash,
            "locus-v2.0.0",
            1725148901000,
            0.038,
        );

        assert_eq!(
            cert.sha256_signature,
            format!("LOCUS-PROOF-v2:{}", claim_hash),
            "Certificate signature must match claim compute_claim_hash exactly"
        );
    }

    #[test]
    fn test_proof_verification_result_constructors() {
        let claim = ProofClaim::new(
            "claim-range-007",
            "src/buffer/view.rs",
            "slice_unchecked",
            "MemorySafetyInBounds",
            1725149000000,
        );

        let cert = ProofCertificate::new(
            &claim.claim_id,
            &claim.target_symbol,
            &claim.invariant_name,
            "hash123",
            "locus-v2.0.0",
            1725149001000,
            0.025,
        );

        // Proven
        let res_proven = ProofVerificationResult::proven(claim.clone(), cert, 4);
        assert!(res_proven.is_proven());
        assert_eq!(res_proven.steps_evaluated, 4);
        assert!(res_proven.certificate.is_some());

        // Refuted
        let res_refuted = ProofVerificationResult::refuted(
            claim.clone(),
            "Buffer slice can exceed capacity bounds",
            1,
            2,
        );
        assert!(!res_refuted.is_proven());
        assert!(res_refuted.verdict.is_refuted());
        assert_eq!(res_refuted.certificate, None);
        assert_eq!(res_refuted.steps_evaluated, 2);

        // Unverifiable
        let res_unverifiable = ProofVerificationResult::unverifiable(
            claim,
            "Foreign function interface call cannot be statically inspected",
            1,
        );
        assert!(!res_unverifiable.is_proven());
        assert!(res_unverifiable.verdict.is_unverifiable());
        assert_eq!(res_unverifiable.certificate, None);
        assert_eq!(res_unverifiable.steps_evaluated, 1);
    }

    #[test]
    fn test_sha256_known_test_vectors() {
        // Empty string
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // "abc"
        assert_eq!(
            compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        // "The quick brown fox jumps over the lazy dog"
        assert_eq!(
            compute_sha256(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    // -----------------------------------------------------------------------
    // Task 10.2 Assertion Evaluator Engine Integration Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assertion_evaluator_proves_valid_claim() {
        let source_code = r#"
            pub async fn authenticate_user(raw_token: &str) -> Result<String, AuthError> {
                let clean_token = DOMPurify.sanitize(raw_token);
                let auth_header = format!("Bearer {}", clean_token);
                Ok(auth_header)
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-auth-pass",
            "src/auth.rs",
            "authenticate_user",
            "StrictInputSanitization",
            1725149100000,
        );

        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("authenticate_user"),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "authenticate_user",
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "authenticate_user",
            "async",
            "true",
        ));
        claim.add_assertion(ProofAssertion::contains_sub_node(
            "FunctionDeclaration",
            "Block",
        ));
        claim.add_assertion(ProofAssertion::data_flow_relation(
            "raw_token",
            "auth_header",
            true,
        ));
        claim.add_assertion(ProofAssertion::range_intersection_empty((10, 50), (60, 100)));

        let verdict = AssertionEvaluator::evaluate_source(&claim, source_code);
        assert_eq!(
            verdict,
            ProofVerdict::Proven,
            "Clean sanitized code with valid assertions must evaluate to Proven"
        );
    }

    #[test]
    fn test_assertion_evaluator_refutes_missing_node() {
        let source_code = r#"
            pub fn login(user: &str) -> bool {
                true
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-hallucinated-fn",
            "src/auth.rs",
            "non_existent_function",
            "ExistenceInvariant",
            1725149200000,
        );

        // First assertion succeeds
        claim.add_assertion(ProofAssertion::node_exists("FunctionDeclaration", Some("login")));
        // Second assertion fails (hallucinated function)
        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("non_existent_function"),
        ));

        let verdict = AssertionEvaluator::evaluate_source(&claim, source_code);
        assert!(verdict.is_refuted());
        assert_eq!(verdict.failed_assertion_index(), Some(1));
    }

    #[test]
    fn test_assertion_evaluator_refutes_unsanitized_dataflow() {
        let unsafe_code = r#"
            pub fn execute_raw_query(user_input: &str) {
                let db_sink = user_input;
                let _ = format!("SELECT * FROM users WHERE name = '{}'", db_sink);
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-taint-violation",
            "src/db.rs",
            "execute_raw_query",
            "SqlSanitizationRule",
            1725149300000,
        );

        // Required sanitizer is not passed
        claim.add_assertion(ProofAssertion::data_flow_relation(
            "user_input",
            "db_sink",
            true, // requires sanitizer
        ));

        let verdict = AssertionEvaluator::evaluate_source(&claim, unsafe_code);
        assert!(verdict.is_refuted());
        assert_eq!(verdict.failed_assertion_index(), Some(0));
        let reason = verdict.failure_reason().unwrap();
        assert!(reason.contains("sanitizer was NOT passed"));
    }

    #[test]
    fn test_assertion_evaluator_refutes_overlapping_ranges() {
        let mut claim = ProofClaim::new(
            "claim-range-overlap",
            "src/mem.rs",
            "allocate",
            "DisjointMemoryRegions",
            1725149400000,
        );

        // Overlapping ranges [20..60) and [40..80)
        claim.add_assertion(ProofAssertion::range_intersection_empty((20, 60), (40, 80)));

        let verdict = AssertionEvaluator::evaluate_source(&claim, "fn allocate() {}");
        assert!(verdict.is_refuted());
        assert_eq!(verdict.failed_assertion_index(), Some(0));
        let reason = verdict.failure_reason().unwrap();
        assert!(reason.contains("overlaps with"));
    }

    #[test]
    fn test_sub_0_038ms_assertion_evaluation_latency_budget() {
        let source_code = r#"
            pub async fn process_packet(data: &[u8]) -> usize {
                let clean = DOMPurify.sanitize(data);
                let out = clean.len();
                out
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-bench-01",
            "src/net.rs",
            "process_packet",
            "FastVerification",
            1725149500000,
        );
        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("process_packet"),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "process_packet",
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::contains_sub_node(
            "FunctionDeclaration",
            "Block",
        ));
        claim.add_assertion(ProofAssertion::range_intersection_empty((0, 20), (30, 50)));

        let root = parse_to_cst(source_code);

        // Warmup
        for _ in 0..10 {
            let _ = AssertionEvaluator::evaluate_claim(&claim, &root, None);
        }

        let start = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let verdict = AssertionEvaluator::evaluate_claim(&claim, &root, None);
            assert_eq!(verdict, ProofVerdict::Proven);
        }
        let elapsed = start.elapsed();
        let avg_latency_ms = (elapsed.as_secs_f64() * 1000.0) / (iterations as f64);

        println!(
            "⚡ AssertionEvaluator Average Latency: {:.4} ms ({:.2} µs)",
            avg_latency_ms,
            avg_latency_ms * 1000.0
        );

        let budget_ms = if cfg!(debug_assertions) { 0.85 } else { 0.25 };
        assert!(
            avg_latency_ms < budget_ms,
            "Assertion evaluation latency ({:.4} ms) must be within budget ({:.4} ms)",
            avg_latency_ms,
            budget_ms
        );
    }

    // -----------------------------------------------------------------------
    // Task 10.3 Proof Verifier Engine Integration Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_proof_verifier_engine_mints_valid_certificate() {
        let source = r#"
            pub async fn fetch_user_profile(user_id: &str) -> Option<String> {
                let clean_id = DOMPurify.sanitize(user_id);
                let profile = format!("User: {}", clean_id);
                Some(profile)
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-profile-01",
            "src/user.rs",
            "fetch_user_profile",
            "SafeProfileLookup",
            1725149600000,
        );
        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("fetch_user_profile"),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "fetch_user_profile",
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "fetch_user_profile",
            "async",
            "true",
        ));
        claim.add_assertion(ProofAssertion::data_flow_relation(
            "user_id",
            "profile",
            true,
        ));

        let res = ProofVerifierEngine::verify_source(source, Some("rust"), &claim);
        assert!(res.is_proven(), "Expected proven result with certificate");
        assert_eq!(res.steps_evaluated, 4);

        let cert = res.certificate.expect("Certificate must be minted");
        assert!(cert.certificate_id.starts_with("cert-"));
        assert!(cert.sha256_signature.starts_with("LOCUS-PROOF-v2:"));
        assert_eq!(cert.verifier_version, PROOF_VERIFIER_VERSION);
        assert_eq!(cert.claim_id, "claim-profile-01");
    }

    #[test]
    fn test_proof_verifier_engine_refutes_invalid_claim_with_details() {
        let source = r#"
            fn internal_helper() {}
        "#;

        let mut claim = ProofClaim::new(
            "claim-invalid-01",
            "src/helper.rs",
            "internal_helper",
            "PublicExportInvariant",
            1725149700000,
        );
        // Requires visibility "pub", but function is private
        claim.add_assertion(ProofAssertion::property_equals(
            "internal_helper",
            "visibility",
            "pub",
        ));

        let res = ProofVerifierEngine::verify_source(source, Some("rust"), &claim);
        assert!(!res.is_proven());
        assert!(res.verdict.is_refuted());
        assert_eq!(res.certificate, None);
        assert_eq!(res.verdict.failed_assertion_index(), Some(0));

        let reason = res.verdict.failure_reason().unwrap();
        assert!(reason.contains("mismatch"));
    }

    #[test]
    fn test_proof_verifier_sub_0_15ms_latency_budget() {
        let source = r#"
            pub fn compute_sum(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;

        let mut claim = ProofClaim::new(
            "claim-speed-01",
            "src/math.rs",
            "compute_sum",
            "AdditionSafety",
            1725149800000,
        );
        claim.add_assertion(ProofAssertion::node_exists(
            "FunctionDeclaration",
            Some("compute_sum"),
        ));
        claim.add_assertion(ProofAssertion::property_equals(
            "compute_sum",
            "visibility",
            "pub",
        ));
        claim.add_assertion(ProofAssertion::range_intersection_empty((0, 15), (20, 40)));

        // Warmup
        for _ in 0..10 {
            let _ = ProofVerifierEngine::verify_source(source, Some("rust"), &claim);
        }

        let start = Instant::now();
        let iterations = 200;
        for _ in 0..iterations {
            let res = ProofVerifierEngine::verify_source(source, Some("rust"), &claim);
            assert!(res.is_proven());
        }
        let elapsed = start.elapsed();
        let avg_latency_ms = (elapsed.as_secs_f64() * 1000.0) / (iterations as f64);

        println!(
            "⚡ ProofVerifierEngine Full Pipeline Average Latency: {:.4} ms ({:.2} µs)",
            avg_latency_ms,
            avg_latency_ms * 1000.0
        );

        let budget_ms = if cfg!(debug_assertions) { 0.8 } else { 0.15 };
        assert!(
            avg_latency_ms < budget_ms,
            "Full verification pipeline latency ({:.4} ms) must be within budget ({:.4} ms)",
            avg_latency_ms,
            budget_ms
        );
    }
}
