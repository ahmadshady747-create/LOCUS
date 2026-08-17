use crate::task::{AgentHandle, AgentStats, AgentStatus};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, warn};
use uuid::Uuid;

pub struct AgentMonitor {
    agents: Arc<DashMap<Uuid, AgentHandle>>,
    stats_cache: Arc<DashMap<Uuid, AgentStats>>,
}

impl AgentMonitor {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            stats_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, handle: AgentHandle) {
        self.agents.insert(handle.id, handle);
    }

    pub fn unregister(&self, agent_id: Uuid) -> Option<AgentHandle> {
        self.agents.remove(&agent_id).map(|(_, v)| v)
    }

    pub fn get(&self, agent_id: Uuid) -> Option<AgentHandle> {
        self.agents.get(&agent_id).map(|v| v.clone())
    }

    pub fn list_active(&self) -> Vec<AgentHandle> {
        self.agents
            .iter()
            .filter(|entry| matches!(
                entry.status,
                AgentStatus::Starting | AgentStatus::Running | AgentStatus::Testing
            ))
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn list_all(&self) -> Vec<AgentHandle> {
        self.agents.iter().map(|entry| entry.clone()).collect()
    }

    pub async fn monitor_agent(&self, agent_id: Uuid) -> Result<AgentStats> {
        if let Some(mut entry) = self.agents.get_mut(&agent_id) {
            let stats = self.collect_stats(&entry).await?;
            self.stats_cache.insert(agent_id, stats.clone());
            Ok(stats)
        } else {
            Err(anyhow::anyhow!("Agent not found: {}", agent_id))
        }
    }

    async fn collect_stats(&self, handle: &AgentHandle) -> Result<AgentStats> {
        let pid = handle.pid.unwrap_or(0);
        
        #[cfg(target_os = "linux")]
        {
            if pid > 0 {
                return self.collect_linux_stats(pid, handle).await;
            }
        }

        Ok(AgentStats {
            cpu_percent: 0.0,
            memory_mb: 0,
            memory_peak_mb: 0,
            disk_read_mb: 0,
            disk_write_mb: 0,
            network_rx_mb: 0,
            network_tx_mb: 0,
            uptime_seconds: handle.started_at
                .map(|t| (chrono::Utc::now() - t).num_seconds() as u64)
                .unwrap_or(0),
            thread_count: 0,
        })
    }

    #[cfg(target_os = "linux")]
    async fn collect_linux_stats(&self, pid: u32, handle: &AgentHandle) -> Result<AgentStats> {
        use std::fs;

        let stat_path = format!("/proc/{}/stat", pid);
        let status_path = format!("/proc/{}/status", pid);
        let io_path = format!("/proc/{}/io", pid);

        let mut cpu_percent = 0.0;
        let mut memory_mb = 0;
        let mut memory_peak_mb = 0;
        let mut thread_count = 0;

        if let Ok(stat_content) = fs::read_to_string(&stat_path) {
            let parts: Vec<&str> = stat_content.split_whitespace().collect();
            if parts.len() >= 22 {
                let utime: u64 = parts[13].parse().unwrap_or(0);
                let stime: u64 = parts[14].parse().unwrap_or(0);
                let starttime: u64 = parts[21].parse().unwrap_or(0);
                let total_time = utime + stime;

                thread_count = parts[19].parse().unwrap_or(0);

                if let Ok(uptime_content) = fs::read_to_string("/proc/uptime") {
                    let uptime_secs: f64 = uptime_content.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    
                    let clk_tck = 100.0;
                    let process_uptime = uptime_secs - (starttime as f64 / clk_tck);
                    if process_uptime > 0.0 {
                        cpu_percent = (total_time as f64 / clk_tck / process_uptime * 100.0) as f32;
                    }
                }
            }
        }

        if let Ok(status_content) = fs::read_to_string(&status_path) {
            for line in status_content.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        memory_mb = parts[1].parse::<u64>().unwrap_or(0) / 1024;
                    }
                } else if line.starts_with("VmPeak:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        memory_peak_mb = parts[1].parse::<u64>().unwrap_or(0) / 1024;
                    }
                }
            }
        }

        let mut disk_read_mb = 0;
        let mut disk_write_mb = 0;
        if let Ok(io_content) = fs::read_to_string(&io_path) {
            for line in io_content.lines() {
                if line.starts_with("read_bytes:") {
                    disk_read_mb = line.split_whitespace().nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0) / (1024 * 1024);
                } else if line.starts_with("write_bytes:") {
                    disk_write_mb = line.split_whitespace().nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0) / (1024 * 1024);
                }
            }
        }

        Ok(AgentStats {
            cpu_percent,
            memory_mb,
            memory_peak_mb,
            disk_read_mb,
            disk_write_mb,
            network_rx_mb: 0,
            network_tx_mb: 0,
            uptime_seconds: handle.started_at
                .map(|t| (chrono::Utc::now() - t).num_seconds() as u64)
                .unwrap_or(0),
            thread_count,
        })
    }

    pub async fn auto_kill_on_timeout(&self, agent_id: Uuid, timeout: Duration) {
        let agents = self.agents.clone();
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            
            if let Some(mut entry) = agents.get_mut(&agent_id) {
                if matches!(entry.status, AgentStatus::Running | AgentStatus::Testing) {
                    warn!("Auto-killing agent {} due to timeout", agent_id);
                    entry.status = AgentStatus::Killed;
                    entry.completed_at = Some(chrono::Utc::now());
                }
            }
        });
    }

    pub async fn auto_kill_on_memory(&self, agent_id: Uuid, max_memory_mb: u64, check_interval: Duration) {
        let agents = self.agents.clone();
        let stats_cache = self.stats_cache.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(check_interval);
            loop {
                ticker.tick().await;
                
                if let Some(entry) = agents.get(&agent_id) {
                    if !matches!(entry.status, AgentStatus::Running | AgentStatus::Testing) {
                        break;
                    }
                    
                    let pid = entry.pid.unwrap_or(0);
                    if pid == 0 {
                        continue;
                    }

                    #[cfg(target_os = "linux")]
                    {
                        if let Ok(status_content) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
                            for line in status_content.lines() {
                                if line.starts_with("VmRSS:") {
                                    let parts: Vec<&str> = line.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        if let Ok(current_mb) = parts[1].parse::<u64>() {
                                            let current_mb = current_mb / 1024;
                                            if current_mb > max_memory_mb {
                                                warn!("Auto-killing agent {} due to memory limit: {}MB > {}MB", 
                                                    agent_id, current_mb, max_memory_mb);
                                                if let Some(mut e) = agents.get_mut(&agent_id) {
                                                    e.status = AgentStatus::Killed;
                                                    e.completed_at = Some(chrono::Utc::now());
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    pub fn get_cached_stats(&self, agent_id: Uuid) -> Option<AgentStats> {
        self.stats_cache.get(&agent_id).map(|v| v.clone())
    }
}

impl Default for AgentMonitor {
    fn default() -> Self {
        Self::new()
    }
}