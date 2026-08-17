use crate::types::{BackendType, GpuInfo, LocalModel, ModelSelection};
use locus_network::types::{DeviceCapabilities, Specialization};
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub struct ModelSelector {
    system_info: SystemInfoCache,
}

#[derive(Debug, Clone)]
struct SystemInfoCache {
    cpu_cores: u32,
    total_memory_gb: f32,
    available_memory_gb: f32,
    gpus: Vec<GpuInfo>,
    last_updated: std::time::Instant,
}

impl ModelSelector {
    pub fn new() -> Self {
        Self {
            system_info: SystemInfoCache {
                cpu_cores: num_cpus::get() as u32,
                total_memory_gb: Self::get_total_memory_gb(),
                available_memory_gb: Self::get_available_memory_gb(),
                gpus: Self::detect_gpus(),
                last_updated: std::time::Instant::now(),
            },
        }
    }

    pub async fn select_model(
        &self,
        available_models: &[LocalModel],
        task_type: Option<Specialization>,
        preferred_backend: Option<BackendType>,
    ) -> Result<ModelSelection> {
        let system_info = self.get_system_info().await;
        
        let filtered_models: Vec<_> = available_models
            .iter()
            .filter(|m| {
                if let Some(ref backend) = preferred_backend {
                    m.backend == *backend
                } else {
                    true
                }
            })
            .collect();

        if filtered_models.is_empty() {
            return Err(anyhow::anyhow!("No models available for the selected backend"));
        }

        let best_model = self.score_models(&filtered_models, &system_info, task_type)
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(model, _)| model)
            .ok_or_else(|| anyhow::anyhow!("No suitable model found"))?;

