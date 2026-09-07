//! High-Performance Lock-Free Telemetry Collector.
//!
//! Collects real-time runtime metrics (resident RAM, verification latency, active agents,
//! OCC leases, and WAL transaction throughput) using atomic primitives in pure safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Immutable point-in-time telemetry snapshot of the LOCUS runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub timestamp_ns: u64,
    pub resident_ram_kb: usize,
    pub verification_latency_ns: u64,
    pub active_agents: usize,
    pub occ_active_leases: usize,
    pub wal_transactions_committed: u64,
    pub total_operations: u64,
    pub throughput_ops_per_sec: f64,
    pub uptime_secs: f64,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            timestamp_ns: 0,
            resident_ram_kb: 0,
            verification_latency_ns: 0,
            active_agents: 0,
            occ_active_leases: 0,
            wal_transactions_committed: 0,
            total_operations: 0,
            throughput_ops_per_sec: 0.0,
            uptime_secs: 0.0,
        }
    }
}

/// Lock-free thread-safe telemetry collector.
#[derive(Debug)]
pub struct TelemetryCollector {
    resident_ram_kb: AtomicUsize,
    verification_latency_ns: AtomicU64,
    active_agents: AtomicUsize,
    occ_active_leases: AtomicUsize,
    wal_transactions_committed: AtomicU64,
    total_operations: AtomicU64,
    start_time: Instant,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryCollector {
    /// Creates a new telemetry collector with zeroed counters.
    pub fn new() -> Self {
        Self {
            resident_ram_kb: AtomicUsize::new(0),
            verification_latency_ns: AtomicU64::new(0),
            active_agents: AtomicUsize::new(0),
            occ_active_leases: AtomicUsize::new(0),
            wal_transactions_committed: AtomicU64::new(0),
            total_operations: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Records current resident RAM usage in kilobytes.
    #[inline]
    pub fn record_ram_kb(&self, kb: usize) {
        self.resident_ram_kb.store(kb, Ordering::Relaxed);
    }

    /// Records the latest AST verification latency in nanoseconds.
    #[inline]
    pub fn record_verification_latency(&self, latency_ns: u64) {
        self.verification_latency_ns.store(latency_ns, Ordering::Relaxed);
    }

    /// Updates the number of currently active swarm agents.
    #[inline]
    pub fn set_active_agents(&self, count: usize) {
        self.active_agents.store(count, Ordering::Relaxed);
    }

    /// Updates the number of active OCC subtree leases.
    #[inline]
    pub fn set_occ_leases(&self, count: usize) {
        self.occ_active_leases.store(count, Ordering::Relaxed);
    }

    /// Increments the total committed WAL transactions count by 1.
    #[inline]
    pub fn record_wal_commit(&self) {
        self.wal_transactions_committed.fetch_add(1, Ordering::Relaxed);
        self.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the total operations counter.
    #[inline]
    pub fn record_operation(&self) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Captures a consistent immutable point-in-time snapshot of all telemetry metrics.
    pub fn capture_snapshot(&self) -> TelemetrySnapshot {
        let uptime = self.start_time.elapsed();
        let uptime_secs = uptime.as_secs_f64();
        let total_ops = self.total_operations.load(Ordering::Relaxed);

        let throughput = if uptime_secs > 0.0001 {
            (total_ops as f64) / uptime_secs
        } else {
            0.0
        };

        TelemetrySnapshot {
            timestamp_ns: uptime.as_nanos() as u64,
            resident_ram_kb: self.resident_ram_kb.load(Ordering::Relaxed),
            verification_latency_ns: self.verification_latency_ns.load(Ordering::Relaxed),
            active_agents: self.active_agents.load(Ordering::Relaxed),
            occ_active_leases: self.occ_active_leases.load(Ordering::Relaxed),
            wal_transactions_committed: self.wal_transactions_committed.load(Ordering::Relaxed),
            total_operations: total_ops,
            throughput_ops_per_sec: throughput,
            uptime_secs,
        }
    }
}
