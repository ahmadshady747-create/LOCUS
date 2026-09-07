//! Subtree Optimistic Concurrency Control (OCC) & Isolated Shadow Buffers Engine Module.
//!
//! Coordinates up to 300 autonomous swarm agents modifying hierarchical AST subtrees concurrently
//! with zero global write-locks, private virtual overlays, and deterministic monotonic versioning.

#![forbid(unsafe_code)]

pub mod shadow_buffer;
pub mod types;
pub mod validator;

pub use shadow_buffer::{ShadowBufferManager, MAX_SWARM_BUFFERS};
pub use types::{
    AgentShadowBuffer, BufferId, OccError, OccLockState, OccValidationResult, StagedMutation,
    SubtreeSpan, VersionTag,
};
pub use validator::OccValidationEngine;

use std::sync::Arc;

/// Central Swarm Workspace Coordinator managing per-agent shadow buffers and atomic OCC commits.
#[derive(Debug, Default, Clone)]
pub struct SwarmWorkspaceManager {
    buffer_mgr: Arc<ShadowBufferManager>,
    validator: Arc<OccValidationEngine>,
}

impl SwarmWorkspaceManager {
    pub fn new() -> Self {
        Self {
            buffer_mgr: Arc::new(ShadowBufferManager::new()),
            validator: Arc::new(OccValidationEngine::new()),
        }
    }

    pub fn buffer_manager(&self) -> &ShadowBufferManager {
        &self.buffer_mgr
    }

    pub fn validator(&self) -> &OccValidationEngine {
        &self.validator
    }

    /// Sets up a master source file.
    pub fn init_file(&self, file_path: impl Into<String>, content: impl Into<String>) {
        self.validator.set_master_file(file_path, content);
    }

    /// Allocates an isolated shadow buffer for an agent.
    pub fn open_buffer(&self, agent_id: u32) -> Result<BufferId, OccError> {
        self.buffer_mgr.create_buffer(agent_id)
    }

    /// Stages a mutation within the agent's private buffer.
    pub fn stage_change(
        &self,
        buffer_id: BufferId,
        target_span: SubtreeSpan,
        replacement: impl Into<String>,
        base_version: VersionTag,
    ) -> Result<(), OccError> {
        let mutation = StagedMutation::new(1, target_span, replacement, base_version);
        self.buffer_mgr.stage_mutation(buffer_id, mutation)
    }

    /// Reads a file through the agent's private virtual overlay.
    pub fn read_virtual_file(&self, buffer_id: BufferId, file_path: &str) -> Result<String, OccError> {
        let master = self.validator.get_master_file(file_path).unwrap_or_default();
        self.buffer_mgr.render_overlay(buffer_id, file_path, &master)
    }

