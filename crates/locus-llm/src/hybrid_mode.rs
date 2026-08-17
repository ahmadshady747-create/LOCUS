use crate::types::{LocalModel, Message, MessageRole};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    pub enabled: bool,
    pub cloud_provider: Option<CloudProvider>,
    pub api_key: Option<String>,
    pub api_base_url: Option<String>,
    pub privacy_mode: PrivacyMode,
    pub sensitive_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CloudProvider {
    OpenAI,
    Anthropic,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyMode {
    FullLocal,
    StructureOnly,
    MaskedData,
    FullCloud,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cloud_provider: None,
            api_key: None,
            api_base_url: None,
            privacy_mode: PrivacyMode::FullLocal,
            sensitive_patterns: vec![
                r"(?i)(api[_-]?key|secret|password|token)\s*[:=]\s*\S+".to_string(),
                r"(?i)(private[_-]?key|ssh[_-]?key)\s*[:=]\s*\S+".to_string(),
                r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b".to_string(),
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridRequest {
    pub messages: Vec<Message>,
    pub local_model: Option<String>,
    pub cloud_model: Option<String>,
    pub config: HybridConfig,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResponse {
    pub content: String,
    pub model_used: String,
    pub backend: String,
    pub privacy_mode: PrivacyMode,
    pub local_tokens: usize,
    pub cloud_tokens: usize,
    pub masked_secrets: Vec<MaskedSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedSecret {
    pub pattern: String,
    pub original_hash: String,
    pub placeholder: String,
}

pub struct HybridMode {
    config: HybridConfig,
    secret_map: HashMap<String, String>,
}

impl HybridMode {
    pub fn new(config: HybridConfig) -> Self {
        Self {
            config,
            secret_map: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: HybridConfig) -> Self {
        self.config = config;
        self
    }

    pub fn detect_sensitive_data(&self, text: &str) -> Vec<MaskedSecret> {
        let mut secrets = Vec::new();
        
        for pattern in &self.config.sensitive_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for mat in re.find_iter(text) {
                    let matched = mat.as_str();
                    let hash = Self::hash_secret(matched);
                    let placeholder = format!("{{{{SECRET_{}}}}}", &hash[..8]);
                    
                    secrets.push(MaskedSecret {
                        pattern: pattern.clone(),
                        original_hash: hash,
                        placeholder,
                    });
                }
            }
        }
        
        secrets
    }

    fn hash_secret(secret: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        secret.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn mask_secrets(&mut self, text: &str) -> (String, Vec<MaskedSecret>) {
        let mut secrets = Vec::new();
        let mut masked = text.to_string();
        
        for pattern in &self.config.sensitive_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                let matches: Vec<String> = re.find_iter(text).map(|m| m.as_str().to_string()).collect();
                for matched in matches {
                    let hash = Self::hash_secret(&matched);
                    let placeholder = format!("{{{{SECRET_{}}}}}", &hash[..8]);
                    
                    self.secret_map.insert(placeholder.clone(), matched.clone());
                    masked = masked.replace(&matched, &placeholder);
                    
                    secrets.push(MaskedSecret {
                        pattern: pattern.clone(),
                        original_hash: hash,
                        placeholder,
                    });
                }
            }
        }
        
        (masked, secrets)
    }

    pub fn unmask_secrets(&self, text: &str) -> String {
        let mut unmasked = text.to_string();
        
        for (placeholder, original) in &self.secret_map {
            unmasked = unmasked.replace(placeholder, original);
        }
        
        unmasked
    }

    pub fn extract_structure(&self, text: &str) -> serde_json::Value {
        let mut structure = serde_json::Map::new();
        
        let lines: Vec<&str> = text.lines().collect();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut imports = Vec::new();
        let mut constants = Vec::new();
        
        for line in &lines {
            let trimmed = line.trim();
            
            if trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") {
                if let Some(name) = Self::extract_fn_name(trimmed) {
                    functions.push(name);
                }
            } else if trimmed.starts_with("struct ") {
                if let Some(name) = Self::extract_struct_name(trimmed) {
                    structs.push(name);
                }
            } else if trimmed.starts_with("use ") || trimmed.starts_with("import ") {
                imports.push(trimmed.to_string());
            } else if trimmed.starts_with("const ") || trimmed.starts_with("static ") {
                if let Some(name) = Self::extract_const_name(trimmed) {
                    constants.push(name);
                }
            }
        }
        
        structure.insert("functions".to_string(), serde_json::json!(functions));
        structure.insert("structs".to_string(), serde_json::json!(structs));
        structure.insert("imports".to_string(), serde_json::json!(imports));
        structure.insert("constants".to_string(), serde_json::json!(constants));
        structure.insert("line_count".to_string(), serde_json::json!(lines.len()));
        
        serde_json::Value::Object(structure)
    }

    fn extract_fn_name(line: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?:async\s+)?fn\s+(\w+)").ok()?;
        re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
    }

    fn extract_struct_name(line: &str) -> Option<String> {
        let re = regex::Regex::new(r"struct\s+(\w+)").ok()?;
        re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
    }

    fn extract_const_name(line: &str) -> Option<String> {
        let re = regex::Regex::new(r"(?:const|static)\s+(\w+)").ok()?;
        re.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
    }

    pub async fn execute_hybrid(&mut self, request: HybridRequest) -> Result<HybridResponse> {
        match self.config.privacy_mode {
            PrivacyMode::FullLocal => {
                return Err(anyhow::anyhow!("Full local mode - cloud not allowed"));
            }
            PrivacyMode::StructureOnly => {
                self.execute_structure_only(request).await
            }
            PrivacyMode::MaskedData => {
                self.execute_masked_data(request).await
            }
            PrivacyMode::FullCloud => {
                self.execute_full_cloud(request).await
            }
        }
    }

    async fn execute_structure_only(&mut self, request: HybridRequest) -> Result<HybridResponse> {
        let local_context = if let Some(ctx) = request.context {
            self.extract_structure(&serde_json::to_string(&ctx)?)
        } else {
            serde_json::json!({})
        };
        
        let prompt = self.build_structure_prompt(&request.messages, &local_context);
        
        let (masked_prompt, secrets) = self.mask_secrets(&prompt);
        
        let cloud_response = self.call_cloud_api(&masked_prompt, request.cloud_model.as_deref()).await?;
        
        let unmasked = self.unmask_secrets(&cloud_response);
        
        Ok(HybridResponse {
            content: unmasked,
            model_used: request.cloud_model.unwrap_or_default(),
            backend: "cloud".to_string(),
            privacy_mode: PrivacyMode::StructureOnly,
            local_tokens: 0,
            cloud_tokens: self.estimate_tokens(&cloud_response),
            masked_secrets: secrets,
        })
    }

    async fn execute_masked_data(&mut self, request: HybridRequest) -> Result<HybridResponse> {
        let full_prompt = self.build_full_prompt(&request.messages);
        
        let (masked_prompt, secrets) = self.mask_secrets(&full_prompt);
        
        let cloud_response = self.call_cloud_api(&masked_prompt, request.cloud_model.as_deref()).await?;
        
        let unmasked = self.unmask_secrets(&cloud_response);
        
        Ok(HybridResponse {
            content: unmasked,
            model_used: request.cloud_model.unwrap_or_default(),
            backend: "cloud".to_string(),
            privacy_mode: PrivacyMode::MaskedData,
            local_tokens: self.estimate_tokens(&full_prompt),
            cloud_tokens: self.estimate_tokens(&cloud_response),
            masked_secrets: secrets,
        })
    }

    async fn execute_full_cloud(&mut self, request: HybridRequest) -> Result<HybridResponse> {
        let prompt = self.build_full_prompt(&request.messages);
        
        let cloud_response = self.call_cloud_api(&prompt, request.cloud_model.as_deref()).await?;
        
        Ok(HybridResponse {
            content: cloud_response.clone(),
            model_used: request.cloud_model.unwrap_or_default(),
            backend: "cloud".to_string(),
            privacy_mode: PrivacyMode::FullCloud,
            local_tokens: 0,
            cloud_tokens: self.estimate_tokens(&prompt) + self.estimate_tokens(&cloud_response),
            masked_secrets: vec![],
        })
    }

    fn build_full_prompt(&self, messages: &[Message]) -> String {
        messages.iter().map(|m| {
            let role = match m.role {
                MessageRole::System => "System",
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::Tool => "Tool",
            };
            format!("{}: {}", role, m.content)
        }).collect::<Vec<_>>().join("\n\n")
    }

    fn build_structure_prompt(&self, messages: &[Message], structure: &serde_json::Value) -> String {
        format!(
            "You are a code assistant. Here is the code structure:\n{}\n\nUser request: {}\n\nProvide implementation based on this structure only.",
            serde_json::to_string_pretty(structure).unwrap_or_default(),
            messages.iter().filter(|m| m.role == MessageRole::User).map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n")
        )
    }

    async fn call_cloud_api(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let provider = self.config.cloud_provider.as_ref().unwrap_or(&CloudProvider::OpenAI);
        let api_key = self.config.api_key.as_ref().ok_or_else(|| anyhow::anyhow!("API key not configured"))?;
        
        match provider {
            CloudProvider::OpenAI => self.call_openai(prompt, model, api_key).await,
            CloudProvider::Anthropic => self.call_anthropic(prompt, model, api_key).await,
            CloudProvider::Custom => self.call_custom(prompt, model, api_key).await,
        }
    }

    async fn call_openai(&self, prompt: &str, model: Option<&str>, api_key: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = self.config.api_base_url.as_deref().unwrap_or("https://api.openai.com/v1/chat/completions");
        
        let payload = serde_json::json!({
            "model": model.unwrap_or("gpt-4"),
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.7,
            "max_tokens": 4096,
        });
        
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        Ok(result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
    }

    async fn call_anthropic(&self, prompt: &str, model: Option<&str>, api_key: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = self.config.api_base_url.as_deref().unwrap_or("https://api.anthropic.com/v1/messages");
        
        let payload = serde_json::json!({
            "model": model.unwrap_or("claude-3-opus-20240229"),
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}],
        });
        
        let response = client
            .post(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        Ok(result["content"][0]["text"].as_str().unwrap_or("").to_string())
    }

    async fn call_custom(&self, prompt: &str, model: Option<&str>, api_key: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = self.config.api_base_url.as_ref().ok_or_else(|| anyhow::anyhow!("Custom API URL not configured"))?;
        
        let payload = serde_json::json!({
            "model": model.unwrap_or("default"),
            "prompt": prompt,
        });
        
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        Ok(result["response"].as_str().or_else(|| result["content"].as_str()).unwrap_or("").to_string())
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        (text.len() as f32 / 3.5).ceil() as usize
    }
}

impl Default for HybridMode {
    fn default() -> Self {
        Self::new(HybridConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Message, MessageRole};

    #[test]
    fn test_detect_api_key() {
        let config = HybridConfig::default();
        let mode = HybridMode::new(config);
        
        let text = "const API_KEY = \"sk-1234567890abcdef\";";
        let secrets = mode.detect_sensitive_data(text);
        
        assert!(!secrets.is_empty());
    }

    #[test]
    fn test_mask_unmask() {
        let config = HybridConfig::default();
        let mut mode = HybridMode::new(config);
        
        let text = "password = \"secret123\"";
        let (masked, secrets) = mode.mask_secrets(text);
        
        assert!(masked.contains("SECRET_"));
        assert!(!masked.contains("secret123"));
        
        let unmasked = mode.unmask_secrets(&masked);
        assert!(unmasked.contains("secret123"));
    }

    #[test]
    fn test_extract_structure() {
        let config = HybridConfig::default();
        let mode = HybridMode::new(config);
        
        let code = r#"
use std::collections::HashMap;

struct User {
    name: String,
}

fn get_user(id: u64) -> User {
    User { name: "test".to_string() }
}

const MAX_USERS: u32 = 100;
"#;
        
        let structure = mode.extract_structure(code);
        
        assert!(structure["functions"].as_array().unwrap().contains(&serde_json::json!("get_user")));
        assert!(structure["structs"].as_array().unwrap().contains(&serde_json::json!("User")));
        assert!(structure["constants"].as_array().unwrap().contains(&serde_json::json!("MAX_USERS")));
    }
}
