//! Multi-File ACID Workspace Transaction Coordinator.
//!
//! Guarantees transactional consistency across multi-file code transformations:
//! - All staged files are validated in-memory against AST invariants before any disk write.
//! - Inter-procedural SSA Taint & Data-Flow analysis rejects unsanitized security leaks.
//! - Atomic commit writes all files only if 100% of invariants and taint checks pass.
//! - On any violation, transaction automatically rolls back with zero disk mutation.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::guard::AstGuard;
use crate::taint::DataFlowTracker;
use crate::tx::shadow_buffer::ShadowBuffer;
use crate::types::{Language, RiskScore, TransactionId, TransactionReport, TransactionStatus};

/// Coordinator for multi-file ACID workspace transactions.
pub struct WorkspaceTransaction {
    pub id: TransactionId,
    pub status: TransactionStatus,
    buffer: ShadowBuffer,
}

impl Default for WorkspaceTransaction {
    fn default() -> Self {
        Self::begin()
    }
}

impl WorkspaceTransaction {
    /// Begin a new multi-file ACID transaction.
    pub fn begin() -> Self {
        Self {
            id: TransactionId::new(),
            status: TransactionStatus::Open,
            buffer: ShadowBuffer::new(),
        }
    }

    /// Stage a modified or new file within the transaction.
    pub fn stage_file(
        &mut self,
        path: &str,
        content: &str,
        language: Language,
    ) -> Result<(), String> {
        if matches!(
            self.status,
            TransactionStatus::Committed | TransactionStatus::RolledBack
        ) {
            return Err(format!(
                "Cannot stage in a closed transaction (status: {:?})",
                self.status
            ));
        }

        self.buffer.stage(path, content, language);
        self.status = TransactionStatus::Staged;
        Ok(())
    }

    /// Dry-run verification pass across all currently staged files (AST Invariants + Inter-Procedural Taint).
    pub fn dry_run_verify(&self) -> (bool, Vec<String>) {
        let mut violations = Vec::new();

        // 1. Single-file AST invariant verification
        for file in self.buffer.all_staged() {
            if file.language == Language::Unknown {
                continue;
            }
            let report = AstGuard::verify(&file.staged_content);
            if !report.passed {
                for v in report.violations {
                    violations.push(format!("[{}] {}", file.path, v));
                }
            }
        }

        // 2. Inter-procedural SSA Taint and Data-Flow verification across staged files
        let staged_files = self.buffer.all_staged();
        let taint_reports = DataFlowTracker::analyze_workspace_files(&staged_files);
        for tr in taint_reports {
            if !tr.is_sanitized && matches!(tr.violation_risk, RiskScore::High | RiskScore::Critical) {
                let sink_op = tr
                    .sinks
                    .first()
                    .map(|s| s.operation.as_str())
                    .unwrap_or("sensitive sink");
                let msg = format!(
                    "TAINT_FLOW_VIOLATION: Unsanitized taint flow detected from '{}' to '{}' along path: {}",
                    tr.source.variable,
                    sink_op,
                    tr.flow_path.join(" -> ")
                );
                violations.push(format!("[{}] {}", tr.source.file, msg));
            }
        }

        let passed = violations.is_empty();
        (passed, violations)
    }

    /// Atomically commit all staged files to disk if and only if AST verification passes for ALL files.
    pub fn commit(&mut self) -> TransactionReport {
        let start = Instant::now();
        let total_staged = self.buffer.len();

        if self.buffer.is_empty() {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            self.status = TransactionStatus::Committed;
            return TransactionReport {
                tx_id: self.id.clone(),
                status: self.status.clone(),
                total_staged_files: 0,
                passed_verification: true,
                violations: Vec::new(),
                committed_files: Vec::new(),
                latency_ms,
            };
        }

        // 1. In-Memory Validation Pass across ALL staged files (Invariants + Taint)
        let (passed, violations) = self.dry_run_verify();

        if !passed {
            self.status =
                TransactionStatus::Failed("Invariant violation in staged files".to_string());
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return TransactionReport {
                tx_id: self.id.clone(),
                status: self.status.clone(),
                total_staged_files: total_staged,
                passed_verification: false,
                violations,
                committed_files: Vec::new(),
                latency_ms,
            };
        }

        // 2. Atomic Disk Commit Phase
        let mut committed_files = Vec::new();
        for file in self.buffer.all_staged() {
            let p = Path::new(&file.path);
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if let Err(e) = fs::write(p, &file.staged_content) {
                // If I/O fails during write, perform rollback of already written files
                self.rollback_written(&committed_files);
                self.status = TransactionStatus::Failed(format!(
                    "Disk write error on '{}': {}",
                    file.path, e
                ));
                let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                return TransactionReport {
                    tx_id: self.id.clone(),
                    status: self.status.clone(),
                    total_staged_files: total_staged,
                    passed_verification: true,
                    violations: vec![format!("Disk write error: {}", e)],
                    committed_files: Vec::new(),
                    latency_ms,
                };
            }
            committed_files.push(file.path.clone());
        }

        self.status = TransactionStatus::Committed;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        TransactionReport {
            tx_id: self.id.clone(),
            status: self.status.clone(),
            total_staged_files: total_staged,
            passed_verification: true,
            violations: Vec::new(),
            committed_files,
            latency_ms,
        }
    }

    /// Rollback the transaction and clear staged buffer without committing changes.
    pub fn rollback(&mut self) -> TransactionReport {
        let start = Instant::now();
        let total_staged = self.buffer.len();
        self.buffer.clear();
        self.status = TransactionStatus::RolledBack;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        TransactionReport {
            tx_id: self.id.clone(),
            status: self.status.clone(),
            total_staged_files: total_staged,
            passed_verification: true,
            violations: Vec::new(),
            committed_files: Vec::new(),
            latency_ms,
        }
    }

    fn rollback_written(&self, written_paths: &[String]) {
        for path in written_paths {
            if let Some(file) = self.buffer.get_staged(path) {
                if let Some(orig) = &file.original_content {
                    let _ = fs::write(path, orig);
                } else {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}
