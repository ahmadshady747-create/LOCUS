//! Production Release Audit Gate & CI/CD Verification Harness.
//!
//! Provides zero-regression quality gate evaluation, deterministic verdicts,
//! and exit-code enforcement in 100% pure safe Rust.

#![forbid(unsafe_code)]

pub mod audit_gate;
pub mod types;

pub use audit_gate::ReleaseAuditGate;
pub use types::{AuditGateConfig, AuditVerdict, ReleaseError};

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sarif::generator::SarifReportGenerator;
    use crate::types::ViolationKind;

    #[test]
    fn test_audit_gate_approves_clean_run() {
        let gate = ReleaseAuditGate::new(AuditGateConfig::zero_tolerance());
        let generator = SarifReportGenerator::new();
        let log = generator.build_log();

        let verdict = gate.evaluate_sarif(&log, 0.05);
        assert!(verdict.is_approved());
        assert_eq!(verdict.exit_code(), 0);

        if let AuditVerdict::Approved { error_count, warning_count, .. } = verdict {
            assert_eq!(error_count, 0);
            assert_eq!(warning_count, 0);
        } else {
            panic!("Expected Approved verdict");
        }
    }

    #[test]
    fn test_audit_gate_rejects_critical_error() {
        let gate = ReleaseAuditGate::new(AuditGateConfig::zero_tolerance());
        let mut generator = SarifReportGenerator::new();

        generator.add_violation(
            "src/crypto/auth.rs",
            ViolationKind::ConstantTimeCrypto,
            "Variable-time password comparison detected",
            10,
            4,
            10,
            20,
            Some("if password == expected".to_string()),
        );

        let log = generator.build_log();
        let verdict = gate.evaluate_sarif(&log, 0.08);

        assert!(!verdict.is_approved());
        assert_eq!(verdict.exit_code(), 1);

        if let AuditVerdict::Rejected { blocker_count, violations, .. } = verdict {
            assert_eq!(blocker_count, 1);
            assert_eq!(violations[0].rule_id, "LOCUS023");
        } else {
            panic!("Expected Rejected verdict");
        }
    }

    #[test]
    fn test_audit_gate_permissive_warning_mode() {
        let gate = ReleaseAuditGate::new(AuditGateConfig::permissive_warnings(2));
        let mut generator = SarifReportGenerator::new();

        // Add 1 warning
        generator.add_violation(
            "src/math/calc.rs",
            ViolationKind::UnsafeUnwrap,
            "Unsafe unwrap on Option",
            15,
            5,
            15,
            15,
            None,
        );

        let log1 = generator.build_log();
        let verdict1 = gate.evaluate_sarif(&log1, 0.05);
        assert!(verdict1.is_approved());

        // Add 2 more warnings (total 3, exceeds limit of 2)
        generator.add_violation("src/a.rs", ViolationKind::UnsafeUnwrap, "unwrap", 1, 1, 1, 5, None);
        generator.add_violation("src/b.rs", ViolationKind::UnsafeUnwrap, "unwrap", 2, 1, 2, 5, None);

        let log2 = generator.build_log();
        let verdict2 = gate.evaluate_sarif(&log2, 0.05);
        assert!(!verdict2.is_approved());
    }

    #[test]
    fn test_audit_gate_latency_budget_rejection() {
        let config = AuditGateConfig::zero_tolerance().with_max_latency(1.0);
        let gate = ReleaseAuditGate::new(config);
        let generator = SarifReportGenerator::new();
        let log = generator.build_log();

        // 1.5ms exceeds 1.0ms limit
        let verdict = gate.evaluate_sarif(&log, 1.5);
        assert!(!verdict.is_approved());
        assert_eq!(verdict.exit_code(), 1);
    }
}
