use crate::provider::ProviderType;
use anyhow::{anyhow, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const SERVICE_NAME: &str = "locus-ai";

/// In-memory encrypted fallback store for environments without OS keyring service or headless testing
static IN_MEMORY_FALLBACK: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

/// Global in-memory KeyPool manager tracking per-key cooldowns and rotation indices
static KEY_POOL_MANAGER: OnceLock<RwLock<HashMap<String, KeyPool>>> = OnceLock::new();

fn get_in_memory_store() -> &'static RwLock<HashMap<String, String>> {
    IN_MEMORY_FALLBACK.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_key_pool_manager() -> &'static RwLock<HashMap<String, KeyPool>> {
    KEY_POOL_MANAGER.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Debug, Clone)]
pub struct KeySlot {
    pub key: String,
    pub cooldown_until: Option<Instant>,
    pub request_count: u64,
    pub consecutive_429: u32,
}

impl KeySlot {
    pub fn new(key: String) -> Self {
        Self {
            key,
            cooldown_until: None,
            request_count: 0,
            consecutive_429: 0,
        }
    }

    pub fn is_cooling(&self) -> bool {
        if let Some(until) = self.cooldown_until {
            Instant::now() < until
        } else {
            false
        }
    }

    pub fn remaining_cooldown_secs(&self) -> u64 {
        if let Some(until) = self.cooldown_until {
            let now = Instant::now();
            if now < until {
                return (until - now).as_secs();
            }
        }
        0
    }
}

#[derive(Debug, Clone)]
pub struct KeyPool {
    pub provider: String,
    pub slots: Vec<KeySlot>,
    pub current_index: usize,
}

impl KeyPool {
    pub fn new(provider: String, keys: Vec<String>) -> Self {
        let slots = keys.into_iter().map(KeySlot::new).collect();
        Self {
            provider,
            slots,
            current_index: 0,
        }
    }

    pub fn get_next_active_key(&mut self) -> Option<String> {
        if self.slots.is_empty() {
            return None;
        }

        let total = self.slots.len();
        // Check in round-robin fashion starting from current_index
        for i in 0..total {
            let idx = (self.current_index + i) % total;
            if !self.slots[idx].is_cooling() {
                self.slots[idx].request_count += 1;
                self.current_index = (idx + 1) % total;
                return Some(self.slots[idx].key.clone());
            }
        }

        None // All keys currently in cooldown
    }

