//! Isolated Shadow Buffer Manager for 300 Swarm Agents.
//!
//! Allocates and maintains private, virtual overlay buffers where agents stage mutations
//! in complete isolation from the master CST tree and peer agents.

#![forbid(unsafe_code)]

use super::types::{AgentShadowBuffer, BufferId, OccError, StagedMutation, SubtreeSpan};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum concurrent shadow buffers supported by the swarm OS.
pub const MAX_SWARM_BUFFERS: usize = 300;

/// High-throughput manager for concurrent per-agent shadow buffers.
#[derive(Debug)]
pub struct ShadowBufferManager {
    buffers: RwLock<HashMap<BufferId, AgentShadowBuffer>>,
    sequence_counter: AtomicU64,
}

impl Default for ShadowBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadowBufferManager {
    pub fn new() -> Self {
        Self {
            buffers: RwLock::new(HashMap::with_capacity(MAX_SWARM_BUFFERS)),
            sequence_counter: AtomicU64::new(1),
        }
    }

    /// Allocates an isolated shadow buffer for an autonomous agent.
    pub fn create_buffer(&self, agent_id: u32) -> Result<BufferId, OccError> {
        let mut map = self.buffers.write();
        if map.len() >= MAX_SWARM_BUFFERS {
            return Err(OccError::CapacityExceeded(map.len()));
        }

        let seq = self.sequence_counter.fetch_add(1, Ordering::Relaxed);
        let buffer_id = BufferId::new(agent_id, seq);
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let shadow = AgentShadowBuffer::new(buffer_id, agent_id, created_at_ms);
        map.insert(buffer_id, shadow);

        Ok(buffer_id)
    }

    /// Stages a mutation within the agent's private shadow buffer.
    pub fn stage_mutation(&self, buffer_id: BufferId, mutation: StagedMutation) -> Result<(), OccError> {
        let mut map = self.buffers.write();
        let buffer = map.get_mut(&buffer_id).ok_or(OccError::BufferNotFound(buffer_id))?;
        buffer.stage(mutation);
        Ok(())
    }

    /// Records an AST subtree read dependency into the agent's shadow buffer read_set.
    pub fn record_read(&self, buffer_id: BufferId, span: SubtreeSpan) -> Result<(), OccError> {
        let mut map = self.buffers.write();
        let buffer = map.get_mut(&buffer_id).ok_or(OccError::BufferNotFound(buffer_id))?;
        buffer.record_read(span);
        Ok(())
    }

    /// Retrieves an immutable snapshot of the agent's shadow buffer.
    pub fn get_buffer(&self, buffer_id: BufferId) -> Result<AgentShadowBuffer, OccError> {
        let map = self.buffers.read();
        map.get(&buffer_id)
            .cloned()
            .ok_or(OccError::BufferNotFound(buffer_id))
    }

    /// Discards and releases a shadow buffer upon transaction commit or abort.
    pub fn discard_buffer(&self, buffer_id: BufferId) -> Result<(), OccError> {
        let mut map = self.buffers.write();
        map.remove(&buffer_id)
            .map(|_| ())
            .ok_or(OccError::BufferNotFound(buffer_id))
    }

    /// Renders the virtual overlay for a file by applying all staged mutations on top of the master source.
    pub fn render_overlay(
        &self,
        buffer_id: BufferId,
        file_path: &str,
        master_source: &str,
    ) -> Result<String, OccError> {
        let buffer = self.get_buffer(buffer_id)?;
        let mut result = master_source.to_string();

        // Apply mutations targeting this file in descending order of byte offset to maintain valid offsets
        let mut file_mutations: Vec<&StagedMutation> = buffer
            .staged_mutations
            .iter()
            .filter(|m| m.target_span.file_path == file_path)
            .collect();

        file_mutations.sort_by(|a, b| b.target_span.byte_range.0.cmp(&a.target_span.byte_range.0));

        for m in file_mutations {
            if let Some(replacement) = &m.replacement_source {
                let start = m.target_span.byte_range.0.min(result.len());
                let end = m.target_span.byte_range.1.min(result.len());
                if start <= end {
                    result.replace_range(start..end, replacement);
                }
            }
        }

        Ok(result)
    }

    /// Returns the number of currently active shadow buffers.
    pub fn active_buffers_count(&self) -> usize {
        self.buffers.read().len()
    }
}
