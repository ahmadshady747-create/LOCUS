//! High-Throughput Deterministic SARIF 2.1.0 Report Generator.
//!
//! Synthesizes OASIS SARIF logs from AST Guard invariant reports, taint flow traces,
//! and security audits with sub-millisecond serialization in 100% safe Rust.

#![forbid(unsafe_code)]

use crate::meta_mcp::verifier::types::SecurityViolation;
use crate::sarif::rules::{get_canonical_rules, lookup_rule, rule_id_for_security_violation, rule_id_for_violation};
use crate::sarif::types::{
    ArtifactContent, ArtifactLocation, CodeFlow, Invocation, Location, Message, PhysicalLocation,
    Region, ReportingDescriptor, SarifError, SarifLevel, SarifLog, SarifResult, SarifRun,
    ThreadFlow, ThreadFlowLocation, ToolComponent, ToolComponentDriver, SARIF_SCHEMA_URI,
    SARIF_VERSION,
};
use crate::types::{VerificationReport, ViolationKind};
use serde_json::json;

/// High-throughput deterministic SARIF report synthesizer.
#[derive(Debug, Clone)]
pub struct SarifReportGenerator {
    tool_name: String,
    tool_version: String,
    information_uri: String,
    rules: Vec<ReportingDescriptor>,
    results: Vec<SarifResult>,
    invocations: Vec<Invocation>,
}

impl Default for SarifReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SarifReportGenerator {
    /// Creates a fresh generator pre-populated with canonical LOCUS Invariant Rules.
    pub fn new() -> Self {
        Self {
            tool_name: "locus-engine".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            information_uri: "https://github.com/locus-engine/locus".to_string(),
            rules: get_canonical_rules(),
            results: Vec::new(),
            invocations: Vec::new(),
        }
    }