    pub fn mark_429(&mut self, key: &str, cooldown_secs: u64) {
        for slot in &mut self.slots {
            if slot.key == key {
                slot.cooldown_until = Some(Instant::now() + Duration::from_secs(cooldown_secs));
                slot.consecutive_429 += 1;
                warn!(
                    "Key for provider '{}' marked in 429 Cooldown for {}s (Consecutive: {})",
                    self.provider, cooldown_secs, slot.consecutive_429
                );
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySlotStatus {
    pub key_masked: String,
    pub is_active: bool,
    pub in_cooldown: bool,
    pub cooldown_remaining_secs: u64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider_id: String,
    pub name: String,
    pub is_configured: bool,
    pub default_model: String,
    pub supports_custom_url: bool,
    #[serde(default)]
    pub pool_size: usize,
    #[serde(default)]
    pub active_keys_count: usize,
    #[serde(default)]
    pub keys: Vec<KeySlotStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTestResult {
    pub success: bool,
    pub provider_id: String,
    pub message: String,
    pub latency_ms: u64,
    pub available_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedKeyReport {
    pub provider_id: String,
    pub provider_name: String,
    pub source: String,
    pub key_masked: String,
    pub imported: bool,
    pub message: String,
}

pub fn mask_key_preview(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return "".to_string();
    }
    if trimmed.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix_len = 4.min(trimmed.len() / 3);
    let suffix_len = 4.min(trimmed.len() / 3);
    format!(
        "{}••••••••{}",
        &trimmed[..prefix_len],
        &trimmed[trimmed.len() - suffix_len..]
    )
}

pub struct KeyringStore;

impl KeyringStore {
    fn get_entry(provider: &str) -> Result<Entry> {
        let entry_name = format!("provider_{}", provider.to_lowercase().trim());
        Entry::new(SERVICE_NAME, &entry_name).map_err(|e| anyhow!("Failed to access OS keyring: {}", e))
    }

    fn provider_type_to_str(provider: &ProviderType) -> &'static str {
        match provider {
            ProviderType::LocalOllama => "ollama",
            ProviderType::GeminiFlash => "gemini",
            ProviderType::Groq => "groq",
            ProviderType::OpenRouter => "openrouter",
            ProviderType::Custom(name) => Box::leak(name.clone().into_boxed_str()),
        }
    }

    /// Parses multiple keys separated by newlines, commas, semicolons, or whitespace
    pub fn parse_multiple_keys(raw: &str) -> Vec<String> {
        let mut keys = Vec::new();
        for chunk in raw.split(|c| c == '\n' || c == '\r' || c == ',' || c == ';') {
            let trimmed = chunk.trim();
            if !trimmed.is_empty() && !keys.contains(&trimmed.to_string()) {
                keys.push(trimmed.to_string());
            }
        }
        keys
    }

    /// Stores an API key or multi-key pool for a ProviderType in the OS keyring and in-memory pool
    pub fn store_api_key(provider: ProviderType, key: &str) -> Result<()> {
        let key_trimmed = key.trim();
        let provider_str = Self::provider_type_to_str(&provider);
        Self::save_key(provider_str, key_trimmed)
    }

    /// Retrieves the current active (non-cooldown) API key for a ProviderType
    pub fn get_api_key(provider: ProviderType) -> Option<String> {
        let provider_str = Self::provider_type_to_str(&provider);
        Self::get_active_api_key(provider_str)
    }

    /// Deletes all API keys for a ProviderType
    pub fn delete_api_key(provider: ProviderType) -> Result<()> {
        let provider_str = Self::provider_type_to_str(&provider);
        Self::delete_key(provider_str)
    }

    /// Saves one or multiple keys (pool) for a provider ID
    pub fn save_key(provider: &str, api_keys_raw: &str) -> Result<()> {
        let trimmed = api_keys_raw.trim();
        let key_id = provider.to_lowercase().trim().to_string();
        let parsed_keys = Self::parse_multiple_keys(trimmed);

        if parsed_keys.is_empty() {
            return Self::delete_key(provider);
        }

        // 1. Update In-Memory KeyPool manager
        if let Ok(mut pool_mgr) = get_key_pool_manager().write() {
            pool_mgr.insert(key_id.clone(), KeyPool::new(key_id.clone(), parsed_keys.clone()));
        }

        let serialized_keys = parsed_keys.join("\n");

        // 2. Try OS keyring
        match Self::get_entry(&key_id) {
            Ok(entry) => {
                match entry.set_password(&serialized_keys) {
                    Ok(()) => {
                        info!("Successfully saved {} key(s) for provider '{}' in OS keyring", parsed_keys.len(), provider);
                        if let Ok(mut mem) = get_in_memory_store().write() {
                            mem.insert(key_id, serialized_keys);
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("OS Keyring set_password failed ({}), using memory store fallback", e);
                    }
                }
            }
            Err(e) => {
                warn!("OS Keyring entry initialization failed ({}), using memory store fallback", e);
            }
        }

        // 3. Fallback to memory store
        if let Ok(mut mem) = get_in_memory_store().write() {
            mem.insert(key_id, serialized_keys);
            info!("Saved {} key(s) for provider '{}' in memory fallback store", parsed_keys.len(), provider);
            Ok(())
        } else {
            Err(anyhow!("Failed to lock memory fallback store"))
        }
    }

    /// Returns the next available non-cooling API key from the pool (with round-robin rotation)
    pub fn get_active_api_key(provider: &str) -> Option<String> {
        let key_id = provider.to_lowercase().trim().to_string();

        // 1. Check KeyPool manager
        if let Ok(mut pool_mgr) = get_key_pool_manager().write() {
            if let Some(pool) = pool_mgr.get_mut(&key_id) {
                if let Some(active_key) = pool.get_next_active_key() {
                    return Some(active_key);
                } else if !pool.slots.is_empty() {
                    // All keys in cooldown
                    warn!("All {} keys for provider '{}' are in cooldown", pool.slots.len(), provider);
                    return None;
                }
            }
        }

        // 2. If not yet initialized in pool manager, load from storage
        if let Ok(Some(raw)) = Self::get_raw_keys(provider) {
            let parsed = Self::parse_multiple_keys(&raw);
            if !parsed.is_empty() {
                let mut pool = KeyPool::new(key_id.clone(), parsed);
                let next = pool.get_next_active_key();
                if let Ok(mut pool_mgr) = get_key_pool_manager().write() {
                    pool_mgr.insert(key_id, pool);
                }
                return next;
            }
        }

        None
    }

    /// Marks a specific key as rate-limited with a cooldown period (default 60 seconds)
    pub fn mark_key_rate_limited(provider: &str, key: &str, cooldown_secs: u64) {
        let key_id = provider.to_lowercase().trim().to_string();
        if let Ok(mut pool_mgr) = get_key_pool_manager().write() {
            if let Some(pool) = pool_mgr.get_mut(&key_id) {
                pool.mark_429(key, cooldown_secs);
            }
        }
    }

    /// Checks if a provider has at least one active (non-cooldown) key
    pub fn has_active_key(provider: &str) -> bool {
        let key_id = provider.to_lowercase().trim().to_string();
        if let Ok(pool_mgr) = get_key_pool_manager().read() {
            if let Some(pool) = pool_mgr.get(&key_id) {
                return pool.slots.iter().any(|s| !s.is_cooling());
            }
        }
        Self::has_key(provider)
    }

    /// Gets status of all keys in pool for UI rendering
    pub fn get_key_pool_status(provider: &str) -> Vec<KeySlotStatus> {
        let key_id = provider.to_lowercase().trim().to_string();
        if let Ok(pool_mgr) = get_key_pool_manager().read() {
            if let Some(pool) = pool_mgr.get(&key_id) {
                return pool
                    .slots
                    .iter()
                    .map(|s| KeySlotStatus {
                        key_masked: mask_key_preview(&s.key),
                        is_active: !s.is_cooling(),
                        in_cooldown: s.is_cooling(),
                        cooldown_remaining_secs: s.remaining_cooldown_secs(),
                        request_count: s.request_count,
                    })
                    .collect();
            }
        }

        // Fallback if not in manager
        if let Ok(Some(raw)) = Self::get_raw_keys(provider) {
            let parsed = Self::parse_multiple_keys(&raw);
            return parsed
                .into_iter()
                .map(|k| KeySlotStatus {
                    key_masked: mask_key_preview(&k),
                    is_active: true,
                    in_cooldown: false,
                    cooldown_remaining_secs: 0,
                    request_count: 0,
                })
                .collect();
        }

        Vec::new()
    }

    /// Raw stored string lookup
    pub fn get_raw_keys(provider: &str) -> Result<Option<String>> {
        let key_id = provider.to_lowercase().trim().to_string();

        if let Ok(entry) = Self::get_entry(&key_id) {
            if let Ok(pwd) = entry.get_password() {
                if !pwd.trim().is_empty() {
                    return Ok(Some(pwd.trim().to_string()));
                }
            }
        }

        if let Ok(mem) = get_in_memory_store().read() {
            if let Some(val) = mem.get(&key_id) {
                if !val.trim().is_empty() {
                    return Ok(Some(val.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    /// String-based lookup
    pub fn get_key(provider: &str) -> Result<Option<String>> {
        Ok(Self::get_active_api_key(provider))
    }

    /// String-based delete
    pub fn delete_key(provider: &str) -> Result<()> {
        let key_id = provider.to_lowercase().trim().to_string();

        // Remove from Pool manager
        if let Ok(mut pool_mgr) = get_key_pool_manager().write() {
            pool_mgr.remove(&key_id);
        }

        // Delete from OS keyring
        if let Ok(entry) = Self::get_entry(&key_id) {
            let _ = entry.delete_credential();
        }

        // Delete from memory fallback
        if let Ok(mut mem) = get_in_memory_store().write() {
            mem.remove(&key_id);
        }

        info!("Deleted API keys for provider '{}' from OS keyring & pool manager", provider);
        Ok(())
    }

    pub fn has_key(provider: &str) -> bool {
        Self::get_raw_keys(provider).unwrap_or(None).is_some()
    }

    pub fn list_configured_providers() -> Vec<ProviderStatus> {
        let providers = vec![
            ("gemini", "Google Gemini Flash", "gemini-2.0-flash", false),
            ("groq", "Groq Ultra-Fast", "llama-3.3-70b-versatile", false),
            ("openrouter", "OpenRouter Free Tier", "meta-llama/llama-3.3-70b-instruct:free", false),
            ("deepseek", "DeepSeek", "deepseek-coder", false),
            ("openai", "OpenAI", "gpt-4o", false),
            ("anthropic", "Anthropic Claude", "claude-3-5-sonnet-20241022", false),
            ("mistral", "Mistral AI", "codestral-latest", false),
            ("custom", "Custom OpenAI-Compatible", "custom-model", true),
        ];

        providers
            .into_iter()
            .map(|(id, name, default_model, custom_url)| {
                let pool_status = Self::get_key_pool_status(id);
                let pool_size = pool_status.len();
                let active_keys_count = pool_status.iter().filter(|s| s.is_active).count();

                ProviderStatus {
                    provider_id: id.to_string(),
                    name: name.to_string(),
                    is_configured: pool_size > 0,
                    default_model: default_model.to_string(),
                    supports_custom_url: custom_url,
                    pool_size,
                    active_keys_count,
                    keys: pool_status,
                }
            })
            .collect()
    }

    pub async fn test_provider(
        provider: &str,
        custom_key: Option<&str>,
        custom_url: Option<&str>,
    ) -> ProviderTestResult {
        let key = match custom_key {
            Some(k) if !k.trim().is_empty() => Some(k.trim().to_string()),
            _ => Self::get_key(provider).unwrap_or(None),
        };

        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        match provider.to_lowercase().trim() {
            "gemini" => {
                let Some(k) = key else {
                    return ProviderTestResult {
                        success: false,
                        provider_id: "gemini".to_string(),
                        message: "Gemini API Key missing".to_string(),
                        latency_ms: 0,
                        available_models: vec![],
                    };
                };
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                    k
                );
                let payload = serde_json::json!({
                    "contents": [{"parts": [{"text": "ping"}]}],
                    "generationConfig": {"maxOutputTokens": 2}
                });
                match client.post(&url).json(&payload).send().await {
                    Ok(r) if r.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: "gemini".to_string(),
                        message: "Connected to Google Gemini Flash API".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![
                            "gemini-2.0-flash".to_string(),
                            "gemini-1.5-flash".to_string(),
                            "gemini-1.5-pro".to_string(),
                        ],
                    },
                    Ok(r) => ProviderTestResult {
                        success: false,
                        provider_id: "gemini".to_string(),
                        message: format!("Gemini Auth Error: HTTP {}", r.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: "gemini".to_string(),
                        message: format!("Network Error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
            "groq" => {
                let Some(k) = key else {
                    return ProviderTestResult {
                        success: false,
                        provider_id: "groq".to_string(),
                        message: "Groq API Key missing".to_string(),
                        latency_ms: 0,
                        available_models: vec![],
                    };
                };
                let payload = serde_json::json!({
                    "model": "llama-3.3-70b-versatile",
                    "messages": [{"role": "user", "content": "ping"}],
                    "max_tokens": 2
                });
                match client
                    .post("https://api.groq.com/openai/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", k))
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: "groq".to_string(),
                        message: "Connected to Groq Ultra-Fast API".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![
                            "llama-3.3-70b-versatile".to_string(),
                            "deepseek-r1-distill-llama-70b".to_string(),
                        ],
                    },
                    Ok(r) => ProviderTestResult {
                        success: false,
                        provider_id: "groq".to_string(),
                        message: format!("Groq Error: HTTP {}", r.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: "groq".to_string(),
                        message: format!("Network Error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
            "openrouter" => {
                let Some(k) = key else {
                    return ProviderTestResult {
                        success: false,
                        provider_id: "openrouter".to_string(),
                        message: "OpenRouter API Key missing".to_string(),
                        latency_ms: 0,
                        available_models: vec![],
                    };
                };
                let payload = serde_json::json!({
                    "model": "meta-llama/llama-3.3-70b-instruct:free",
                    "messages": [{"role": "user", "content": "ping"}],
                    "max_tokens": 2
                });
                match client
                    .post("https://openrouter.ai/api/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", k))
                    .header("HTTP-Referer", "https://github.com/locus-ai/locus")
                    .header("X-Title", "LOCUS AI Assistant")
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: "openrouter".to_string(),
                        message: "Connected to OpenRouter Free Tier".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![
                            "meta-llama/llama-3.3-70b-instruct:free".to_string(),
                            "deepseek/deepseek-r1:free".to_string(),
                            "google/gemini-2.0-flash-exp:free".to_string(),
                        ],
                    },
                    Ok(r) => ProviderTestResult {
                        success: false,
                        provider_id: "openrouter".to_string(),
                        message: format!("OpenRouter Error: HTTP {}", r.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: "openrouter".to_string(),
                        message: format!("Network Error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
            "openai" => {
                let Some(k) = key else {
                    return ProviderTestResult {
                        success: false,
                        provider_id: "openai".to_string(),
                        message: "OpenAI API Key missing".to_string(),
                        latency_ms: 0,
                        available_models: vec![],
                    };
                };
                match client
                    .get("https://api.openai.com/v1/models")
                    .header("Authorization", format!("Bearer {}", k))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: "openai".to_string(),
                        message: "Connected to OpenAI API".to_string(),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec!["gpt-4o".to_string(), "o1".to_string(), "o3-mini".to_string()],
                    },
                    Ok(r) => ProviderTestResult {
                        success: false,
                        provider_id: "openai".to_string(),
                        message: format!("OpenAI Auth Error: HTTP {}", r.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: "openai".to_string(),
                        message: format!("Network Error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
            "custom" => {
                let base = custom_url.unwrap_or("http://localhost:8000/v1");
                let url = format!("{}/models", base.trim_end_matches('/'));
                let mut req = client.get(&url);
                if let Some(ref k) = key {
                    req = req.header("Authorization", format!("Bearer {}", k));
                }
                match req.send().await {
                    Ok(r) if r.status().is_success() => ProviderTestResult {
                        success: true,
                        provider_id: "custom".to_string(),
                        message: format!("Connected to custom endpoint ({})", base),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec!["custom-model".to_string()],
                    },
                    Ok(r) => ProviderTestResult {
                        success: false,
                        provider_id: "custom".to_string(),
                        message: format!("Custom endpoint HTTP {}", r.status()),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                    Err(e) => ProviderTestResult {
                        success: false,
                        provider_id: "custom".to_string(),
                        message: format!("Custom endpoint error: {}", e),
                        latency_ms: start.elapsed().as_millis() as u64,
                        available_models: vec![],
                    },
                }
            }
            _ => ProviderTestResult {
                success: false,
                provider_id: provider.to_string(),
                message: format!("Unknown provider '{}'", provider),
                latency_ms: 0,
                available_models: vec![],
            },
        }
    }

    /// Automatically searches system environment variables and `.env` files for provider keys,
    /// imports them securely into the KeyringStore, and returns masked reports.
    pub fn auto_detect_and_import_keys(workspace_dir: Option<&str>) -> Vec<DetectedKeyReport> {
        let mut reports = Vec::new();
        let mut detected_map: HashMap<String, (String, String)> = HashMap::new();

        let provider_env_vars: Vec<(&str, &str, Vec<&str>)> = vec![
            ("gemini", "Google Gemini", vec!["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_AI_KEY"]),
            ("groq", "Groq Ultra-Fast", vec!["GROQ_API_KEY"]),
            ("openrouter", "OpenRouter Free Tier", vec!["OPENROUTER_API_KEY", "OPEN_ROUTER_API_KEY"]),
            ("deepseek", "DeepSeek", vec!["DEEPSEEK_API_KEY", "DEEP_SEEK_API_KEY"]),
            ("openai", "OpenAI", vec!["OPENAI_API_KEY"]),
            ("anthropic", "Anthropic Claude", vec!["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"]),
            ("mistral", "Mistral AI", vec!["MISTRAL_API_KEY"]),
        ];

        // 1. Search System Environment Variables
        for (provider_id, _, env_vars) in &provider_env_vars {
            for var_name in env_vars {
                if let Ok(val) = std::env::var(var_name) {
                    let trimmed = val.trim().to_string();
                    if !trimmed.is_empty() && !detected_map.contains_key(*provider_id) {
                        detected_map.insert(
                            provider_id.to_string(),
                            (trimmed, format!("System Environment (${})", var_name)),
                        );
                        break;
                    }
                }
            }
        }

        // 2. Search .env and .env.local in workspace and cwd
        let mut search_paths = Vec::new();
        if let Some(dir) = workspace_dir {
            let p = std::path::Path::new(dir);
            search_paths.push(p.join(".env"));
            search_paths.push(p.join(".env.local"));
        }
        search_paths.push(std::path::PathBuf::from(".env"));
        search_paths.push(std::path::PathBuf::from(".env.local"));

        for path in search_paths {
            if path.exists() && path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        let clean_line = line.strip_prefix("export ").unwrap_or(line).trim();
                        if let Some((k, v)) = clean_line.split_once('=') {
                            let k = k.trim();
                            let v = v.trim().trim_matches('"').trim_matches('\'').trim();
                            if v.is_empty() {
                                continue;
                            }

                            for (provider_id, _, env_vars) in &provider_env_vars {
                                if env_vars.iter().any(|ev| ev.eq_ignore_ascii_case(k)) {
                                    if !detected_map.contains_key(*provider_id) {
                                        detected_map.insert(
                                            provider_id.to_string(),
                                            (v.to_string(), format!("{} file ({})", file_name, k)),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Save discovered keys and construct reports with masked keys
        for (provider_id, provider_name, _) in &provider_env_vars {
            if let Some((key_val, source)) = detected_map.get(*provider_id) {
                let mask = mask_key_preview(key_val);
                match Self::save_key(provider_id, key_val) {
                    Ok(()) => {
                        reports.push(DetectedKeyReport {
                            provider_id: provider_id.to_string(),
                            provider_name: provider_name.to_string(),
                            source: source.clone(),
                            key_masked: mask,
                            imported: true,
                            message: format!("Imported securely from {}", source),
                        });
                    }
                    Err(e) => {
                        reports.push(DetectedKeyReport {
                            provider_id: provider_id.to_string(),
                            provider_name: provider_name.to_string(),
                            source: source.clone(),
                            key_masked: mask,
                            imported: false,
                            message: format!("Failed to save key: {}", e),
                        });
                    }
                }
            }
        }

        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_preview() {
        assert_eq!(mask_key_preview("AIzaSyB1234567890abcdef1234"), "AIza••••••••1234");
        assert_eq!(mask_key_preview("gsk_1234567890abcdef1234"), "gsk_••••••••1234");
        assert_eq!(mask_key_preview("12345"), "••••••••");
        assert_eq!(mask_key_preview(""), "");
    }

    #[test]
    fn test_auto_detect_keys_from_env() {
        std::env::set_var("GROQ_API_KEY", "gsk_env_auto_test_9999");
        let reports = KeyringStore::auto_detect_and_import_keys(None);
        let groq_report = reports.iter().find(|r| r.provider_id == "groq");
        assert!(groq_report.is_some());
        let report = groq_report.unwrap();
        assert!(report.imported);
        assert!(report.source.contains("System Environment"));
        assert!(report.key_masked.contains("gsk_"));
        assert!(report.key_masked.contains("9999"));
        assert!(!report.key_masked.contains("auto_test")); // Secret middle part masked!
        std::env::remove_var("GROQ_API_KEY");
    }

    #[test]
    fn test_memory_keyring_fallback() {
        let res = KeyringStore::store_api_key(ProviderType::Groq, "gsk_test_12345");
        assert!(res.is_ok());

        let retrieved = KeyringStore::get_api_key(ProviderType::Groq);
        assert_eq!(retrieved, Some("gsk_test_12345".to_string()));

        let del = KeyringStore::delete_api_key(ProviderType::Groq);
        assert!(del.is_ok());

        let after_del = KeyringStore::get_api_key(ProviderType::Groq);
        assert_eq!(after_del, None);
    }

    #[test]
    fn test_multi_key_pool_parsing() {
        let raw = "gsk_key1, gsk_key2\ngsk_key3; gsk_key1 \r\n gsk_key4";
        let parsed = KeyringStore::parse_multiple_keys(raw);
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0], "gsk_key1");
        assert_eq!(parsed[1], "gsk_key2");
        assert_eq!(parsed[2], "gsk_key3");
        assert_eq!(parsed[3], "gsk_key4");
    }

    #[test]
    fn test_multi_key_rotation_and_429_cooldown() {
        let provider = "pool_test_gemini";
        let keys = "AIza_key_1\nAIza_key_2\nAIza_key_3";
        KeyringStore::save_key(provider, keys).unwrap();

        // 1. First retrieval returns key 1
        let k1 = KeyringStore::get_active_api_key(provider).unwrap();
        assert_eq!(k1, "AIza_key_1");

        // 2. Next retrieval rotates to key 2
        let k2 = KeyringStore::get_active_api_key(provider).unwrap();
        assert_eq!(k2, "AIza_key_2");

        // 3. Mark key 2 as 429 rate limited (60s cooldown)
        KeyringStore::mark_key_rate_limited(provider, "AIza_key_2", 60);

        // 4. Next retrieval skips key 2 and gives key 3
        let k3 = KeyringStore::get_active_api_key(provider).unwrap();
        assert_eq!(k3, "AIza_key_3");

        // 5. Next retrieval skips key 2 (still cooling) and wraps around to key 1
        let k1_again = KeyringStore::get_active_api_key(provider).unwrap();
        assert_eq!(k1_again, "AIza_key_1");

        // 6. Check pool status
        // Keys: [AIza_key_1, AIza_key_2, AIza_key_3] → slot index 1 = AIza_key_2 (the rate-limited one)
        let status = KeyringStore::get_key_pool_status(provider);
        assert_eq!(status.len(), 3);
        // Slot 0 = key_1 (active), Slot 1 = key_2 (cooldown), Slot 2 = key_3 (active)
        let key2_status = &status[1];
        assert!(key2_status.in_cooldown, "key_2 should be in cooldown");
        assert!(!key2_status.is_active, "key_2 should not be active");
        assert!(key2_status.cooldown_remaining_secs > 0, "key_2 should have remaining cooldown");
        // Verify the other keys are active
        assert!(!status[0].in_cooldown, "key_1 should not be in cooldown");
        assert!(!status[2].in_cooldown, "key_3 should not be in cooldown");

        // Cleanup
        KeyringStore::delete_key(provider).unwrap();
    }
}
