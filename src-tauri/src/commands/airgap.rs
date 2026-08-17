//! Tauri IPC commands for Air-Gapped Animated QR Sync.

use locus_core::{AirGapExporter, AirGapIngestProgress, AirGapReceiver, SyncPayload};
use once_cell::sync::Lazy;
use std::fs;
use uuid::Uuid;

static AIRGAP_RECEIVER: Lazy<AirGapReceiver> = Lazy::new(AirGapReceiver::new);

#[tauri::command]
pub fn airgap_generate_sync_frames() -> Result<Vec<String>, String> {
    let home = dirs::home_dir().map(|h| h.join(".locus"));
    let mut payload = SyncPayload::default();

    if let Some(ref dir) = home {
        // Read config.json
        let config_path = dir.join("config.json");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                payload.config_json = content;
            }
        }

        // Read slots.json
        let slots_path = dir.join("slots.json");
        if slots_path.exists() {
            if let Ok(content) = fs::read_to_string(&slots_path) {
                payload.slots_config = Some(content);
            }
        }

        // Read active addons from plugins_registry.json
        let reg_path = dir.join("plugins_registry.json");
        if reg_path.exists() {
            if let Ok(content) = fs::read_to_string(&reg_path) {
                if let Ok(map) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = map.as_object() {
                        payload.active_addons = obj.keys().cloned().collect();
                    }
                }
            }
        }
    }

    let session_id = Uuid::new_v4().to_string()[..8].to_string();
    AirGapExporter::generate_frames(payload, &session_id, 180).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn airgap_ingest_frame(frame_data: String) -> Result<AirGapIngestProgress, String> {
    AIRGAP_RECEIVER
        .ingest_frame(&frame_data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn airgap_apply_synced_payload(session_id: Option<String>) -> Result<bool, String> {
    let sess_id = match session_id {
        Some(id) => id,
        None => {
            return Err("Missing session_id".to_string());
        }
    };

    AIRGAP_RECEIVER
        .apply_payload(&sess_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn airgap_reset_receiver(session_id: Option<String>) -> Result<bool, String> {
    AIRGAP_RECEIVER.reset_session(session_id.as_deref());
    Ok(true)
}