    /// Sets custom tool component metadata.
    pub fn with_tool_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.tool_name = name.into();
        self.tool_version = version.into();
        self
    }

    /// Appends a raw SARIF result.
    pub fn add_result(&mut self, result: SarifResult) {
        self.results.push(result);
    }

    /// Appends an invariant violation finding with line and column span.
    pub fn add_violation(
        &mut self,
        file_uri: &str,
        kind: ViolationKind,
        detail: &str,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        snippet: Option<String>,
    ) {
        let rule_id = rule_id_for_violation(&kind);
        let level = match lookup_rule(rule_id)
            .and_then(|r| r.default_configuration)
            .map(|c| c.level)
        {
            Some(l) => l,
            None => SarifLevel::Error,
        };

        let mut region = Region::from_line_col(start_line, start_col, end_line, end_col);
        if let Some(s) = snippet {
            region.snippet = Some(ArtifactContent { text: s });
        }

        let location = Location::from_physical(PhysicalLocation {
            artifact_location: ArtifactLocation::new(file_uri),
            region: Some(region),
            context_region: None,
        });

        let result = SarifResult {
            rule_id: rule_id.to_string(),
            rule_index: self.rules.iter().position(|r| r.id == rule_id),
            level,
            message: Message::new(detail),
            locations: vec![location],
            code_flows: None,
            related_locations: None,
            properties: Some(json!({
                "violationKind": kind.to_string(),
                "engine": "locus-ast-guard",
            })),
        };

        self.results.push(result);
    }

    /// Synthesizes a SARIF report directly from an `AstGuard` `VerificationReport`.
    pub fn from_verification_report(
        file_uri: &str,
        report: &VerificationReport,
        _source_code: &str,
    ) -> Self {
        let mut generator = Self::new();

        if let Some(violation) = &report.violation {
            let detail = report.detail.as_deref().unwrap_or("Invariant violation detected");
            generator.add_violation(
                file_uri,
                violation.clone(),
                detail,
                1,
                1,
                1,
                1,
                None,
            );
        }

        generator
    }

    /// Ingests security violations from the dynamic sandbox verifier.
    pub fn from_security_violations(
        file_uri: &str,
        violations: &[SecurityViolation],
    ) -> Self {
        let mut generator = Self::new();

        for v in violations {
            let rule_id = rule_id_for_security_violation(&v.violation_kind);
            let region = Region {
                start_line: Some(v.line_offset),
                start_column: Some(1),
                end_line: Some(v.line_offset),
                end_column: Some(1),
                char_offset: None,
                char_length: None,
                snippet: Some(ArtifactContent {
                    text: v.target_node.clone(),
                }),
            };

            let location = Location::from_physical(PhysicalLocation {
                artifact_location: ArtifactLocation::new(file_uri),
                region: Some(region),
                context_region: None,
            });

            let result = SarifResult {
                rule_id: rule_id.to_string(),
                rule_index: generator.rules.iter().position(|r| r.id == rule_id),
                level: SarifLevel::Error,
                message: Message::with_markdown(&v.description, &v.remediation_hint),
                locations: vec![location],
                code_flows: None,
                related_locations: None,
                properties: Some(json!({
                    "securityViolationKind": format!("{:?}", v.violation_kind),
                    "targetNode": v.target_node.clone(),
                })),
            };

            generator.results.push(result);
        }

        generator
    }

    /// Appends a multi-step taint or data flow trace to the SARIF report.
    pub fn add_taint_flow(
        &mut self,
        rule_id: &str,
        message: &str,
        steps: &[(String, usize, usize, String)], // (file_uri, line, col, snippet)
    ) {
        let mut thread_flow_locations = Vec::with_capacity(steps.len());

        for (order, (file_uri, line, col, snippet)) in steps.iter().enumerate() {
            let region = Region {
                start_line: Some(*line),
                start_column: Some(*col),
                end_line: Some(*line),
                end_column: Some(col + snippet.len()),
                char_offset: None,
                char_length: None,
                snippet: Some(ArtifactContent {
                    text: snippet.clone(),
                }),
            };

            let loc = Location::from_physical(PhysicalLocation {
                artifact_location: ArtifactLocation::new(file_uri),
                region: Some(region),
                context_region: None,
            });

            thread_flow_locations.push(ThreadFlowLocation {
                location: Some(loc),
                state: None,
                nesting_level: Some(0),
                execution_order: Some(order + 1),
            });
        }

        let primary_loc = if let Some(first) = steps.first() {
            Location::from_physical(PhysicalLocation {
                artifact_location: ArtifactLocation::new(&first.0),
                region: Some(Region::from_line_col(first.1, first.2, first.1, first.2 + first.3.len())),
                context_region: None,
            })
        } else {
            Location::from_physical(PhysicalLocation {
                artifact_location: ArtifactLocation::new("unknown"),
                region: None,
                context_region: None,
            })
        };

        let code_flow = CodeFlow {
            thread_flows: vec![ThreadFlow {
                locations: thread_flow_locations,
            }],
            message: Some(Message::new("Taint propagation path")),
        };

        let result = SarifResult {
            rule_id: rule_id.to_string(),
            rule_index: self.rules.iter().position(|r| r.id == rule_id),
            level: SarifLevel::Error,
            message: Message::new(message),
            locations: vec![primary_loc],
            code_flows: Some(vec![code_flow]),
            related_locations: None,
            properties: Some(json!({
                "taintTraceLength": steps.len(),
            })),
        };

        self.results.push(result);
    }

    /// Records an execution invocation metric into the report.
    pub fn record_invocation(&mut self, success: bool, exit_code: Option<i32>) {
        self.invocations.push(Invocation {
            execution_successful: success,
            exit_code,
            start_time_utc: None,
            end_time_utc: None,
        });
    }

    /// Compiles the in-memory results and tool metadata into a complete `SarifLog`.
    pub fn build_log(&self) -> SarifLog {
        let run = SarifRun {
            tool: ToolComponent {
                driver: ToolComponentDriver {
                    name: self.tool_name.clone(),
                    version: Some(self.tool_version.clone()),
                    information_uri: Some(self.information_uri.clone()),
                    rules: self.rules.clone(),
                },
            },
            results: self.results.clone(),
            invocations: if self.invocations.is_empty() {
                None
            } else {
                Some(self.invocations.clone())
            },
        };

        SarifLog {
            schema: SARIF_SCHEMA_URI.to_string(),
            version: SARIF_VERSION.to_string(),
            runs: vec![run],
        }
    }

    /// Exports the SARIF report to a pretty-formatted JSON string with zero copying.
    pub fn export_to_json(&self) -> Result<String, SarifError> {
        let log = self.as_ref_log();
        serde_json::to_string_pretty(&log).map_err(|e| SarifError::SerializationError(e.to_string()))
    }

    /// Exports the SARIF report to a high-throughput compact JSON string with zero copying.
    pub fn export_to_compact_json(&self) -> Result<String, SarifError> {
        let log = self.as_ref_log();
        serde_json::to_string(&log).map_err(|e| SarifError::SerializationError(e.to_string()))
    }

    /// Exports the SARIF report directly as a `serde_json::Value`.
    pub fn export_to_value(&self) -> Result<serde_json::Value, SarifError> {
        let log = self.as_ref_log();
        serde_json::to_value(&log).map_err(|e| SarifError::SerializationError(e.to_string()))
    }

    fn as_ref_log(&self) -> SarifLogRef<'_> {
        let run = SarifRunRef {
            tool: ToolComponentRef {
                driver: ToolComponentDriverRef {
                    name: &self.tool_name,
                    version: Some(&self.tool_version),
                    information_uri: Some(&self.information_uri),
                    rules: &self.rules,
                },
            },
            results: &self.results,
            invocations: if self.invocations.is_empty() {
                None
            } else {
                Some(&self.invocations)
            },
        };

        SarifLogRef {
            schema: SARIF_SCHEMA_URI,
            version: SARIF_VERSION,
            runs: vec![run],
        }
    }

    /// Returns the count of recorded findings.
    pub fn results_count(&self) -> usize {
        self.results.len()
    }
}

// ---------------------------------------------------------------------------
// Zero-Copy Serialization References
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct ToolComponentDriverRef<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "informationUri")]
    information_uri: Option<&'a str>,
    rules: &'a [ReportingDescriptor],
}

#[derive(serde::Serialize)]
struct ToolComponentRef<'a> {
    driver: ToolComponentDriverRef<'a>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRunRef<'a> {
    tool: ToolComponentRef<'a>,
    results: &'a [SarifResult],
    #[serde(skip_serializing_if = "Option::is_none")]
    invocations: Option<&'a [Invocation]>,
}

#[derive(serde::Serialize)]
struct SarifLogRef<'a> {
    #[serde(rename = "$schema")]
    schema: &'a str,
    version: &'a str,
    runs: Vec<SarifRunRef<'a>>,
}
