use anyhow::{anyhow, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

const SERVICE_NAME: &str = "locus-ai";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider_id: String,
    pub name: String,
    pub is_configured: bool,
    pub default_model: String,
    pub supports_custom_url: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTestResult {
    pub success: bool,
    pub provider_id: String,
    pub message: String,
    pub latency_ms: u64,
    pub available_models: Vec<String>,
}

pub struct KeyringStore;

impl KeyringStore {
    fn get_entry(provider: &str) -> Result<Entry> {
        let entry_name = format!("provider_{}", provider.to_lowercase().trim());
        Entry::new(SERVICE_NAME, &entry_name).map_err(|e| anyhow!("Failed to access OS keyring: {}", e))
    }

    pub fn save_key(provider: &str, api_key: &str) -> Result<()> {
        let entry = Self::get_entry(provider)?;
        entry.set_password(api_key.trim())
            .map_err(|e| anyhow!("Failed to save secret to OS keyring: {}", e))?;
        info!("Successfully saved API key for provider '{}' in OS keyring", provider);
        Ok(())
    }

    pub fn get_key(provider: &str) -> Result<Option<String>> {
        let entry = Self::get_entry(provider)?;
        match entry.get_password() {
            Ok(pwd) if !pwd.trim().is_empty() => Ok(Some(pwd.trim().to_string())),
            Ok(_) => Ok(None),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => {
                debug!("Keyring entry lookup for '{}': {}", provider, e);
                Ok(None)
            }
        }
    }

    pub fn delete_key(provider: &str) -> Result<()> {
        let entry = Self::get_entry(provider)?;
        match entry.delete_credential() {
            Ok(()) => {
                info!("Deleted API key for provider '{}' from OS keyring", provider);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow!("Failed to delete key from OS keyring: {}", e)),
        }
    }

    pub fn has_key(provider: &str) -> bool {
        Self::get_key(provider).unwrap_or(None).is_some()
    }

    pub fn list_configured_providers() -> Vec<ProviderStatus> {
        let known = vec![
            ("openai", "OpenAI (GPT-4o / o1)", "gpt-4o", false),
            ("anthropic", "Anthropic Claude (3.5 Sonnet)", "claude-3-5-sonnet-20241022", false),
            ("gemini", "Google Gemini (1.5 Pro / Flash)", "gemini-1.5-pro", false),
            ("deepseek", "DeepSeek (Coder / Chat)", "deepseek-coder", false),
            ("groq", "Groq (Llama 3 / Mixtral)", "llama-3.3-70b-versatile", false),
            ("mistral", "Mistral AI (Large / Codestral)", "codestral-latest", false),
            ("custom", "Custom OpenAI-Compatible", "custom-model", true),
        ];

        known
            .into_iter()
            .map(|(id, name, def_model, custom_url)| ProviderStatus {
                provider_id: id.to_string(),
                name: name.to_string(),
                is_configured: Self::has_key(id),
                default_model: def_model.to_string(),
                supports_custom_url: custom_url,
            })
            .collect()
    }

    pub async fn test_provider(
        provider: &str,
        key: Option<&str>,
        custom_base_url: Option<&str>,
    ) -> ProviderTestResult {
        let start = std::time::Instant::now();
        let api_key = match key {
            Some(k) if !k.trim().is_empty() => k.trim().to_string(),
            _ => match Self::get_key(provider) {
                Ok(Some(k)) => k,
                _ => {
                    return ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: "No API key found in OS keyring or input".to_string(),
                        latency_ms: 0,
                        available_models: vec![],
                    }
                }
            },
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        match provider.to_lowercase().as_str() {
            "openai" => {
                let url = custom_base_url.unwrap_or("https://api.openai.com/v1/models");
                match client.get(url).header("Authorization", format!("Bearer {}", api_key)).send().await {
                    Ok(res) if res.status().is_success() => {
                        let json: serde_json::Value = res.json().await.unwrap_or_default();
                        let models: Vec<String> = json["data"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                                    .filter(|id| id.starts_with("gpt") || id.starts_with("o1") || id.starts_with("o3"))
                                    .take(8)
                                    .collect()
                            })
                            .unwrap_or_default();

                        ProviderTestResult {
                            success: true,
                            provider_id: provider.to_string(),
                            message: "Connected to OpenAI API successfully".to_string(),
                            latency_ms: start.elapsed().as_millis() as u64,
                            available_models: models,
                        }
                    }
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("OpenAI authentication failed: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Network connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "anthropic" => {
                let url = "https://api.anthropic.com/v1/messages";
                let payload = serde_json::json!({
                    "model": "claude-3-5-haiku-20241022",
                    "max_tokens": 1,
                    "messages": [{"role": "user", "content": "ping"}]
                });

                match client
                    .post(url)
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(res) if res.status().is_success() || res.status().as_u16() == 200 => {
                        ProviderTestResult {
                            success: true,
                            provider_id: provider.to_string(),
                            message: "Connected to Anthropic Claude API successfully".to_string(),
                            latency_ms: start.elapsed().as_millis() as u64,
                            available_models: vec![
                                "claude-3-5-sonnet-20241022".to_string(),
                                "claude-3-5-haiku-20241022".to_string(),
                                "claude-3-opus-20240229".to_string(),
                            ],
                        }
                    }
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Anthropic authentication failed: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Network connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "gemini" => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                    api_key
                );
                match client.get(&url).send().await {
                    Ok(res) if res.status().is_success() => {
                        let json: serde_json::Value = res.json().await.unwrap_or_default();
                        let models: Vec<String> = json["models"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m["name"].as_str().map(|s| s.replace("models/", "")))
                                    .filter(|id| id.starts_with("gemini"))
                                    .take(8)
                                    .collect()
                            })
                            .unwrap_or_default();

                        ProviderTestResult {
                            success: true,
                            provider_id: provider.to_string(),
                            message: "Connected to Google Gemini API successfully".to_string(),
                            latency_ms: start.elapsed().as_millis() as u64,
                            available_models: models,
                        }
                    }
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Gemini API key rejected: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Gemini connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "deepseek" => {
                let url = custom_base_url.unwrap_or("https://api.deepseek.com/v1/models");
                match client.get(url).header("Authorization", format!("Bearer {}", api_key)).send().await {
                    Ok(res) if res.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: provider.to_string(),
                        message: "Connected to DeepSeek API successfully".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![
                            "deepseek-chat".to_string(),
                            "deepseek-coder".to_string(),
                            "deepseek-reasoner".to_string(),
                        ],
                    },
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("DeepSeek key rejected: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("DeepSeek connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "groq" => {
                let url = "https://api.groq.com/openai/v1/models";
                match client.get(url).header("Authorization", format!("Bearer {}", api_key)).send().await {
                    Ok(res) if res.status().is_success() => {
                        let json: serde_json::Value = res.json().await.unwrap_or_default();
                        let models: Vec<String> = json["data"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                                    .take(8)
                                    .collect()
                            })
                            .unwrap_or_default();

                        ProviderTestResult {
                            success: true,
                            provider_id: provider.to_string(),
                            message: "Connected to Groq Ultra-Fast API successfully".to_string(),
                            latency_ms: start.elapsed().as_millis() as u64,
                            available_models: models,
                        }
                    }
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Groq key rejected: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Groq connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "mistral" => {
                let url = "https://api.mistral.ai/v1/models";
                match client.get(url).header("Authorization", format!("Bearer {}", api_key)).send().await {
                    Ok(res) if res.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: provider.to_string(),
                        message: "Connected to Mistral AI API successfully".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![
                            "codestral-latest".to_string(),
                            "mistral-large-latest".to_string(),
                            "mistral-small-latest".to_string(),
                        ],
                    },
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Mistral key rejected: HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Mistral connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }

            "custom" | _ => {
                let default_url = "http://localhost:8000/v1/models";
                let url = custom_base_url.unwrap_or(default_url);
                match client.get(url).header("Authorization", format!("Bearer {}", api_key)).send().await {
                    Ok(res) if res.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: provider.to_string(),
                        message: format!("Connected to Custom endpoint ({}) successfully", url),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec!["custom-model".to_string()],
                    },
                    Ok(res) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Custom endpoint returned HTTP {}", res.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: provider.to_string(),
                        message: format!("Custom endpoint connection error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
        }
    }
}
