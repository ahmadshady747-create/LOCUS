//! Multi-Agent Symbol Lease Model & Concurrency Conflict Broker.

#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::SymbolLease;

pub struct LeaseBroker;

impl LeaseBroker {
    /// Get the current system timestamp in milliseconds.
    pub fn current_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Create a new symbol lease with given FQN, agent ID, and TTL in milliseconds.
    pub fn create_lease(fqn: &str, agent_id: &str, ttl_ms: u64) -> SymbolLease {
        let now = Self::current_time_ms();
        let lease_id = format!("lease_{:x}_{}", now, crate::types::fnv1a_64(fqn.as_bytes()));
        SymbolLease {
            lease_id,
            fqn: fqn.to_string(),
            holder_agent_id: agent_id.to_string(),
            acquired_at_ms: now,
            ttl_ms,
            expires_at_ms: now + ttl_ms,
        }
    }

    /// Validate if an FQN format is structurally sound (e.g. `crate::module::symbol` or `path/file.tsx::Symbol`).
    pub fn is_valid_fqn(fqn: &str) -> bool {
        !fqn.trim().is_empty() && (fqn.contains("::") || fqn.contains('/') || fqn.contains('.'))
    }
}