    /// Validates and atomically commits an agent's shadow buffer.
    pub fn commit_buffer(&self, buffer_id: BufferId) -> Result<OccValidationResult, OccError> {
        let buffer = self.buffer_mgr.get_buffer(buffer_id)?;
        let result = self.validator.validate_and_commit(&buffer)?;
        if result.is_valid {
            let _ = self.buffer_mgr.discard_buffer(buffer_id);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_10_parallel_agents_disjoint_subtrees_all_commit_successfully() {
        let manager = SwarmWorkspaceManager::new();
        let file = "src/models/user.rs";
        let initial_code = "fn a() {} \nfn b() {} \nfn c() {} \nfn d() {} \nfn e() {} \nfn f() {} \nfn g() {} \nfn h() {} \nfn i() {} \nfn j() {}";
        manager.init_file(file, initial_code);

        let base_v = manager.validator().current_version();

        let mut buffers = Vec::new();
        // Allocate 10 buffers for 10 agents editing distinct functions at disjoint byte ranges
        for agent_id in 0..10 {
            let buf_id = manager.open_buffer(agent_id).expect("Buffer allocated");
            let start_idx = (agent_id as usize) * 11;
            let end_idx = start_idx + 9;
            let span = SubtreeSpan::new(file, (agent_id as u64) + 1, (start_idx, end_idx));
            manager
                .stage_change(buf_id, span, format!("fn f_{}() {{}}", agent_id), base_v)
                .expect("Mutation staged");
            buffers.push(buf_id);
        }

        // All 10 commit in sequence without any conflict
        for buf_id in buffers {
            let result = manager.commit_buffer(buf_id).expect("Commit call succeeds");
            assert!(result.is_valid, "Disjoint subtree commit must be valid");
        }

        assert_eq!(manager.validator().current_version().value(), 10);
    }

    #[test]
    fn test_conflict_detection_and_instant_abort_on_overlapping_spans() {
        let manager = SwarmWorkspaceManager::new();
        let file = "src/auth/token.rs";
        manager.init_file(file, "pub fn verify_token() -> bool { true }");

        let base_v = manager.validator().current_version();

        // Agent 1 and Agent 2 open buffers on the exact same span
        let span = SubtreeSpan::new(file, 0x1234, (0, 37));
        let buf_1 = manager.open_buffer(1).expect("Buffer 1 opened");
        let buf_2 = manager.open_buffer(2).expect("Buffer 2 opened");

        manager
            .stage_change(buf_1, span.clone(), "pub fn verify_token() -> bool { false }", base_v)
            .expect("Agent 1 stages");

        manager
            .stage_change(buf_2, span.clone(), "pub fn verify_token() -> bool { check() }", base_v)
            .expect("Agent 2 stages");

        // Agent 1 commits first -> Success, bumps to V_1
        let res_1 = manager.commit_buffer(buf_1).expect("Agent 1 commits");
        assert!(res_1.is_valid);
        assert_eq!(res_1.current_version, VersionTag::new(1));

        // Agent 2 tries to commit with stale base V_0 -> Conflict & Abort
        let res_2 = manager.commit_buffer(buf_2).expect("Agent 2 validation call");
        assert!(!res_2.is_valid, "Agent 2 must be rejected due to overlapping span commit");
        assert_eq!(res_2.conflict_span, Some(span.clone()));

        // Agent 2 retries against new version V_1 -> Success, bumps to V_2
        let retry_buf = manager.open_buffer(2).expect("Retry buffer");
        manager
            .stage_change(retry_buf, span, "pub fn verify_token() -> bool { check() }", res_1.current_version)
            .expect("Retry stage");

        let res_retry = manager.commit_buffer(retry_buf).expect("Retry commit");
        assert!(res_retry.is_valid);
        assert_eq!(res_retry.current_version, VersionTag::new(2));
    }

    #[test]
    fn test_shadow_buffer_isolation_uncommitted_changes_are_invisible() {
        let manager = SwarmWorkspaceManager::new();
        let file = "src/secret.rs";
        let initial = "const SECRET: &str = \"original\";";
        manager.init_file(file, initial);

        let base_v = manager.validator().current_version();
        let buf_1 = manager.open_buffer(1).expect("Buffer 1");
        let span = SubtreeSpan::new(file, 0, (0, initial.len()));

        manager
            .stage_change(buf_1, span, "const SECRET: &str = \"mutated_draft\";", base_v)
            .expect("Staged");

        // Agent 1 sees its private mutated overlay
        let agent1_view = manager.read_virtual_file(buf_1, file).expect("Agent 1 view");
        assert_eq!(agent1_view, "const SECRET: &str = \"mutated_draft\";");

        // Master workspace and another agent see the original uncommitted source
        let buf_2 = manager.open_buffer(2).expect("Buffer 2");
        let agent2_view = manager.read_virtual_file(buf_2, file).expect("Agent 2 view");
        assert_eq!(agent2_view, "const SECRET: &str = \"original\";");
    }

    #[test]
    fn test_sub_0_05ms_commit_latency_budget() {
        let manager = SwarmWorkspaceManager::new();
        let file = "src/perf/bench.rs";
        manager.init_file(file, "pub fn fast() -> i32 { 42 }");

        let iterations = 1000;
        let mut total_latency_us = 0.0;

        for i in 0..iterations {
            let base_v = manager.validator().current_version();
            let buf = manager.open_buffer(i as u32).expect("Buffer");
            let span = SubtreeSpan::new(file, (i as u64) + 1, (0, 27));
            manager
                .stage_change(buf, span, format!("pub fn fast_{}() -> i32 {{ {} }}", i, i), base_v)
                .expect("Stage");

            let start = Instant::now();
            let res = manager.commit_buffer(buf).expect("Commit");
            let elapsed = start.elapsed();
            total_latency_us += elapsed.as_micros() as f64;
            assert!(res.is_valid);
        }

        let avg_us = total_latency_us / (iterations as f64);
        println!("⚡ Average OCC validation & atomic commit latency: {:.3} µs", avg_us);

        let limit = if cfg!(debug_assertions) { 1000.0 } else { 50.0 };
        assert!(
            avg_us < limit,
            "OCC commit must execute in sub-0.05ms (<50µs in release), got: {:.3} µs",
            avg_us
        );
    }
}
