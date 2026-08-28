//! Phase 1.6 Verification: Inter-Procedural SSA Taint Engine v2 & Sanitizer Proof Chains.

use locus_engine::taint::DataFlowTracker;
use locus_engine::tx::WorkspaceTransaction;
use locus_engine::types::{
    Language, RiskScore, SanitizerRule, TaintAuditCertificate, TxStagedFile,
};

#[test]
fn test_intra_procedural_taint_detection() {
    let source = r#"
    export async function handleOperation(req, res) {
        const client_target = req.headers["x-target"];
        const data = await db.execute(client_target);
        return data;
    }
    "#;

    let reports = DataFlowTracker::analyze_source("src/operation.ts", "handleOperation", source);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert_eq!(r.source.variable, "client_target");
    assert!(!r.is_sanitized);
    assert_eq!(r.violation_risk, RiskScore::High);
    assert!(r.certificate.is_none());
}

#[test]
fn test_inter_procedural_argument_propagation() {
    let file1 = TxStagedFile {
        path: "src/controller.ts".to_string(),
        original_content: None,
        staged_content: r#"
        import { runDatabaseQuery } from './service';
        export async function handleRequest(req, res) {
            const client_target = req.headers["x-query"];
            await runDatabaseQuery(client_target);
        }
        "#
        .to_string(),
        language: Language::TypeScript,
    };

    let file2 = TxStagedFile {
        path: "src/service.ts".to_string(),
        original_content: None,
        staged_content: r#"
        export async function runDatabaseQuery(rawQuery: string) {
            return await db.execute(rawQuery);
        }
        "#
        .to_string(),
        language: Language::TypeScript,
    };

    let reports = DataFlowTracker::analyze_owned_files(&[file1, file2]);
    assert!(!reports.is_empty());
    let r = &reports[0];
    assert_eq!(r.source.variable, "client_target");
    assert!(!r.is_sanitized);
    assert_eq!(r.violation_risk, RiskScore::High);
    assert!(r.flow_path.len() >= 2);
}

#[test]
fn test_sanitizer_proof_chain_dompurify() {
    let source = r#"
    export function renderUserBio(req, res) {
        const raw_input = req.headers["x-bio"];
        const cleanBio = DOMPurify.sanitize(raw_input);
        container.innerHTML = cleanBio;
    }
    "#;

    let reports = DataFlowTracker::analyze_source("src/profile.ts", "renderUserBio", source);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert_eq!(r.source.variable, "raw_input");
    assert!(r.is_sanitized);
    assert_eq!(r.violation_risk, RiskScore::Low);

    let cert = r.certificate.as_ref().expect("Expected TaintAuditCertificate");
    assert_eq!(cert.sanitizer_name, "DOMPurify.sanitize");
    assert_eq!(cert.sanitizer_rule, SanitizerRule::HtmlSanitization);
    assert_eq!(cert.source_variable, "raw_input");
    assert_eq!(cert.sha256_fingerprint.len(), 64);
}

#[test]
fn test_sanitizer_proof_chain_sql() {
    let source = r#"
    export async function searchUser(req, res) {
        const query_input = req.headers["x-name"];
        const safeQuery = sanitize_sql(query_input);
        return await db.query(safeQuery);
    }
    "#;

    let reports = DataFlowTracker::analyze_source("src/search.ts", "searchUser", source);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert!(r.is_sanitized);
    assert_eq!(r.violation_risk, RiskScore::Low);

    let cert = r.certificate.as_ref().expect("Expected TaintAuditCertificate");
    assert_eq!(cert.sanitizer_rule, SanitizerRule::SqlParamBinding);
}

#[test]
fn test_sanitizer_proof_chain_url_encoding() {
    let source = r#"
    export async function proxyRequest(req, res) {
        const client_target = req.headers["x-target-url"];
        const encodedUrl = encodeURIComponent(client_target);
        return await fetch(encodedUrl);
    }
    "#;

    let reports = DataFlowTracker::analyze_source("src/proxy.ts", "proxyRequest", source);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert!(r.is_sanitized);

    let cert = r.certificate.as_ref().expect("Expected TaintAuditCertificate");
    assert_eq!(cert.sanitizer_rule, SanitizerRule::UrlEncoding);
}

#[test]
fn test_taint_audit_certificate_sha256_verification() {
    let cert = TaintAuditCertificate::generate(
        "taint_test_1",
        "raw_user_input",
        "DOMPurify.sanitize",
        SanitizerRule::HtmlSanitization,
        "innerHTML = cleanHtml",
        vec![
            "src/app.ts:raw_user_input".to_string(),
            "src/app.ts:cleanHtml".to_string(),
        ],
    );

    assert_eq!(cert.sha256_fingerprint.len(), 64);
    assert!(cert.certificate_id.starts_with("cert_"));
    assert_eq!(cert.sanitizer_rule, SanitizerRule::HtmlSanitization);
}

#[test]
fn test_transaction_blocks_unsanitized_taint_flow() {
    let mut tx = WorkspaceTransaction::begin();
    let unsanitized_code = r#"
    export async function executeCommand(req, res) {
        const client_target = req.headers["x-cmd"];
        await db.execute(client_target);
    }
    "#;

    let _ = tx.stage_file(
        "src/vulnerable.ts",
        unsanitized_code,
        Language::TypeScript,
    );
    let (passed, violations) = tx.dry_run_verify();
    assert!(!passed);
    assert!(violations
        .iter()
        .any(|v| v.contains("TAINT_FLOW_VIOLATION")));

    let rep = tx.commit();
    assert!(!rep.passed_verification);
    assert_eq!(rep.committed_files.len(), 0);
}

#[test]
fn test_transaction_accepts_sanitized_taint_flow() {
    let mut tx = WorkspaceTransaction::begin();
    let sanitized_code = r#"
    export function renderContent(req, res) {
        const raw_input = req.headers["x-text"];
        const cleanText = DOMPurify.sanitize(raw_input);
        container.innerHTML = cleanText;
    }
    "#;

    let _ = tx.stage_file("src/safe.ts", sanitized_code, Language::TypeScript);
    let (passed, violations) = tx.dry_run_verify();
    assert!(passed, "Violations found: {:?}", violations);

    let rep = tx.commit();
    assert!(rep.passed_verification);
    assert_eq!(rep.committed_files.len(), 1);
}
