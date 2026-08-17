//! Core Slots Registry and Engine.

use crate::drivers::{InMemoryBM25Driver, MockIsolationDriver, NativeProcessDriver, RipgrepDriver};
use crate::traits::{ContextSlot, SandboxSlot};
use crate::types::{SlotDescriptor, SlotError, SlotType, SlotsConfig};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

pub struct SlotsEngine {
    config: RwLock<SlotsConfig>,
    context_drivers: HashMap<String, Arc<dyn ContextSlot>>,
    sandbox_drivers: HashMap<String, Arc<dyn SandboxSlot>>,
    config_path: Option<PathBuf>,
}

impl SlotsEngine {
    /// Creates a new SlotsEngine with default in-tree drivers and configuration.
    pub fn new() -> Self {
        let mut context_drivers: HashMap<String, Arc<dyn ContextSlot>> = HashMap::new();
        let mut sandbox_drivers: HashMap<String, Arc<dyn SandboxSlot>> = HashMap::new();

        // Register default drivers
        context_drivers.insert("bm25".to_string(), Arc::new(InMemoryBM25Driver::new()));
        context_drivers.insert("ripgrep".to_string(), Arc::new(RipgrepDriver::new()));

        sandbox_drivers.insert("native".to_string(), Arc::new(NativeProcessDriver::new()));
        sandbox_drivers.insert("mock".to_string(), Arc::new(MockIsolationDriver::new()));

        let default_config = SlotsConfig::default();
        let config_path = Self::default_config_path();

        Self {
            config: RwLock::new(default_config),
            context_drivers,
            sandbox_drivers,
            config_path,
        }
    }

    /// Loads the configuration from disk (`~/.locus/slots.json`) or falls back to defaults.
    pub fn load_from_disk_or_default() -> Self {
        let engine = Self::new();

        if let Some(path) = &engine.config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(loaded_config) = serde_json::from_str::<SlotsConfig>(&content) {
                        info!("Loaded SlotsConfig from disk: {:?}", path);
                        *engine.config.write() = loaded_config;
                        return engine;
                    }
                }
            }
        }

        engine
    }

    /// Returns the active configuration snapshot.
    pub fn get_config(&self) -> SlotsConfig {
        self.config.read().clone()
    }

    /// Updates the active driver for a specific slot type.
    pub fn set_active_driver(&self, slot_type: SlotType, driver_id: &str) -> Result<SlotsConfig, SlotError> {
        match slot_type {
            SlotType::Context => {
                if !self.context_drivers.contains_key(driver_id) {
                    return Err(SlotError::DriverNotFound(driver_id.to_string(), slot_type));
                }
                let mut cfg = self.config.write();
                cfg.active_context_driver = driver_id.to_string();
                for desc in &mut cfg.descriptors {
                    if desc.slot_type == SlotType::Context {
                        desc.is_active = desc.id == driver_id;
                    }
                }
            }
            SlotType::Sandbox => {
                if !self.sandbox_drivers.contains_key(driver_id) {
                    return Err(SlotError::DriverNotFound(driver_id.to_string(), slot_type));
                }
                let mut cfg = self.config.write();
                cfg.active_sandbox_driver = driver_id.to_string();
                for desc in &mut cfg.descriptors {
                    if desc.slot_type == SlotType::Sandbox {
                        desc.is_active = desc.id == driver_id;
                    }
                }
            }
        }

        let updated = self.config.read().clone();
        let _ = self.save_to_disk(&updated);
        info!("Updated active driver for {:?} to '{}'", slot_type, driver_id);
        Ok(updated)
    }

    /// Returns the currently active ContextSlot driver.
    pub fn get_active_context_driver(&self) -> Result<Arc<dyn ContextSlot>, SlotError> {
        let active_id = self.config.read().active_context_driver.clone();
        self.context_drivers
            .get(&active_id)
            .cloned()
            .ok_or_else(|| SlotError::DriverNotFound(active_id, SlotType::Context))
    }

    /// Returns the currently active SandboxSlot driver.
    pub fn get_active_sandbox_driver(&self) -> Result<Arc<dyn SandboxSlot>, SlotError> {
        let active_id = self.config.read().active_sandbox_driver.clone();
        self.sandbox_drivers
            .get(&active_id)
            .cloned()
            .ok_or_else(|| SlotError::DriverNotFound(active_id, SlotType::Sandbox))
    }

    /// Lists all available slot descriptors.
    pub fn list_available_descriptors(&self) -> Vec<SlotDescriptor> {
        self.config.read().descriptors.clone()
    }

    /// Saves the current configuration to disk.
    fn save_to_disk(&self, config: &SlotsConfig) -> Result<(), SlotError> {
        if let Some(path) = &self.config_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let json = serde_json::to_string_pretty(config)
                .map_err(|e| SlotError::ConfigError(e.to_string()))?;
            fs::write(path, json).map_err(|e| SlotError::ConfigError(e.to_string()))?;
            debug!("Saved SlotsConfig to: {:?}", path);
        }
        Ok(())
    }

    fn default_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".locus").join("slots.json"))
    }
}

impl Default for SlotsEngine {
    fn default() -> Self {
        Self::new()
    }
}
