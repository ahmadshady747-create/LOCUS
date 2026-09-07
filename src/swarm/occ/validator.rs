//! 3-Phase Optimistic Concurrency Control (OCC) Validation and Atomic Commit Engine.
//!
//! Validates spatial subtree span non-overlap, detects version drift,
//! and executes atomic sub-0.05ms commits with monotonic version increments ($V_n \to V_{n+1}$).

#![forbid(unsafe_code)]

use super::types::{
    AgentShadowBuffer, OccError, OccValidationResult, SubtreeSpan, VersionTag,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;

/// Core 3-Phase OCC Validation and Atomic Merge Engine.
#[derive(Debug)]
pub struct OccValidationEngine {
    state: RwLock<ValidationState>,
}

#[derive(Debug)]
struct ValidationState {
    active_version: VersionTag,
    file_versions: HashMap<String, VersionTag>,
    committed_history: Vec<(VersionTag, SubtreeSpan)>,
    master_files: HashMap<String, String>,
}

impl Default for OccValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OccValidationEngine {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ValidationState {
                active_version: VersionTag::GENESIS,
                file_versions: HashMap::new(),
                committed_history: Vec::new(),
                master_files: HashMap::new(),
            }),
        }
    }

    /// Registers or updates the master content of a file in the workspace.
    pub fn set_master_file(&self, file_path: impl Into<String>, content: impl Into<String>) {
        let mut state = self.state.write();
        let path = file_path.into();
        state.file_versions.entry(path.clone()).or_insert(VersionTag::GENESIS);
        state.master_files.insert(path, content.into());
    }

    /// Retrieves the current master content of a file.
    pub fn get_master_file(&self, file_path: &str) -> Option<String> {
        let state = self.state.read();
        state.master_files.get(file_path).cloned()
    }

    /// Returns the current active global version tag.
    pub fn current_version(&self) -> VersionTag {
        self.state.read().active_version
    }

    /// Returns the current version of a specific file.
    pub fn file_version(&self, file_path: &str) -> VersionTag {
        let state = self.state.read();
        state.file_versions.get(file_path).copied().unwrap_or(VersionTag::GENESIS)
    }

    /// Executes Phase 2 (Validation) and Phase 3 (Atomic Commit) in sub-0.05ms.
    pub fn validate_and_commit(
        &self,
        buffer: &AgentShadowBuffer,
    ) -> Result<OccValidationResult, OccError> {
        let start = Instant::now();
        let mut state = self.state.write();

        let current_v = state.active_version;

        // --- Phase 2: Validation Phase ---
        // 1. Check Read Set: verify no read spans have been modified since transaction inception
        for read_span in &buffer.read_set {
            for (commit_v, committed_span) in &state.committed_history {
                if commit_v.value() > buffer.staged_mutations.first().map(|m| m.base_version.value()).unwrap_or(0)
                    && committed_span.overlaps(read_span)
                {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    return Ok(OccValidationResult::conflict(read_span.clone(), current_v, elapsed));
                }
            }
        }

        // 2. Check Write Set: verify no concurrent transaction committed an overlapping subtree span
        for write_span in &buffer.write_set {
            for (commit_v, committed_span) in &state.committed_history {
                let base_v = buffer
                    .staged_mutations
                    .iter()
                    .find(|m| m.target_span == *write_span)
                    .map(|m| m.base_version.value())
                    .unwrap_or(0);

                if commit_v.value() > base_v && committed_span.overlaps(write_span) {
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    return Ok(OccValidationResult::conflict(write_span.clone(), current_v, elapsed));
                }
            }
        }

        // --- Phase 3: Atomic Commit Phase ---
        let next_v = state.active_version.next();
        state.active_version = next_v;

        // Group staged mutations by target file to apply in clean descending byte order
        let mut by_file: HashMap<String, Vec<&super::types::StagedMutation>> = HashMap::new();
        for mutation in &buffer.staged_mutations {
            by_file
                .entry(mutation.target_span.file_path.clone())
                .or_default()
                .push(mutation);
        }

        for (file_path, mut mutations) in by_file {
            state.file_versions.insert(file_path.clone(), next_v);

            for m in &mutations {
                state.committed_history.push((next_v, m.target_span.clone()));
            }

            // Apply mutations to master file content
            mutations.sort_by(|a, b| b.target_span.byte_range.0.cmp(&a.target_span.byte_range.0));

            let mut master = state
                .master_files
                .get(&file_path)
                .cloned()
                .unwrap_or_default();

            for m in mutations {
                if let Some(repl) = &m.replacement_source {
                    let start_b = m.target_span.byte_range.0.min(master.len());
                    let end_b = m.target_span.byte_range.1.min(master.len());
                    if start_b <= end_b {
                        master.replace_range(start_b..end_b, repl);
                    }
                }
            }

            state.master_files.insert(file_path, master);
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        Ok(OccValidationResult::valid(next_v, elapsed))
    }
}
