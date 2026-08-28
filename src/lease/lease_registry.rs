//! In-Memory Lock-Free Symbol Lease Registry with Hierarchical Wildcards, OCC, and Deadlock Detection.

#![forbid(unsafe_code)]

use crate::lease::symbol_lease::LeaseBroker;
use crate::types::{DeadlockResolution, LeaseStatus, SymbolLease};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

/// In-memory symbol lease registry for multi-agent swarm synchronization.
pub struct LeaseRegistry {
    leases_by_fqn: RwLock<HashMap<String, SymbolLease>>,
    fqn_by_lease_id: RwLock<HashMap<String, String>>,
    occ_versions: RwLock<HashMap<String, u64>>,
    wait_for_graph: RwLock<HashMap<String, HashSet<String>>>, // waiter_agent -> set of blocking holders
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
            occ_versions: RwLock::new(HashMap::new()),
            wait_for_graph: RwLock::new(HashMap::new()),
        }
    }

    /// Attempt to acquire a lease on an FQN or Wildcard pattern for a specific agent.
    pub fn acquire(&self, fqn: &str, agent_id: &str, ttl_ms: u64) -> LeaseStatus {
        let now = LeaseBroker::current_time_ms();
        let mut by_fqn = self.leases_by_fqn.write();
        let mut by_id = self.fqn_by_lease_id.write();

        // 1. Check exact match on this FQN
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

        // 2. Hierarchical Sub-Tree Conflict Check (Wildcard vs Single FQN)
        for (active_fqn, active_lease) in by_fqn.iter() {
            if !active_lease.is_expired(now) && active_lease.holder_agent_id != agent_id {
                if LeaseBroker::check_hierarchical_conflict(fqn, active_fqn) {
                    return LeaseStatus::HierarchicalConflict {
                        requested_fqn: fqn.to_string(),
                        blocking_lease_fqn: active_fqn.clone(),
                        current_holder: active_lease.holder_agent_id.clone(),
                        remaining_ttl_ms: active_lease.remaining_ttl_ms(now),
                    };
                }
            }
        }

        // 3. Issue new lease
        let lease = LeaseBroker::create_lease(fqn, agent_id, ttl_ms);
        by_fqn.insert(fqn.to_string(), lease.clone());
        by_id.insert(lease.lease_id.clone(), fqn.to_string());

        // Remove any waiting edges for this agent on this fqn
        self.remove_wait(agent_id);

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

    /// Check if a symbol or module is actively leased and not expired.
    pub fn get_active_lease(&self, fqn: &str) -> Option<SymbolLease> {
        let now = LeaseBroker::current_time_ms();
        let by_fqn = self.leases_by_fqn.read();

        // 1. Direct exact match
        if let Some(l) = by_fqn.get(fqn) {
            if !l.is_expired(now) {
                return Some(l.clone());
            }
        }

        // 2. Hierarchical wildcard match
        for (active_fqn, lease) in by_fqn.iter() {
            if !lease.is_expired(now) && LeaseBroker::matches_hierarchical(active_fqn, fqn) {
                return Some(lease.clone());
            }
        }

        None
    }

    // -----------------------------------------------------------------------
    // Optimistic Concurrency Control (OCC)
    // -----------------------------------------------------------------------

    /// Get current monotonic OCC version for an FQN (defaults to 1).
    pub fn get_occ_version(&self, fqn: &str) -> u64 {
        *self.occ_versions.read().get(fqn).unwrap_or(&1)
    }

    /// Verify that an FQN has not been concurrently modified.
    pub fn verify_occ_token(&self, fqn: &str, expected_version: u64) -> Result<u64, LeaseStatus> {
        let current = self.get_occ_version(fqn);
        if current == expected_version {
            Ok(current)
        } else {
            Err(LeaseStatus::OccMismatch {
                fqn: fqn.to_string(),
                current_version: current,
                expected_version,
            })
        }
    }

    /// Atomically commit modification with OCC version advancement.
    pub fn commit_occ(&self, fqn: &str, expected_version: u64) -> Result<u64, LeaseStatus> {
        let mut occ = self.occ_versions.write();
        let current = *occ.get(fqn).unwrap_or(&1);

        if current == expected_version {
            let next_version = current + 1;
            occ.insert(fqn.to_string(), next_version);
            Ok(next_version)
        } else {
            Err(LeaseStatus::OccMismatch {
                fqn: fqn.to_string(),
                current_version: current,
                expected_version,
            })
        }
    }

    // -----------------------------------------------------------------------
    // Deadlock Detection & Swarm Resolution (Wait-For Graph)
    // -----------------------------------------------------------------------

    /// Register that an agent is waiting for a lease held by another agent.
    pub fn register_wait(&self, waiter_agent: &str, fqn: &str) -> Result<(), DeadlockResolution> {
        if let Some(active_lease) = self.get_active_lease(fqn) {
            let holder = &active_lease.holder_agent_id;
            if holder != waiter_agent {
                {
                    let mut wfg = self.wait_for_graph.write();
                    wfg.entry(waiter_agent.to_string())
                        .or_default()
                        .insert(holder.clone());
                }

                // Check if this wait introduces a directed cycle
                if let Some(cycle) = self.find_cycle_from(waiter_agent) {
                    let resolution = self.resolve_cycle(&cycle);
                    return Err(resolution);
                }
            }
        }
        Ok(())
    }

    /// Remove all waiting edges for a specific agent.
    pub fn remove_wait(&self, waiter_agent: &str) {
        let mut wfg = self.wait_for_graph.write();
        wfg.remove(waiter_agent);
        for holders in wfg.values_mut() {
            holders.remove(waiter_agent);
        }
    }

    /// Find all directed deadlock cycles in the Wait-For Graph.
    pub fn detect_deadlocks(&self) -> Vec<Vec<String>> {
        let wfg = self.wait_for_graph.read();
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();

        for start_node in wfg.keys() {
            if !visited.contains(start_node) {
                let mut path = Vec::new();
                let mut on_path = HashSet::new();
                self.dfs_cycle(start_node, &wfg, &mut path, &mut on_path, &mut cycles);
                visited.insert(start_node.clone());
            }
        }

        cycles
    }

    /// Automatically detect and break all active deadlock cycles.
    pub fn detect_and_resolve_deadlocks(&self) -> Vec<DeadlockResolution> {
        let cycles = self.detect_deadlocks();
        let mut resolutions = Vec::new();

        for cycle in cycles {
            let resolution = self.resolve_cycle(&cycle);
            resolutions.push(resolution);
        }

        resolutions
    }

    fn find_cycle_from(&self, start: &str) -> Option<Vec<String>> {
        let wfg = self.wait_for_graph.read();
        let mut path = Vec::new();
        let mut on_path = HashSet::new();
        let mut cycles = Vec::new();
        self.dfs_cycle(start, &wfg, &mut path, &mut on_path, &mut cycles);
        cycles.into_iter().next()
    }

    fn dfs_cycle(
        &self,
        curr: &str,
        wfg: &HashMap<String, HashSet<String>>,
        path: &mut Vec<String>,
        on_path: &mut HashSet<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        path.push(curr.to_string());
        on_path.insert(curr.to_string());

        if let Some(neighbors) = wfg.get(curr) {
            for next in neighbors {
                if on_path.contains(next) {
                    // Cycle detected: extract subslice from 'next' to end
                    if let Some(idx) = path.iter().position(|p| p == next) {
                        let mut cycle = path[idx..].to_vec();
                        cycle.push(next.clone());
                        cycles.push(cycle);
                    }
                } else if !path.contains(next) {
                    self.dfs_cycle(next, wfg, path, on_path, cycles);
                }
            }
        }

        path.pop();
        on_path.remove(curr);
    }

    fn resolve_cycle(&self, cycle: &[String]) -> DeadlockResolution {
        let now = LeaseBroker::current_time_ms();
        let mut by_fqn = self.leases_by_fqn.write();
        let mut by_id = self.fqn_by_lease_id.write();

        // Evict the lease of the agent in the cycle with the most recent acquisition timestamp
        let mut victim_agent = &cycle[0];
        let mut victim_lease: Option<SymbolLease> = None;

        for agent in cycle {
            for lease in by_fqn.values() {
                if &lease.holder_agent_id == agent {
                    match &victim_lease {
                        None => {
                            victim_lease = Some(lease.clone());
                            victim_agent = agent;
                        }
                        Some(current_vic) => {
                            if lease.acquired_at_ms > current_vic.acquired_at_ms {
                                victim_lease = Some(lease.clone());
                                victim_agent = agent;
                            }
                        }
                    }
                }
            }
        }

        let (broken_lease_id, broken_fqn) = if let Some(vl) = victim_lease {
            by_fqn.remove(&vl.fqn);
            by_id.remove(&vl.lease_id);
            (vl.lease_id, vl.fqn)
        } else {
            ("none".to_string(), "none".to_string())
        };

        // Clear edges involving victim from wait-for graph
        {
            let mut wfg = self.wait_for_graph.write();
            wfg.remove(victim_agent);
            for holders in wfg.values_mut() {
                holders.remove(victim_agent);
            }
        }

        DeadlockResolution {
            cycle: cycle.to_vec(),
            broken_lease_id,
            broken_fqn,
            evicted_agent: victim_agent.to_string(),
            resolved_at_ms: now,
        }
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
        by_fqn
            .values()
            .filter(|l| !l.is_expired(now))
            .cloned()
            .collect()
    }
}
