//! Decentralized Addon Registry & Git Installer for LOCUS.
//!
//! Enables zero-friction installation of community plugins/addons directly from Git repositories
//! (e.g. `github:user/repo` or full HTTPS Git URLs) into `~/.locus/addons/`, with manifest validation
//! and local registry state management (`~/.locus/plugins_registry.json`).

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddonManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository: String,
    pub entrypoint: String,
    pub required_slots: Vec<String>,
    pub permissions: Vec<String>,
}

impl Default for AddonManifest {
    fn default() -> Self {
        Self {
            id: "unknown_addon".to_string(),
            name: "Community Addon".to_string(),
            version: "0.1.0".to_string(),
            description: "LOCUS Community Plugin".to_string(),
            author: "Unknown".to_string(),
            repository: String::new(),
            entrypoint: "index.js".to_string(),
            required_slots: Vec::new(),
            permissions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledAddon {
    pub manifest: AddonManifest,
    pub install_path: PathBuf,
    pub enabled: bool,
    pub installed_at: String,
    pub last_updated: String,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Addon '{0}' not found in registry")]
    NotFound(String),

    #[error("Invalid Git URL '{0}': {1}")]
    InvalidGitUrl(String, String),

    #[error("Git clone failed: {0}")]
    GitCloneFailed(String),

    #[error("Manifest file not found or invalid: {0}")]
    InvalidManifest(String),

    #[error("Registry I/O error: {0}")]
    IoError(String),
}

/// Git Addon Installer Engine.
pub struct GitAddonInstaller;

impl GitAddonInstaller {
    /// Normalizes git shorthands into standard cloneable HTTPS URLs.
    pub fn normalize_git_url(raw_url: &str) -> Result<String, RegistryError> {
        let trimmed = raw_url.trim();

        if trimmed.is_empty() {
            return Err(RegistryError::InvalidGitUrl(raw_url.to_string(), "URL cannot be empty".to_string()));
        }

        if trimmed.starts_with("github:") {
            let repo = trimmed.trim_start_matches("github:").trim_matches('/');
            if repo.contains('/') {
                return Ok(format!("https://github.com/{}.git", repo));
            }
        } else if trimmed.starts_with("gitlab:") {
            let repo = trimmed.trim_start_matches("gitlab:").trim_matches('/');
            if repo.contains('/') {
                return Ok(format!("https://gitlab.com/{}.git", repo));
            }
        } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("git@") {
            return Ok(trimmed.to_string());
        }

        // Default heuristic: if format is "owner/repo", assume GitHub
        if trimmed.contains('/') && !trimmed.contains(' ') && !trimmed.contains(':') {
            return Ok(format!("https://github.com/{}.git", trimmed.trim_matches('/')));
        }

        Err(RegistryError::InvalidGitUrl(
            raw_url.to_string(),
            "Expected format like 'github:owner/repo', 'owner/repo', or 'https://github.com/...".to_string(),
        ))
    }

    /// Clones a git repository into the specified directory with shallow clone (`--depth 1`).
    pub async fn clone_addon(git_url: &str, destination: &Path) -> Result<(), RegistryError> {
        if destination.exists() {
            let _ = fs::remove_dir_all(destination);
        }

        if let Some(parent) = destination.parent() {
            let _ = fs::create_dir_all(parent);
        }

        debug!("Cloning addon from '{}' to {:?}", git_url, destination);

        let mut cmd = Command::new("git");
        cmd.args(["clone", "--depth", "1", git_url, &destination.to_string_lossy()]);

        let output = cmd
            .output()
            .await
            .map_err(|e| RegistryError::GitCloneFailed(format!("Failed to execute git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RegistryError::GitCloneFailed(stderr.to_string()));
        }

        Ok(())
    }

    /// Reads and validates `manifest.json` or `locus-plugin.toml` inside the cloned directory.
    pub fn read_manifest(addon_dir: &Path) -> Result<AddonManifest, RegistryError> {
        let json_manifest = addon_dir.join("manifest.json");
        let toml_manifest = addon_dir.join("locus-plugin.toml");

        if json_manifest.exists() {
            let content = fs::read_to_string(&json_manifest)
                .map_err(|e| RegistryError::InvalidManifest(format!("Failed to read manifest.json: {}", e)))?;
            let manifest: AddonManifest = serde_json::from_str(&content)
                .map_err(|e| RegistryError::InvalidManifest(format!("Failed to parse manifest.json: {}", e)))?;
            return Ok(manifest);
        }

        if toml_manifest.exists() {
            let content = fs::read_to_string(&toml_manifest)
                .map_err(|e| RegistryError::InvalidManifest(format!("Failed to read locus-plugin.toml: {}", e)))?;
            let manifest: AddonManifest = toml::from_str(&content)
                .map_err(|e| RegistryError::InvalidManifest(format!("Failed to parse locus-plugin.toml: {}", e)))?;
            return Ok(manifest);
        }

        // Fallback inferred manifest from directory name if no explicit manifest exists
        let dir_name = addon_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("addon")
            .to_string();

        Ok(AddonManifest {
            id: dir_name.to_lowercase().replace(' ', "_"),
            name: dir_name,
            version: "0.1.0".to_string(),
            description: "Installed Git Addon".to_string(),
            author: "Community".to_string(),
            repository: String::new(),
            entrypoint: "index.js".to_string(),
            required_slots: Vec::new(),
            permissions: Vec::new(),
        })
    }
}

/// Local Registry Store (`~/.locus/plugins_registry.json`).
pub struct RegistryStore {
    addons: RwLock<HashMap<String, InstalledAddon>>,
    registry_file: Option<PathBuf>,
    addons_dir: Option<PathBuf>,
}

impl RegistryStore {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir().map(|h| h.join(".locus"));
        let registry_file = base_dir.as_ref().map(|b| b.join("plugins_registry.json"));
        let addons_dir = base_dir.map(|b| b.join("addons"));

        Self::with_paths(registry_file, addons_dir)
    }

    pub fn with_paths(registry_file: Option<PathBuf>, addons_dir: Option<PathBuf>) -> Self {
        Self {
            addons: RwLock::new(HashMap::new()),
            registry_file,
            addons_dir,
        }
    }

    /// Loads the registry from disk or initializes an empty registry.
    pub fn load_or_default() -> Self {
        let store = Self::new();

        if let Some(ref path) = store.registry_file {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, InstalledAddon>>(&content) {
                        info!("Loaded {} installed addon(s) from registry", map.len());
                        *store.addons.write() = map;
                        return store;
                    }
                }
            }
        }

        store
    }

    /// Lists all installed addons.
    pub fn list_installed(&self) -> Vec<InstalledAddon> {
        self.addons.read().values().cloned().collect()
    }

    /// Installs an addon from a Git URL.
    pub async fn install_from_git(&self, raw_url: &str) -> Result<InstalledAddon, RegistryError> {
        let git_url = GitAddonInstaller::normalize_git_url(raw_url)?;
        let repo_name = git_url
            .split('/')
            .last()
            .unwrap_or("addon")
            .trim_end_matches(".git")
            .to_string();

        let target_dir = match &self.addons_dir {
            Some(dir) => dir.join(&repo_name),
            None => PathBuf::from(".locus_addons").join(&repo_name),
        };

        // Clone via Git
        GitAddonInstaller::clone_addon(&git_url, &target_dir).await?;

        // Read manifest
        let mut manifest = GitAddonInstaller::read_manifest(&target_dir)?;
        if manifest.repository.is_empty() {
            manifest.repository = git_url.clone();
        }

        let now = Utc::now().to_rfc3339();
        let installed = InstalledAddon {
            manifest: manifest.clone(),
            install_path: target_dir,
            enabled: true,
            installed_at: now.clone(),
            last_updated: now,
        };

        {
            let mut map = self.addons.write();
            map.insert(manifest.id.clone(), installed.clone());
        }

        let _ = self.save_to_disk();
        info!("Successfully installed addon '{}' ({})", manifest.name, manifest.id);
        Ok(installed)
    }

    /// Toggles an addon's enabled status.
    pub fn toggle_addon(&self, addon_id: &str, enabled: bool) -> Result<bool, RegistryError> {
        let found = {
            let mut map = self.addons.write();
            if let Some(addon) = map.get_mut(addon_id) {
                addon.enabled = enabled;
                addon.last_updated = Utc::now().to_rfc3339();
                true
            } else {
                false
            }
        };

        if found {
            let _ = self.save_to_disk();
            info!("Toggled addon '{}' -> enabled: {}", addon_id, enabled);
            return Ok(enabled);
        }
        Err(RegistryError::NotFound(addon_id.to_string()))
    }

    /// Uninstalls and deletes an addon from disk and the registry.
    pub fn uninstall_addon(&self, addon_id: &str) -> Result<bool, RegistryError> {
        let removed = {
            let mut map = self.addons.write();
            map.remove(addon_id)
        };

        if let Some(addon) = removed {
            if addon.install_path.exists() {
                let _ = fs::remove_dir_all(&addon.install_path);
            }
            let _ = self.save_to_disk();
            info!("Uninstalled addon '{}'", addon_id);
            return Ok(true);
        }

        Err(RegistryError::NotFound(addon_id.to_string()))
    }

    /// Saves the registry map to disk.
    fn save_to_disk(&self) -> Result<(), RegistryError> {
        if let Some(ref path) = self.registry_file {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let json = serde_json::to_string_pretty(&*self.addons.read())
                .map_err(|e| RegistryError::IoError(e.to_string()))?;
            fs::write(path, json).map_err(|e| RegistryError::IoError(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for RegistryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_url_normalization() {
        assert_eq!(
            GitAddonInstaller::normalize_git_url("github:locus-ai/locus-tools").unwrap(),
            "https://github.com/locus-ai/locus-tools.git"
        );

        assert_eq!(
            GitAddonInstaller::normalize_git_url("gitlab:org/addon").unwrap(),
            "https://gitlab.com/org/addon.git"
        );

        assert_eq!(
            GitAddonInstaller::normalize_git_url("author/my-theme").unwrap(),
            "https://github.com/author/my-theme.git"
        );

        assert_eq!(
            GitAddonInstaller::normalize_git_url("https://github.com/custom/repo.git").unwrap(),
            "https://github.com/custom/repo.git"
        );
    }

    #[test]
    fn test_registry_store_toggle_and_uninstall() {
        let store = RegistryStore::with_paths(None, None);
        let manifest = AddonManifest {
            id: "test_addon".to_string(),
            name: "Test Addon".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Tester".to_string(),
            repository: "https://github.com/test/addon.git".to_string(),
            entrypoint: "index.js".to_string(),
            required_slots: vec!["context".to_string()],
            permissions: vec!["fs:read".to_string()],
        };

        let installed = InstalledAddon {
            manifest: manifest.clone(),
            install_path: PathBuf::from("target/test_addon"),
            enabled: true,
            installed_at: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };

        store.addons.write().insert("test_addon".to_string(), installed);

        assert_eq!(store.list_installed().len(), 1);

        // Toggle
        assert_eq!(store.toggle_addon("test_addon", false).unwrap(), false);
        assert_eq!(store.addons.read().get("test_addon").unwrap().enabled, false);

        // Uninstall
        assert!(store.uninstall_addon("test_addon").unwrap());
        assert_eq!(store.list_installed().len(), 0);
    }
}
