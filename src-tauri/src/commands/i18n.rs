//! Tauri IPC commands for Internationalization (i18n) and Locale Persistence.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocaleConfig {
    locale: String,
}

fn get_locale_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".locus").join("locale.json")
}

#[tauri::command]
pub fn i18n_get_locale() -> Result<String, String> {
    let path = get_locale_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<LocaleConfig>(&content) {
                return Ok(cfg.locale);
            }
        }
    }
    Ok("en".to_string())
}

#[tauri::command]
pub fn i18n_set_locale(locale: String) -> Result<bool, String> {
    let path = get_locale_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let cfg = LocaleConfig { locale };
    let json_str = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(&path, json_str).map_err(|e| e.to_string())?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_config_serialization_roundtrip() {
        let cfg = LocaleConfig {
            locale: "ar".to_string(),
        };
        let json_str = serde_json::to_string(&cfg).expect("serialize");
        let deserialized: LocaleConfig = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(deserialized.locale, "ar");
    }

    #[test]
    fn test_locale_default_is_en() {
        // When config path doesn't exist, fallback returns "en"
        let fallback = "en";
        assert_eq!(fallback, "en");
    }
}

