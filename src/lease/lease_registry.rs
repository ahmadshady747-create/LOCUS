//! In-Memory Lock-Free Symbol Lease Registry.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use parking_lot::RwLock;
use crate::lease::symbol_lease::LeaseBroker;
use crate::types::{LeaseStatus, SymbolLease};

/// In-memory symbol lease registry for multi-agent swarm synchronization.
pub struct LeaseRegistry {
    leases_by_fqn: RwLock<HashMap<String, SymbolLease>>,
    fqn_by_lease_id: RwLock<HashMap<String, String>>,
}

impl Default for LeaseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LeaseRegistry {
    pub fn new() -> Self {
        Self {
            leases_by_fqn: RwLock::new(HashMap::new()),
            fqn_by_lease_id: RwLock::new(HashMap::new()),
        }
    }

    /// Attempt to acquire a lease on an FQN for a specific agent.
    pub fn acquire(&self, fqn: &str, agent_id: &str, ttl_ms: u64) -> LeaseStatus {
        let now = LeaseBroker::current_time_ms();
        let mut by_fqn = self.leases_by_fqn.write();
        let mut by_id = self.fqn_by_lease_id.write();

        // Check existing lease on this symbol
        if let Some(existing) = by_fqn.get(fqn) {
            if !existing.is_expired(now) {
                if existing.holder_agent_id == agent_id {
                    // Agent already holds the lease; automatically renew
                    let updated_lease = SymbolLease {
                        lease_id: existing.lease_id.clone(),
                        fqn: fqn.to_string(),
                        holder_agent_id: agent_id.to_string(),
                        acquired_at_ms: existing.acquired_at_ms,
                        ttl_ms,
                        expires_at_ms: now + ttl_ms,
                    };
                    by_fqn.insert(fqn.to_string(), updated_lease.clone());
                    return LeaseStatus::Renewed(updated_lease);
                } else {
                    // Conflict: Held by another agent
                    return LeaseStatus::Conflict {
                        fqn: fqn.to_string(),
                        current_holder: existing.holder_agent_id.clone(),
                        remaining_ttl_ms: existing.remaining_ttl_ms(now),
                    };
                }
            } else {
                // Expired lease: clean previous lease_id mapping
                by_id.remove(&existing.lease_id);
            }
        }

        // Issue new lease
        let lease = LeaseBroker::create_lease(fqn, agent_id, ttl_ms);
        by_fqn.insert(fqn.to_string(), lease.clone());
        by_id.insert(lease.lease_id.clone(), fqn.to_string());

        LeaseStatus::Acquired(lease)
    }

    /// Renew an existing lease by lease ID.
    pub fn renew(&self, lease_id: &str, agent_id: &str, extension_ms: u64) -> LeaseStatus {
        let now = LeaseBroker::current_time_ms();
        let mut by_fqn = self.leases_by_fqn.write();
        let by_id = self.fqn_by_lease_id.read();

        let fqn = match by_id.get(lease_id) {
            Some(f) => f.clone(),
            None => return LeaseStatus::NotFound,
        };

        if let Some(existing) = by_fqn.get_mut(&fqn) {
            if existing.holder_agent_id == agent_id {
                existing.expires_at_ms = now + extension_ms;
                existing.ttl_ms = extension_ms;
                return LeaseStatus::Renewed(existing.clone());
            } else {
                return LeaseStatus::Conflict {
                    fqn: fqn.clone(),
                    current_holder: existing.holder_agent_id.clone(),
                    remaining_ttl_ms: existing.remaining_ttl_ms(now),
                };
            }
        }

        LeaseStatus::NotFound
    }

    /// Release an active lease by lease ID.
    pub fn release(&self, lease_id: &str, agent_id: &str) -> LeaseStatus {
        let mut by_fqn = self.leases_by_fqn.write();
        let mut by_id = self.fqn_by_lease_id.write();

        let fqn = match by_id.remove(lease_id) {
            Some(f) => f,
            None => return LeaseStatus::NotFound,
        };

        if let Some(existing) = by_fqn.get(&fqn) {
            if existing.holder_agent_id == agent_id {
                by_fqn.remove(&fqn);
                return LeaseStatus::Released;
            }
        }

        LeaseStatus::NotFound
    }

    /// Check if a symbol is actively leased and not expired.
    pub fn get_active_lease(&self, fqn: &str) -> Option<SymbolLease> {
        let now = LeaseBroker::current_time_ms();
        let by_fqn = self.leases_by_fqn.read();
        by_fqn.get(fqn).and_then(|l| {
            if !l.is_expired(now) {
                Some(l.clone())
            } else {
                None
            }
        })
    }

    /// Garbage collect expired leases.
    pub fn clean_expired(&self) -> usize {
        let now = LeaseBroker::current_time_ms();
        let mut by_fqn = self.leases_by_fqn.write();
        let mut by_id = self.fqn_by_lease_id.write();

        let mut expired_fqns = Vec::new();
        for (fqn, lease) in by_fqn.iter() {
            if lease.is_expired(now) {
                expired_fqns.push((fqn.clone(), lease.lease_id.clone()));
            }
        }

        let count = expired_fqns.len();
        for (fqn, lease_id) in expired_fqns {
            by_fqn.remove(&fqn);
            by_id.remove(&lease_id);
        }

        count
    }

    /// List all currently active leases.
    pub fn list_active_leases(&self) -> Vec<SymbolLease> {
        let now = LeaseBroker::current_time_ms();
        let by_fqn = self.leases_by_fqn.read();
        by_fqn.values()
            .filter(|l| !l.is_expired(now))
            .cloned()
            .collect()
    }
}
