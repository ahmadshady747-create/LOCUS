//! Free Provider Radar & Quota Intelligence
//!
//! Proactively discovers permanent free-tier AI providers (Gemini 2.0 Flash, Groq Cloud,
//! Cerebras, OpenRouter Free Pool, Mistral Codestral), monitors generous free quotas, and filters
//! out already configured or dismissed keys to maximize zero-cost developer inference.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeProviderInfo {
    pub id: String,
    pub name: String,
    pub badge: String,
    pub free_tier_limits: String,
    pub speed_tier: String,
    pub key_url: String,
    pub recommended_model: String,
    pub description: String,
    pub card_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeProviderSuggestion {
    pub provider: FreeProviderInfo,
    pub potential_token_savings: String,
    pub is_dismissed: bool,
}

pub struct FreeProviderRadar;

impl FreeProviderRadar {
    /// Returns the complete registry of permanently free or generous free-tier AI providers.
    pub fn get_all_free_profiles() -> Vec<FreeProviderInfo> {
        vec![
            FreeProviderInfo {
                id: "gemini".to_string(),
                name: "Google AI Studio".to_string(),
                badge: "1500 Req/Day Free (No Card)".to_string(),
                free_tier_limits: "15 RPM · 1,000,000 TPM · 1,500 RPD".to_string(),
                speed_tier: "~280 tokens/sec".to_string(),
                key_url: "https://aistudio.google.com/app/apikey".to_string(),
                recommended_model: "gemini-2.0-flash".to_string(),
                description: "Frontier multimodal reasoning with a 1M token context window, completely free with zero credit card requirements.".to_string(),
                card_required: false,
            },
            FreeProviderInfo {
                id: "groq".to_string(),
                name: "Groq Cloud (LPU)".to_string(),
                badge: "14,400 Req/Day Free (Sub-Second)".to_string(),
                free_tier_limits: "30 RPM · 14,400 Requests/Day Free".to_string(),
                speed_tier: "~800 tokens/sec (Ultra-Fast)".to_string(),
                key_url: "https://console.groq.com/keys".to_string(),
                recommended_model: "llama-3.3-70b-versatile".to_string(),
                description: "World's fastest LPU inference engine for Llama 3.3 70B, ideal for routine Micro tasks and AST fixes.".to_string(),
                card_required: false,
            },
            FreeProviderInfo {
                id: "cerebras".to_string(),
                name: "Cerebras Cloud".to_string(),
                badge: "1M Tokens/Day Free".to_string(),
                free_tier_limits: "30 RPM · 1,000,000 Tokens/Day".to_string(),
                speed_tier: "~1,800 tokens/sec".to_string(),
                key_url: "https://cloud.cerebras.ai".to_string(),
                recommended_model: "llama3.1-70b".to_string(),
                description: "Wafer-scale hardware with ultra-dense bandwidth and instant responses.".to_string(),
                card_required: false,
            },
            FreeProviderInfo {
                id: "openrouter".to_string(),
                name: "OpenRouter Free Pool".to_string(),
                badge: "Community Free Routing".to_string(),
                free_tier_limits: "20 RPM · Free Aggregated Pool".to_string(),
                speed_tier: "~150 tokens/sec".to_string(),
                key_url: "https://openrouter.ai/keys".to_string(),
                recommended_model: "meta-llama/llama-3.3-70b-instruct:free".to_string(),
                description: "Aggregated free endpoints across DeepSeek R1, Llama 3.3, and Gemma with a single API key.".to_string(),
                card_required: false,
            },
            FreeProviderInfo {
                id: "mistral".to_string(),
                name: "Mistral Codestral".to_string(),
                badge: "Developer Free Tier".to_string(),
                free_tier_limits: "1 RPS · Codestral Testing Quota".to_string(),
                speed_tier: "~120 tokens/sec".to_string(),
                key_url: "https://console.mistral.ai/api-keys/".to_string(),
                recommended_model: "codestral-latest".to_string(),
                description: "High-accuracy code completion and Fill-In-The-Middle (FIM) model free for developer testing.".to_string(),
                card_required: false,
            },
        ]
    }

