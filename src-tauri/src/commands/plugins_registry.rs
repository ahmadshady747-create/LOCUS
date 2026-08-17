//! Tauri IPC commands for Decentralized Addon Registry.

use locus_plugins::{InstalledAddon, RegistryStore};
use once_cell::sync::Lazy;

static REGISTRY_STORE: Lazy<RegistryStore> = Lazy::new(RegistryStore::load_or_default);

#[tauri::command]
pub fn plugins_registry_list() -> Result<Vec<InstalledAddon>, String> {
    Ok(REGISTRY_STORE.list_installed())
}

#[tauri::command]
pub async fn plugins_registry_install_git(repo_url: String) -> Result<InstalledAddon, String> {
    REGISTRY_STORE
        .install_from_git(&repo_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugins_registry_toggle(addon_id: String, enabled: bool) -> Result<bool, String> {
    REGISTRY_STORE
        .toggle_addon(&addon_id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugins_registry_uninstall(addon_id: String) -> Result<bool, String> {
    REGISTRY_STORE
        .uninstall_addon(&addon_id)
        .map_err(|e| e.to_string())
}