        let selection = self.create_selection(best_model, &system_info);
        Ok(selection)
    }

    fn score_models<'a>(
        &self,
        models: &[&'a LocalModel],
        system_info: &SystemInfoCache,
        task_type: Option<Specialization>,
    ) -> Vec<(&'a LocalModel, f32)> {
        models.iter().copied().map(|model| {
            let mut score = 1.0;
            
            let estimated_vram = Self::estimate_vram_usage(model);
            let estimated_ram = Self::estimate_ram_usage(model);
            
            let total_vram: f32 = system_info.gpus.iter().map(|g| g.vram_gb).sum();
            let fits_in_vram = estimated_vram <= (total_vram * 1024.0) as u64;
            
            if fits_in_vram {
                score *= 2.0;
            } else if total_vram > 0.0 {
                score *= 0.5;
            } else {
                score *= 0.1;
            }
            
            let fits_in_ram = estimated_ram <= (system_info.available_memory_gb * 1024.0) as u64;
            if !fits_in_ram {
                score *= 0.1;
            }
            
            let param_score = Self::score_parameter_size(model);
            score *= param_score;
            
            if let Some(ref task) = task_type {
                let task_score = Self::score_for_task(model, task);
                score *= task_score;
            }
            
            let quant_score = Self::score_quantization(model);
            score *= quant_score;
            
            (model, score)
        }).collect()
    }

    fn estimate_vram_usage(model: &LocalModel) -> u64 {
        let param_count = Self::parse_parameter_count(&model.details.parameter_size);
        let quantization = &model.details.quantization_level;
        
        let bytes_per_param = match quantization.to_lowercase().as_str() {
            s if s.contains("q4") => 0.5,
            s if s.contains("q5") => 0.625,
            s if s.contains("q6") => 0.75,
            s if s.contains("q8") => 1.0,
            s if s.contains("f16") => 2.0,
            s if s.contains("f32") => 4.0,
            _ => 0.5,
        };
        
        let vram_bytes = (param_count as f64 * 1e9 * bytes_per_param * 1.2) as u64;
        vram_bytes / (1024 * 1024)
    }

    fn estimate_ram_usage(model: &LocalModel) -> u64 {
        let vram = Self::estimate_vram_usage(model);
        let param_count = Self::parse_parameter_count(&model.details.parameter_size);
        
        let context_overhead = (4096 * 4096 * 2) as u64;
        let kv_cache = (param_count as f64 * 1e9 * 0.1) as u64;
        
        (vram + context_overhead + kv_cache) / (1024 * 1024)
    }

    fn parse_parameter_count(param_str: &str) -> f64 {
        let param_str = param_str.to_lowercase();
        if param_str.contains('b') {
            param_str.replace('b', "").parse().unwrap_or(7.0)
        } else if param_str.contains('m') {
            param_str.replace('m', "").parse::<f64>().unwrap_or(7000.0) / 1000.0
        } else {
            7.0
        }
    }

    fn score_parameter_size(model: &LocalModel) -> f32 {
        let params = Self::parse_parameter_count(&model.details.parameter_size);
        
        match params {
            p if p <= 1.5 => 0.8,
            p if p <= 3.0 => 0.9,
            p if p <= 7.0 => 1.0,
            p if p <= 14.0 => 1.1,
            p if p <= 32.0 => 1.0,
            p if p <= 70.0 => 0.9,
            _ => 0.7,
        }
    }

    fn score_quantization(model: &LocalModel) -> f32 {
        let quant = model.details.quantization_level.to_lowercase();
        if quant.contains("q4_k_m") || quant.contains("q4_k_s") {
            1.0
        } else if quant.contains("q5") {
            1.1
        } else if quant.contains("q6") {
            1.15
        } else if quant.contains("q8") {
            1.1
        } else if quant.contains("f16") {
            1.2
        } else {
            0.9
        }
    }

    fn score_for_task(model: &LocalModel, task: &Specialization) -> f32 {
        let params = Self::parse_parameter_count(&model.details.parameter_size);
        
        match task {
            Specialization::CodeGeneration => {
                if params >= 7.0 { 1.2 } else { 0.8 }
            }
            Specialization::CodeReview => {
                if params >= 14.0 { 1.2 } else { 0.9 }
            }
            Specialization::Testing => { 1.0 }
            Specialization::Linting => { 1.0 }
            Specialization::Embeddings => { 0.9 }
            Specialization::Documentation => { 1.0 }
            Specialization::SecurityAnalysis => {
                if params >= 14.0 { 1.1 } else { 0.9 }
            }
        }
    }

    fn create_selection(&self, model: &LocalModel, system_info: &SystemInfoCache) -> ModelSelection {
        let estimated_vram = Self::estimate_vram_usage(model);
        let estimated_ram = Self::estimate_ram_usage(model);
        let total_vram: f32 = system_info.gpus.iter().map(|g| g.vram_gb).sum();
        let fits_in_vram = estimated_vram <= (total_vram * 1024.0) as u64;
        
        let reasoning = if fits_in_vram {
            format!(
                "Model {} fits in VRAM ({}MB / {}GB). Quantization: {}. Good for local inference.",
                model.name, estimated_vram, total_vram, model.details.quantization_level
            )
        } else if total_vram > 0.0 {
            format!(
                "Model {} exceeds VRAM ({}MB / {}GB). Will use CPU offloading. Quantization: {}.",
                model.name, estimated_vram, total_vram, model.details.quantization_level
            )
        } else {
            format!(
                "No GPU detected. Model {} will run on CPU ({}MB RAM). Quantization: {}.",
                model.name, estimated_ram, model.details.quantization_level
            )
        };
        
        ModelSelection {
            model_name: model.name.clone(),
            backend: model.backend.clone(),
            estimated_vram_mb: estimated_vram,
            estimated_ram_mb: estimated_ram,
            fits_in_vram,
            quantization: model.details.quantization_level.clone(),
            reasoning,
        }
    }

    async fn get_system_info(&self) -> SystemInfoCache {
        if self.system_info.last_updated.elapsed().as_secs() > 60 {
            SystemInfoCache {
                cpu_cores: num_cpus::get() as u32,
                total_memory_gb: Self::get_total_memory_gb(),
                available_memory_gb: Self::get_available_memory_gb(),
                gpus: Self::detect_gpus(),
                last_updated: std::time::Instant::now(),
            }
        } else {
            self.system_info.clone()
        }
    }

    fn get_total_memory_gb() -> f32 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<f32>() {
                                return kb / (1024.0 * 1024.0);
                            }
                        }
                    }
                }
            }
        }
        16.0
    }

    fn get_available_memory_gb() -> f32 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                let mut total = 0.0;
                let mut available = 0.0;
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            total = parts[1].parse::<f32>().unwrap_or(0.0);
                        }
                    } else if line.starts_with("MemAvailable:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            available = parts[1].parse::<f32>().unwrap_or(0.0);
                        }
                    }
                }
                if available > 0.0 {
                    return available / (1024.0 * 1024.0);
                } else if total > 0.0 {
                    return total * 0.7 / (1024.0 * 1024.0);
                }
            }
        }
        8.0
    }

    fn detect_gpus() -> Vec<GpuInfo> {
        let gpus = Vec::new();
        
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total,driver_version", "--format=csv,noheader,nounits"])
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 3 {
                            let name = parts[0].trim().to_string();
                            let vram_mb = parts[1].trim().parse::<f32>().unwrap_or(0.0);
                            let driver = parts[2].trim().to_string();
                            
                            gpus.push(GpuInfo {
                                name: name.clone(),
                                vram_gb: vram_mb / 1024.0,
                                driver_version: Some(driver),
                                compute_capability: Self::get_compute_capability(&name),
                            });
                        }
                    }
                }
            }
        }
        
        if gpus.is_empty() {
            #[cfg(target_os = "macos")]
            {
                if let Ok(output) = std::process::Command::new("system_profiler")
                    .args(["SPDisplaysDataType", "-json"])
                    .output()
                {
                    // Parse macOS GPU info
                }
            }
        }
        
        gpus
    }

    fn get_compute_capability(name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        if name_lower.contains("rtx 40") || name_lower.contains("rtx 30") {
            Some("8.9".to_string())
        } else if name_lower.contains("rtx 20") || name_lower.contains("gtx 16") {
            Some("7.5".to_string())
        } else if name_lower.contains("gtx 10") {
            Some("6.1".to_string())
        } else if name_lower.contains("a100") || name_lower.contains("h100") {
            Some("8.0".to_string())
        } else {
            None
        }
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LocalModel, ModelDetails, BackendType};

    fn sample_model(name: &str, params: &str, quant: &str) -> LocalModel {
        LocalModel {
            name: name.to_string(),
            size: "4GB".to_string(),
            digest: "abc123".to_string(),
            details: ModelDetails {
                format: "gguf".to_string(),
                family: "llama".to_string(),
                families: None,
                parameter_size: params.to_string(),
                quantization_level: quant.to_string(),
                parent_model: None,
            },
            modified_at: chrono::Utc::now(),
            backend: BackendType::Ollama,
        }
    }

    #[test]
    fn test_vram_estimation() {
        let model = sample_model("test", "7B", "q4_k_m");
        let vram = ModelSelector::estimate_vram_usage(&model);
        assert!(vram > 0);
        assert!(vram < 10000);
    }

    #[test]
    fn test_parameter_parsing() {
        assert_eq!(ModelSelector::parse_parameter_count("7B"), 7.0);
        assert_eq!(ModelSelector::parse_parameter_count("14B"), 14.0);
        assert_eq!(ModelSelector::parse_parameter_count("70B"), 70.0);
    }

    #[tokio::test]
    async fn test_model_selection() {
        let selector = ModelSelector::new();
        let models = vec![
            sample_model("llama3:7b", "7B", "q4_k_m"),
            sample_model("llama3:14b", "14B", "q4_k_m"),
            sample_model("codellama:7b", "7B", "q4_k_m"),
        ];
        
        let selection = selector.select_model(&models, Some(Specialization::CodeGeneration), None).await.unwrap();
        assert!(!selection.model_name.is_empty());
    }
}