    /// Evaluates available free providers against currently configured provider keys
    /// and the local dismiss list, returning active recommendations.
    pub fn get_active_suggestions(configured_provider_ids: &[String]) -> Vec<FreeProviderSuggestion> {
        let dismissed = Self::get_dismissed_providers();
        let profiles = Self::get_all_free_profiles();

        let is_configured = |id: &str| {
            configured_provider_ids
                .iter()
                .any(|c| c.eq_ignore_ascii_case(id))
        };

        profiles
            .into_iter()
            .filter(|p| !is_configured(&p.id) && !dismissed.iter().any(|d| d.eq_ignore_ascii_case(&p.id)))
            .map(|p| {
                let savings = match p.id.as_str() {
                    "groq" => "Saves 100% token cost on all Micro tasks & AST refactors".to_string(),
                    "gemini" => "Saves 100% token cost on high-context architectural planning".to_string(),
                    "cerebras" => "Provides extreme 1,800 tokens/sec speed for quick reviews".to_string(),
                    "openrouter" => "Gives free access to DeepSeek R1 & Llama 3.3".to_string(),
                    _ => "Free developer quota without recurring charges".to_string(),
                };

                FreeProviderSuggestion {
                    provider: p,
                    potential_token_savings: savings,
                    is_dismissed: false,
                }
            })
            .collect()
    }

    // --- Dismiss List Persistence ---

    fn get_dismiss_file_path() -> PathBuf {
        let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join(".locus").join("dismissed_radars.json")
    }

    pub fn get_dismissed_providers() -> Vec<String> {
        let path = Self::get_dismiss_file_path();
        if !path.exists() {
            return Vec::new();
        }
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn dismiss_provider(provider_id: &str) -> Result<()> {
        let mut list = Self::get_dismissed_providers();
        let id_clean = provider_id.trim().to_lowercase();
        if !list.iter().any(|item| item.eq_ignore_ascii_case(&id_clean)) {
            list.push(id_clean);
        }

        let path = Self::get_dismiss_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&list)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn undismiss_provider(provider_id: &str) -> Result<()> {
        let mut list = Self::get_dismissed_providers();
        let id_clean = provider_id.trim().to_lowercase();
        list.retain(|item| !item.eq_ignore_ascii_case(&id_clean));

        let path = Self::get_dismiss_file_path();
        let json = serde_json::to_string_pretty(&list)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_free_profiles_contains_core_providers() {
        let profiles = FreeProviderRadar::get_all_free_profiles();
        assert!(profiles.iter().any(|p| p.id == "gemini"));
        assert!(profiles.iter().any(|p| p.id == "groq"));
        assert!(profiles.iter().any(|p| p.id == "openrouter"));
        assert!(profiles.iter().any(|p| p.id == "cerebras"));
    }

    #[test]
    fn test_filter_out_already_configured_providers() {
        let configured = vec!["gemini".to_string(), "groq".to_string()];
        let suggestions = FreeProviderRadar::get_active_suggestions(&configured);

        assert!(!suggestions.iter().any(|s| s.provider.id == "gemini"));
        assert!(!suggestions.iter().any(|s| s.provider.id == "groq"));
        assert!(suggestions.iter().any(|s| s.provider.id == "cerebras"));
        assert!(suggestions.iter().any(|s| s.provider.id == "openrouter"));
    }

    #[test]
    fn test_free_provider_spec_attributes() {
        let profiles = FreeProviderRadar::get_all_free_profiles();
        let groq = profiles.iter().find(|p| p.id == "groq").unwrap();
        assert!(!groq.card_required);
        assert!(groq.free_tier_limits.contains("14,400"));
        assert!(groq.speed_tier.contains("800"));
    }
}
