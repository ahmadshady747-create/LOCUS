//! Multi-Agent Symbol Lease Model & Concurrency Conflict Broker.

#![forbid(unsafe_code)]

use crate::types::SymbolLease;
use std::time::{SystemTime, UNIX_EPOCH};

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

    /// Validate if an FQN format is structurally sound (e.g. `crate::module::symbol`, `path/file.tsx::Symbol`, or wildcard `src/auth/*`).
    pub fn is_valid_fqn(fqn: &str) -> bool {
        let trimmed = fqn.trim();
        !trimmed.is_empty()
            && (trimmed.contains("::")
                || trimmed.contains('/')
                || trimmed.contains('.')
                || trimmed.ends_with('*'))
    }

    /// Returns true if an FQN is a hierarchical wildcard pattern (e.g. `src/auth/*` or `crate::billing::*`).
    pub fn is_wildcard(fqn: &str) -> bool {
        fqn.ends_with('*')
    }

    /// Extract the prefix from a wildcard pattern (e.g. `src/auth/*` -> `src/auth/`).
    pub fn wildcard_prefix(pattern: &str) -> &str {
        if let Some(pos) = pattern.find('*') {
            &pattern[..pos]
        } else {
            pattern
        }
    }

    /// Check if pattern `pattern` matches or contains `target_fqn` hierarchically.
    pub fn matches_hierarchical(pattern: &str, target_fqn: &str) -> bool {
        if pattern == target_fqn {
            return true;
        }

        if Self::is_wildcard(pattern) {
            let prefix = Self::wildcard_prefix(pattern);
            if target_fqn.starts_with(prefix) {
                return true;
            }

            // Path separator normalization ('/' vs '::')
            let norm_prefix = prefix.replace("::", "/");
            let norm_target = target_fqn.replace("::", "/");
            return norm_target.starts_with(&norm_prefix);
        }

        false
    }

    /// Check if two FQN paths hierarchically conflict (either one contains or covers the other).
    pub fn check_hierarchical_conflict(fqn_a: &str, fqn_b: &str) -> bool {
        Self::matches_hierarchical(fqn_a, fqn_b) || Self::matches_hierarchical(fqn_b, fqn_a)
    }
}
