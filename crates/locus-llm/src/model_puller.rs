//! Silent Streaming Model Puller Engine
//!
//! Connects to the local Ollama daemon streaming API (`POST /api/pull`) to download,
//! verify, and track models in real-time with byte counts, percentage, transfer speed (MB/s),
//! and ETA calculations.

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Progress snapshot of a model download job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPullProgress {
    pub job_id: String,
    pub model_name: String,
    pub status: String,
    pub digest: Option<String>,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub speed_mb_per_sec: f32,
    pub eta_seconds: Option<u64>,
    pub is_done: bool,
    pub error: Option<String>,
}

struct ActiveJobHandle {
    progress: Arc<RwLock<ModelPullProgress>>,
    cancel_flag: Arc<AtomicBool>,
}

static ACTIVE_JOBS: Lazy<DashMap<String, ActiveJobHandle>> = Lazy::new(DashMap::new);

/// Raw JSON object sent by Ollama streaming `/api/pull` endpoint.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OllamaPullChunk {
    pub status: Option<String>,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
    pub error: Option<String>,
}

pub struct ModelPullerEngine;

impl ModelPullerEngine {
    /// Starts a streaming download job in the background and returns a unique job_id.
    pub async fn start_pull(model_name: String, endpoint_url: Option<String>) -> Result<String> {
        let base_url = endpoint_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let job_id = uuid::Uuid::new_v4().to_string();

        let initial_progress = ModelPullProgress {
            job_id: job_id.clone(),
            model_name: model_name.clone(),
            status: "Initiating pull request...".to_string(),
            digest: None,
            completed_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            speed_mb_per_sec: 0.0,
            eta_seconds: None,
            is_done: false,
            error: None,
        };

        let progress_arc = Arc::new(RwLock::new(initial_progress));
        let cancel_flag = Arc::new(AtomicBool::new(false));

        ACTIVE_JOBS.insert(
            job_id.clone(),
            ActiveJobHandle {
                progress: progress_arc.clone(),
                cancel_flag: cancel_flag.clone(),
            },
        );

        // Spawn async background streaming task
        let job_id_clone = job_id.clone();
        let model_name_clone = model_name.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_pull_stream(
                job_id_clone,
                model_name_clone,
                base_url,
                progress_arc,
                cancel_flag,
            )
            .await
            {
                error!("Model pull stream failed: {:#}", e);
            }
        });

        Ok(job_id)
    }

    /// Gets current progress snapshot for a job.
    pub async fn get_progress(job_id: &str) -> Option<ModelPullProgress> {
        if let Some(entry) = ACTIVE_JOBS.get(job_id) {
            let guard = entry.progress.read().await;
            Some(guard.clone())
        } else {
            None
        }
    }

    /// Cancels a running pull job.
    pub fn cancel_pull(job_id: &str) -> Result<()> {
        if let Some(entry) = ACTIVE_JOBS.get(job_id) {
            entry.cancel_flag.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err(anyhow!("Job ID not found"))
        }
    }

    /// Pure parsing function: transforms a raw Ollama chunk JSON into progress updates.
    pub fn parse_pull_chunk(
        chunk: &OllamaPullChunk,
        current_progress: &mut ModelPullProgress,
        last_bytes: &mut u64,
        last_time: &mut Instant,
    ) {
        if let Some(ref err) = chunk.error {
            current_progress.status = "Failed".to_string();
            current_progress.is_done = true;
            current_progress.error = Some(err.clone());
            return;
        }

        if let Some(ref status) = chunk.status {
            current_progress.status = status.clone();

            if status.eq_ignore_ascii_case("success") {
                current_progress.percentage = 100.0;
                current_progress.is_done = true;
                current_progress.status = "Completed".to_string();
                return;
            }
        }

        if let Some(ref digest) = chunk.digest {
            current_progress.digest = Some(digest.clone());
        }

        let total = chunk.total.unwrap_or(0);
        let completed = chunk.completed.unwrap_or(0);

        if total > 0 {
            current_progress.total_bytes = total;
            current_progress.completed_bytes = completed;

            let pct = (completed as f32 / total as f32) * 100.0;
            current_progress.percentage = (pct * 10.0).round() / 10.0;

            let now = Instant::now();
            let elapsed_sec = now.duration_since(*last_time).as_secs_f32();

            if elapsed_sec >= 0.5 && completed >= *last_bytes {
                let bytes_diff = completed - *last_bytes;
                let speed_mb = (bytes_diff as f32 / (1024.0 * 1024.0)) / elapsed_sec;
                current_progress.speed_mb_per_sec = (speed_mb * 10.0).round() / 10.0;

                if speed_mb > 0.05 {
                    let remaining_bytes = total.saturating_sub(completed);
                    let remaining_mb = remaining_bytes as f32 / (1024.0 * 1024.0);
                    let eta = (remaining_mb / speed_mb) as u64;
                    current_progress.eta_seconds = Some(eta);
                }

                *last_bytes = completed;
                *last_time = now;
            }
        }
    }

    // --- Background Stream Runner ---

    async fn run_pull_stream(
        _job_id: String,
        model_name: String,
        base_url: String,
        progress_arc: Arc<RwLock<ModelPullProgress>>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()?;

        let pull_url = format!("{}/api/pull", base_url);
        let payload = serde_json::json!({
            "name": model_name,
            "stream": true
        });

        let res = client.post(&pull_url).json(&payload).send().await;

        let response = match res {
            Ok(r) => {
                if !r.status().is_success() {
                    let err_text = r.text().await.unwrap_or_default();
                    let mut guard = progress_arc.write().await;
                    guard.status = "Failed".to_string();
                    guard.is_done = true;
                    guard.error = Some(format!("Ollama daemon returned error: {}", err_text));
                    return Ok(());
                }
                r
            }
            Err(e) => {
                let mut guard = progress_arc.write().await;
                guard.status = "Failed".to_string();
                guard.is_done = true;
                guard.error = Some(format!("Failed to connect to Ollama at {}: {}", base_url, e));
                return Ok(());
            }
        };

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut last_bytes = 0u64;
        let mut last_time = Instant::now();

        while let Some(chunk_res) = stream.next().await {
            if cancel_flag.load(Ordering::SeqCst) {
                let mut guard = progress_arc.write().await;
                guard.status = "Cancelled".to_string();
                guard.is_done = true;
                guard.error = Some("Download cancelled by user".to_string());
                return Ok(());
            }

            match chunk_res {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    while let Some(idx) = buffer.find('\n') {
                        let line = buffer[..idx].trim().to_string();
                        buffer = buffer[idx + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(chunk) = serde_json::from_str::<OllamaPullChunk>(&line) {
                            let mut guard = progress_arc.write().await;
                            Self::parse_pull_chunk(
                                &chunk,
                                &mut guard,
                                &mut last_bytes,
                                &mut last_time,
                            );
                        }
                    }
                }
                Err(e) => {
                    let mut guard = progress_arc.write().await;
                    guard.status = "Failed".to_string();
                    guard.is_done = true;
                    guard.error = Some(format!("Stream read error: {}", e));
                    return Ok(());
                }
            }
        }

        // Final check: if stream ended without explicit success chunk, mark as completed
        let mut guard = progress_arc.write().await;
        if !guard.is_done {
            guard.is_done = true;
            guard.status = "Completed".to_string();
            guard.percentage = 100.0;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pull_chunk_downloading_progress() {
        let mut progress = ModelPullProgress {
            job_id: "test-job".to_string(),
            model_name: "qwen2.5-coder:7b".to_string(),
            status: "Starting".to_string(),
            digest: None,
            completed_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            speed_mb_per_sec: 0.0,
            eta_seconds: None,
            is_done: false,
            error: None,
        };

        let chunk = OllamaPullChunk {
            status: Some("downloading sha256:9b0d2d3a".to_string()),
            digest: Some("sha256:9b0d2d3a".to_string()),
            total: Some(4_000_000_000),
            completed: Some(2_000_000_000),
            error: None,
        };

        let mut last_bytes = 0;
        let mut last_time = Instant::now() - std::time::Duration::from_secs(1);

        ModelPullerEngine::parse_pull_chunk(&chunk, &mut progress, &mut last_bytes, &mut last_time);

        assert_eq!(progress.status, "downloading sha256:9b0d2d3a");
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.completed_bytes, 2_000_000_000);
        assert_eq!(progress.total_bytes, 4_000_000_000);
        assert!(!progress.is_done);
    }

    #[test]
    fn test_parse_pull_chunk_success_completes_job() {
        let mut progress = ModelPullProgress {
            job_id: "test-job".to_string(),
            model_name: "qwen2.5-coder:7b".to_string(),
            status: "Starting".to_string(),
            digest: None,
            completed_bytes: 4_000_000_000,
            total_bytes: 4_000_000_000,
            percentage: 99.0,
            speed_mb_per_sec: 25.0,
            eta_seconds: Some(1),
            is_done: false,
            error: None,
        };

        let chunk = OllamaPullChunk {
            status: Some("success".to_string()),
            digest: None,
            total: None,
            completed: None,
            error: None,
        };

        let mut last_bytes = 4_000_000_000;
        let mut last_time = Instant::now();

        ModelPullerEngine::parse_pull_chunk(&chunk, &mut progress, &mut last_bytes, &mut last_time);

        assert_eq!(progress.status, "Completed");
        assert_eq!(progress.percentage, 100.0);
        assert!(progress.is_done);
        assert!(progress.error.is_none());
    }

    #[test]
    fn test_parse_pull_chunk_error_flags_failure() {
        let mut progress = ModelPullProgress {
            job_id: "test-job".to_string(),
            model_name: "nonexistent:model".to_string(),
            status: "Starting".to_string(),
            digest: None,
            completed_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            speed_mb_per_sec: 0.0,
            eta_seconds: None,
            is_done: false,
            error: None,
        };

        let chunk = OllamaPullChunk {
            status: None,
            digest: None,
            total: None,
            completed: None,
            error: Some("model 'nonexistent:model' not found".to_string()),
        };

        let mut last_bytes = 0;
        let mut last_time = Instant::now();

        ModelPullerEngine::parse_pull_chunk(&chunk, &mut progress, &mut last_bytes, &mut last_time);

        assert_eq!(progress.status, "Failed");
        assert!(progress.is_done);
        assert_eq!(progress.error, Some("model 'nonexistent:model' not found".to_string()));
    }
}
