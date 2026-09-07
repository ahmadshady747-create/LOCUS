//! Zero-Regression Release Audit Gate & CI/CD Quality Verification Engine.
//!
//! Enforces zero-tolerance invariant safety gates, evaluates SARIF logs,
//! and issues deterministic verdicts with process exit codes in 100% pure safe Rust.

#![forbid(unsafe_code)]

use crate::release::types::{AuditGateConfig, AuditVerdict, ReleaseError};
use crate::sarif::generator::SarifReportGenerator;
use crate::sarif::types::{SarifLevel, SarifLog, SarifResult};
use crate::types::VerificationReport;

/// Autonomous verification quality gate for CI/CD and production releases.
#[derive(Debug, Clone)]
pub struct ReleaseAuditGate {
    config: AuditGateConfig,
}

impl Default for ReleaseAuditGate {
    fn default() -> Self {
        Self::new(AuditGateConfig::default())
    }
}

impl ReleaseAuditGate {
    pub fn new(config: AuditGateConfig) -> Self {
        Self { config }
    }

    /// Evaluates a `SarifLog` against the configured audit gate rules.
    pub fn evaluate_sarif(&self, log: &SarifLog, execution_latency_ms: f64) -> AuditVerdict {
        let mut total_rules_checked = 0;
        let mut total_results = 0;
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut blocker_violations: Vec<SarifResult> = Vec::new();

        for run in &log.runs {
            total_rules_checked += run.tool.driver.rules.len();
            total_results += run.results.len();

            for res in &run.results {
                match res.level {
                    SarifLevel::Error => {
                        error_count += 1;
                        blocker_violations.push(res.clone());
                    }
                    SarifLevel::Warning => {
                        warning_count += 1;
                        if self.config.strict_mode {
                            blocker_violations.push(res.clone());
                        }
                    }
                    SarifLevel::Note | SarifLevel::None => {}
                }
            }
        }

        // 1. Check Error Threshold
        if error_count > self.config.max_errors {
            return AuditVerdict::Rejected {
                blocker_count: blocker_violations.len(),
                total_violations: total_results,
                violations: blocker_violations,
                reason: format!(
                    "Critical invariant errors ({}) exceeded allowable threshold ({})",
                    error_count, self.config.max_errors
                ),
                execution_latency_ms,
            };
        }

        // 2. Check Warning Threshold
        if warning_count > self.config.max_warnings {
            return AuditVerdict::Rejected {
                blocker_count: blocker_violations.len(),
                total_violations: total_results,
                violations: blocker_violations,
                reason: format!(
                    "Warnings ({}) exceeded allowable threshold ({})",
                    warning_count, self.config.max_warnings
                ),
                execution_latency_ms,
            };
        }

        // 3. Check Latency Budget
        if let Some(max_lat) = self.config.max_latency_ms {
            if execution_latency_ms > max_lat {
                return AuditVerdict::Rejected {
                    blocker_count: 0,
                    total_violations: total_results,
                    violations: Vec::new(),
                    reason: format!(
                        "Execution latency ({:.2}ms) exceeded performance budget ({:.2}ms)",
                        execution_latency_ms, max_lat
                    ),
                    execution_latency_ms,
                };
            }
        }

        AuditVerdict::Approved {
            total_rules_checked,
            total_results_evaluated: total_results,
            error_count,
            warning_count,
            execution_latency_ms,
        }
    }

    /// Evaluates an in-memory `VerificationReport` directly by generating an ephemeral SARIF log.
    pub fn evaluate_verification_report(
        &self,
        report: &VerificationReport,
        file_uri: &str,
    ) -> AuditVerdict {
        let generator = SarifReportGenerator::from_verification_report(file_uri, report, "");
        let log = generator.build_log();
        self.evaluate_sarif(&log, report.latency_ms)
    }

    /// Parses and evaluates raw SARIF JSON text.
    pub fn evaluate_raw_sarif_json(
        &self,
        json_str: &str,
        latency_ms: f64,
    ) -> Result<AuditVerdict, ReleaseError> {
        let log: SarifLog = serde_json::from_str(json_str)
            .map_err(|e| ReleaseError::SarifParseError(e.to_string()))?;
        Ok(self.evaluate_sarif(&log, latency_ms))
    }
}
