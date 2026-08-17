//! Hardware Probing & Local Inference Discovery
//!
//! Measures system RAM, VRAM, and CPU capabilities to recommend the optimal on-device coding model
//! (e.g. Qwen2.5-Coder:7B for >=16GB RAM, Qwen2.5-Coder:3B for <16GB RAM) and auto-detects
//! running inference servers (Ollama, LM Studio, vLLM).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub total_ram_gb: f32,
    pub available_ram_gb: f32,
    pub cpu_cores: usize,
    pub os: String,
    pub arch: String,
    pub has_gpu: bool,
    pub gpu_name: Option<String>,
    pub vram_gb: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendedModelSpec {
    pub model_id: String,
    pub display_name: String,
    pub parameter_size: String,
    pub download_size_gb: f32,
    pub min_ram_gb: f32,
    pub recommended_ram_gb: f32,
    pub tier: String,
    pub rationale: String,
    pub is_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalInferenceEndpoint {
    pub name: String,
    pub url: String,
    pub is_reachable: bool,
    pub version: Option<String>,
    pub models_count: usize,
    pub installed_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalDiscoveryReport {
    pub hardware: HardwareProfile,
    pub recommendation: RecommendedModelSpec,
    pub endpoints: Vec<LocalInferenceEndpoint>,
}

pub struct LocalDiscoveryManager;

impl LocalDiscoveryManager {
    /// Probes system memory, CPU cores, OS, and GPU characteristics.
    pub fn probe_hardware() -> HardwareProfile {
        let mut sys = System::new();
        sys.refresh_memory();

        let total_bytes = sys.total_memory();
        let avail_bytes = sys.available_memory();

        let total_ram_gb = (total_bytes as f32) / (1024.0 * 1024.0 * 1024.0);
        let available_ram_gb = (avail_bytes as f32) / (1024.0 * 1024.0 * 1024.0);

        let cpu_cores = num_cpus::get();
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        let (has_gpu, gpu_name, vram_gb) = Self::probe_gpu();

        HardwareProfile {
            total_ram_gb: (total_ram_gb * 10.0).round() / 10.0,
            available_ram_gb: (available_ram_gb * 10.0).round() / 10.0,
            cpu_cores,
            os,
            arch,
            has_gpu,
            gpu_name,
            vram_gb,
        }
    }

    /// Determines the optimal coding model based on total RAM and GPU VRAM.
    pub fn determine_recommendation(
        hardware: &HardwareProfile,
        installed_models: &[String],
    ) -> RecommendedModelSpec {
        let is_installed = |id: &str| {
            installed_models.iter().any(|m| {
                m.eq_ignore_ascii_case(id)
                    || m.starts_with(id)
                    || m.contains(&format!("qwen2.5-coder:{}", id.split(':').last().unwrap_or("")))
            })
        };

        // Decision Tree:
        // 1. Heavy / High-End: >= 32GB RAM or >= 16GB VRAM -> qwen2.5-coder:14b
        // 2. Standard / Recommended: >= 16GB RAM or >= 6GB VRAM -> qwen2.5-coder:7b
        // 3. Lightweight / Ultra-Fast: < 16GB RAM (e.g. 8GB) -> qwen2.5-coder:3b
        // 4. Extreme Low Memory: < 6GB RAM -> qwen2.5-coder:1.5b
        if hardware.total_ram_gb >= 32.0 || hardware.vram_gb.unwrap_or(0.0) >= 16.0 {
            let id = "qwen2.5-coder:14b".to_string();
            let installed = is_installed(&id);
            RecommendedModelSpec {
                model_id: id,
                display_name: "Qwen 2.5 Coder 14B (Frontier Local)".to_string(),
                parameter_size: "14B".to_string(),
                download_size_gb: 9.0,
                min_ram_gb: 20.0,
                recommended_ram_gb: 32.0,
                tier: "High Accuracy / Heavy".to_string(),
                rationale: format!(
                    "System has {:.1}GB RAM. Sufficient for running high-parameter 14B coding models at full context length.",
                    hardware.total_ram_gb
                ),
                is_installed: installed,
            }
        } else if hardware.total_ram_gb >= 15.0 || hardware.vram_gb.unwrap_or(0.0) >= 6.0 {
            let id = "qwen2.5-coder:7b".to_string();
            let installed = is_installed(&id);
            RecommendedModelSpec {
                model_id: id,
                display_name: "Qwen 2.5 Coder 7B (Standard Recommended)".to_string(),
                parameter_size: "7B".to_string(),
                download_size_gb: 4.7,
                min_ram_gb: 12.0,
                recommended_ram_gb: 16.0,
                tier: "Balanced Standard".to_string(),
                rationale: format!(
                    "System has {:.1}GB RAM. 7B provides state-of-the-art code generation, refactoring, and AST analysis.",
                    hardware.total_ram_gb
                ),
                is_installed: installed,
            }
        } else if hardware.total_ram_gb >= 6.0 {
            let id = "qwen2.5-coder:3b".to_string();
            let installed = is_installed(&id);
            RecommendedModelSpec {
                model_id: id,
                display_name: "Qwen 2.5 Coder 3B (Lightweight Fast)".to_string(),
                parameter_size: "3B".to_string(),
                download_size_gb: 2.0,
                min_ram_gb: 6.0,
                recommended_ram_gb: 8.0,
                tier: "Lightweight Fast".to_string(),
                rationale: format!(
                    "System has {:.1}GB RAM. 3B delivers ultra-fast code assistance with a tiny 2.0GB memory footprint.",
                    hardware.total_ram_gb
                ),
                is_installed: installed,
            }
        } else {
            let id = "qwen2.5-coder:1.5b".to_string();
            let installed = is_installed(&id);
            RecommendedModelSpec {
                model_id: id,
                display_name: "Qwen 2.5 Coder 1.5B (Ultra-Compact)".to_string(),
                parameter_size: "1.5B".to_string(),
                download_size_gb: 1.0,
                min_ram_gb: 4.0,
                recommended_ram_gb: 4.0,
                tier: "Ultra Compact".to_string(),
                rationale: "Compact 1.5B model for ultra-low memory environments.".to_string(),
                is_installed: installed,
            }
        }
    }

    /// Scans standard local inference ports (Ollama 11434, LM Studio 1234, vLLM/LocalAI 8000).
    pub async fn scan_endpoints() -> Vec<LocalInferenceEndpoint> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(600))
            .build()
            .unwrap_or_default();

        let mut results = Vec::new();

        // 1. Ollama on port 11434
        results.push(Self::probe_ollama(&client, "http://localhost:11434").await);

        // 2. LM Studio on port 1234
        results.push(Self::probe_openai_compatible(&client, "LM Studio", "http://localhost:1234").await);

        // 3. vLLM / LocalAI on port 8000
        results.push(Self::probe_openai_compatible(&client, "vLLM / LocalAI", "http://localhost:8000").await);

        results
    }

    /// Full discovery report combining hardware, endpoints, and recommended model.
    pub async fn generate_report() -> LocalDiscoveryReport {
        let hardware = Self::probe_hardware();
        let endpoints = Self::scan_endpoints().await;

        let mut all_installed = Vec::new();
        for ep in &endpoints {
            all_installed.extend(ep.installed_models.clone());
        }

        let recommendation = Self::determine_recommendation(&hardware, &all_installed);

        LocalDiscoveryReport {
            hardware,
            recommendation,
            endpoints,
        }
    }

    // --- Internal Endpoint Probers ---

    async fn probe_ollama(client: &reqwest::Client, base_url: &str) -> LocalInferenceEndpoint {
        let version_url = format!("{}/api/version", base_url);
        let tags_url = format!("{}/api/tags", base_url);

        let mut version = None;
        let mut installed_models = Vec::new();
        let is_reachable;

        if let Ok(res) = client.get(&version_url).send().await {
            if res.status().is_success() {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    version = json["version"].as_str().map(|s| s.to_string());
                }
            }
        }

        if let Ok(res) = client.get(&tags_url).send().await {
            if res.status().is_success() {
                is_reachable = true;
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(models) = json["models"].as_array() {
                        for m in models {
                            if let Some(name) = m["name"].as_str() {
                                installed_models.push(name.to_string());
                            }
                        }
                    }
                }
            } else {
                is_reachable = false;
            }
        } else {
            is_reachable = false;
        }

        LocalInferenceEndpoint {
            name: "Ollama".to_string(),
            url: base_url.to_string(),
            is_reachable,
            version,
            models_count: installed_models.len(),
            installed_models,
        }
    }

    async fn probe_openai_compatible(
        client: &reqwest::Client,
        name: &str,
        base_url: &str,
    ) -> LocalInferenceEndpoint {
        let models_url = format!("{}/v1/models", base_url);
        let mut installed_models = Vec::new();
        let mut is_reachable = false;

        if let Ok(res) = client.get(&models_url).send().await {
            if res.status().is_success() {
                is_reachable = true;
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        for item in data {
                            if let Some(id) = item["id"].as_str() {
                                installed_models.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }

        LocalInferenceEndpoint {
            name: name.to_string(),
            url: base_url.to_string(),
            is_reachable,
            version: None,
            models_count: installed_models.len(),
            installed_models,
        }
    }

    fn probe_gpu() -> (bool, Option<String>, Option<f32>) {
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = text.lines().next() {
                        let parts: Vec<&str> = first_line.split(',').collect();
                        if parts.len() >= 2 {
                            let name = parts[0].trim().to_string();
                            let vram_mb = parts[1].trim().parse::<f32>().unwrap_or(0.0);
                            return (true, Some(name), Some((vram_mb / 1024.0 * 10.0).round() / 10.0));
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = text.lines().next() {
                        let parts: Vec<&str> = first_line.split(',').collect();
                        if parts.len() >= 2 {
                            let name = parts[0].trim().to_string();
                            let vram_mb = parts[1].trim().parse::<f32>().unwrap_or(0.0);
                            return (true, Some(name), Some((vram_mb / 1024.0 * 10.0).round() / 10.0));
                        }
                    }
                }
            }
        }

        (false, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_probing_returns_valid_metrics() {
        let hardware = LocalDiscoveryManager::probe_hardware();
        assert!(hardware.total_ram_gb > 0.0);
        assert!(hardware.cpu_cores > 0);
        assert!(!hardware.os.is_empty());
    }

    #[test]
    fn test_recommendation_high_ram_assigns_14b() {
        let hardware = HardwareProfile {
            total_ram_gb: 32.0,
            available_ram_gb: 24.0,
            cpu_cores: 16,
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            has_gpu: true,
            gpu_name: Some("RTX 4090".to_string()),
            vram_gb: Some(24.0),
        };

        let spec = LocalDiscoveryManager::determine_recommendation(&hardware, &[]);
        assert_eq!(spec.model_id, "qwen2.5-coder:14b");
        assert_eq!(spec.parameter_size, "14B");
        assert!(!spec.is_installed);
    }

    #[test]
    fn test_recommendation_16gb_ram_assigns_7b() {
        let hardware = HardwareProfile {
            total_ram_gb: 16.0,
            available_ram_gb: 10.0,
            cpu_cores: 8,
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            has_gpu: false,
            gpu_name: None,
            vram_gb: None,
        };

        let spec = LocalDiscoveryManager::determine_recommendation(
            &hardware,
            &["qwen2.5-coder:7b".to_string()],
        );
        assert_eq!(spec.model_id, "qwen2.5-coder:7b");
        assert_eq!(spec.parameter_size, "7B");
        assert!(spec.is_installed);
    }

    #[test]
    fn test_recommendation_8gb_ram_assigns_3b() {
        let hardware = HardwareProfile {
            total_ram_gb: 8.0,
            available_ram_gb: 4.0,
            cpu_cores: 4,
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            has_gpu: false,
            gpu_name: None,
            vram_gb: None,
        };

        let spec = LocalDiscoveryManager::determine_recommendation(&hardware, &[]);
        assert_eq!(spec.model_id, "qwen2.5-coder:3b");
        assert_eq!(spec.parameter_size, "3B");
    }
}
