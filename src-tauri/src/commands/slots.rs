//! Tauri IPC commands for Swappable Core Slots.

use locus_plugins::{SlotDescriptor, SlotType, SlotsConfig, SlotsEngine};
use once_cell::sync::Lazy;

static SLOTS_ENGINE: Lazy<SlotsEngine> = Lazy::new(SlotsEngine::load_from_disk_or_default);

#[tauri::command]
pub fn slots_get_config() -> Result<SlotsConfig, String> {
    Ok(SLOTS_ENGINE.get_config())
}

#[tauri::command]
pub fn slots_set_driver(slot_type: String, driver_id: String) -> Result<SlotsConfig, String> {
    let parsed_slot = match slot_type.to_lowercase().as_str() {
        "context" => SlotType::Context,
        "sandbox" => SlotType::Sandbox,
        _ => return Err(format!("Unknown slot type: '{}'. Valid: 'context', 'sandbox'", slot_type)),
    };

    SLOTS_ENGINE
        .set_active_driver(parsed_slot, &driver_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn slots_list_available() -> Result<Vec<SlotDescriptor>, String> {
    Ok(SLOTS_ENGINE.list_available_descriptors())
}
