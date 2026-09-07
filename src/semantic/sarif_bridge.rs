//! Bridge connecting Semantic Reports to OASIS SARIF v2.1.0 JSON format.
//!
//! Converts SemanticViolation records and mathematical proofs to standardized SARIF findings.

#![forbid(unsafe_code)]

use serde_json::json;

use crate::sarif::generator::SarifReportGenerator;
use crate::sarif::types::{
    ArtifactLocation, Location, Message, PhysicalLocation, Region, SarifLevel, SarifLog,
    SarifResult,
};
use crate::semantic::types::SemanticReport;

/// Converts LOCUS `SemanticReport` structures into canonical OASIS `SarifLog` documents.
pub struct SemanticSarifBridge;

impl SemanticSarifBridge {
    /// Transforms a `SemanticReport` into a standardized OASIS `SarifLog`.
    pub fn to_sarif_log(report: &SemanticReport, file_uri: &str) -> SarifLog {
        let mut generator = SarifReportGenerator::new();

        for proof in &report.proofs {
            let v = &proof.violation;
            let rule_id = format!("LOCUS_{}", v.rule_id);

            let region = Region {
                start_line: Some(v.location.line),
                start_column: Some(v.location.column),
                end_line: Some(v.location.line),
                end_column: Some(v.location.column + v.location.span_len),
                char_offset: Some(v.location.byte_offset),
                char_length: Some(v.location.span_len),
                snippet: None,
            };

            let location = Location::from_physical(PhysicalLocation {
                artifact_location: ArtifactLocation::new(file_uri),
                region: Some(region),
                context_region: None,
            });

            let markdown_body = format!(
                "### Semantic Invariant Violation: {}\n\n{}\n\n**Mathematical Proof:**\n```\n{}\n```\n\n**Remediation:**\n```javascript\n{}\n```",
                v.rule_name, v.message, proof.mathematical_proof, proof.remediation_patch
            );

            let result = SarifResult {
                rule_id: rule_id.clone(),
                rule_index: None,
                level: SarifLevel::Error,
                message: Message::with_markdown(&v.message, &markdown_body),
                locations: vec![location],
                code_flows: None,
                related_locations: None,
                properties: Some(json!({
                    "semanticRule": v.rule_id,
                    "counterexample": proof.counterexample,
                    "suggestedFix": v.suggested_fix,
                })),
            };

            generator.add_result(result);
        }

        generator.build_log()
    }

    /// Exports the translated SARIF log to a formatted JSON string.
    pub fn export_to_json(report: &SemanticReport, file_uri: &str) -> Result<String, crate::sarif::types::SarifError> {
        let log = Self::to_sarif_log(report, file_uri);
        serde_json::to_string_pretty(&log)
            .map_err(|e| crate::sarif::types::SarifError::SerializationError(e.to_string()))
    }
}
