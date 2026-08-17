use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::skill_manifest::{
    generate_skill_boilerplate, LoadedSkill, SkillLocation, SkillManifest, SkillRuntime,
};
use crate::skill_runner::{SkillExecutionResult, SkillRunner};

/// Thread-safe in-memory registry of all discovered LOCUS skills
pub struct SkillRegistry {
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    skills: Arc<RwLock<HashMap<String, LoadedSkill>>>,
    runner: SkillRunner,
}

impl SkillRegistry {
    pub fn new(workspace_root: Option<PathBuf>) -> Self {
        let registry = Self {
            workspace_root: Arc::new(RwLock::new(workspace_root)),
            skills: Arc::new(RwLock::new(HashMap::new())),
            runner: SkillRunner::new(),
        };
        registry.rescan();
        registry
    }

    /// Update workspace root path and rescan
    pub fn set_workspace_root<P: AsRef<Path>>(&self, root: Option<P>) {
        *self.workspace_root.write() = root.map(|r| r.as_ref().to_path_buf());
        self.rescan();
    }

    /// Rescans workspace `.locus/skills/` and global `~/.locus/skills/`
    pub fn rescan(&self) -> Vec<LoadedSkill> {
        let mut loaded = HashMap::new();

        // 1. Scan Global User Home Skills: ~/.locus/skills/
        if let Some(home_dir) = get_user_home_dir() {
            let global_skills_dir = home_dir.join(".locus").join("skills");
            self.scan_skills_in_directory(&global_skills_dir, true, &mut loaded);
        }

        // 2. Scan Workspace Skills: <workspace>/.locus/skills/ (overrides global if same ID)
        if let Some(ref ws_root) = *self.workspace_root.read() {
            let ws_skills_dir = ws_root.join(".locus").join("skills");
            self.scan_skills_in_directory(&ws_skills_dir, false, &mut loaded);
        }

        let mut skills_guard = self.skills.write();
        *skills_guard = loaded;

        let result: Vec<LoadedSkill> = skills_guard.values().cloned().collect();
        info!("SkillRegistry discovered {} active skills", result.len());
        result
    }

    fn scan_skills_in_directory(
        &self,
        base_dir: &Path,
        is_global: bool,
        map: &mut HashMap<String, LoadedSkill>,
    ) {
        if !base_dir.exists() || !base_dir.is_dir() {
            return;
        }

        let entries = match fs::read_dir(base_dir) {
            Ok(e) => e,
            Err(err) => {
                warn!("Failed to read skills directory at {:?}: {}", base_dir, err);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check for skill.yaml, skill.yml, or skill.json
                let yaml_path = path.join("skill.yaml");
                let yml_path = path.join("skill.yml");
                let json_path = path.join("skill.json");

                let manifest_path = if yaml_path.exists() {
                    Some(yaml_path)
                } else if yml_path.exists() {
                    Some(yml_path)
                } else if json_path.exists() {
                    Some(json_path)
                } else {
                    None
                };

                if let Some(mf_path) = manifest_path {
                    match SkillManifest::from_file(&mf_path) {
                        Ok(manifest) => {
                            let loc = if is_global {
                                SkillLocation::Global(path.clone())
                            } else {
                                SkillLocation::Workspace(path.clone())
                            };
                            let loaded_skill = LoadedSkill::new(manifest.clone(), path, loc);
                            debug!("Loaded skill '{}' from {:?}", manifest.id, mf_path);
                            map.insert(manifest.id, loaded_skill);
                        }
                        Err(e) => {
                            warn!("Failed to load skill manifest at {:?}: {}", mf_path, e);
                        }
                    }
                }
            }
        }
    }

    /// List all loaded skills
    pub fn list_skills(&self) -> Vec<LoadedSkill> {
        self.skills.read().values().cloned().collect()
    }

    /// Retrieve a specific loaded skill by ID
    pub fn get_skill(&self, id: &str) -> Option<LoadedSkill> {
        self.skills.read().get(id).cloned()
    }

    /// Toggles a skill enabled or disabled state
    pub fn toggle_skill(&self, id: &str, enabled: bool) -> Result<bool> {
        let mut skills_guard = self.skills.write();
        if let Some(skill) = skills_guard.get_mut(id) {
            skill.manifest.enabled = enabled;
            // Optionally persist update back to disk manifest file
            let manifest_path = skill.dir_path.join("skill.yaml");
            if manifest_path.exists() {
                if let Ok(yaml_str) = serde_yaml::to_string(&skill.manifest) {
                    let _ = fs::write(&manifest_path, yaml_str);
                }
            }
            Ok(enabled)
        } else {
            Err(anyhow!("Skill with ID '{}' not found", id))
        }
    }

    /// Executes a skill by ID with given arguments
    pub async fn execute_skill(
        &self,
        id: &str,
        args: &serde_json::Value,
    ) -> Result<SkillExecutionResult> {
        let skill = self
            .get_skill(id)
            .ok_or_else(|| anyhow!("Skill '{}' not found in registry", id))?;

        self.runner.execute(&skill, args, None).await
    }

    /// Scaffolds and writes a new skill to disk and immediately indexes it
    pub fn create_skill(
        &self,
        id: &str,
        name: &str,
        runtime: SkillRuntime,
        language: &str,
        description: &str,
        target_in_workspace: bool,
    ) -> Result<LoadedSkill> {
        let (manifest, code) =
            generate_skill_boilerplate(id, name, runtime, language, description);

        // Determine destination folder
        let base_skills_dir = if target_in_workspace {
            let ws_guard = self.workspace_root.read();
            let ws_dir = ws_guard
                .as_ref()
                .ok_or_else(|| anyhow!("Cannot create workspace skill: No active workspace opened"))?;
            ws_dir.join(".locus").join("skills")
        } else {
            let home = get_user_home_dir()
                .ok_or_else(|| anyhow!("Could not resolve user home directory"))?;
            home.join(".locus").join("skills")
        };

        let skill_dir = base_skills_dir.join(&manifest.id);
        fs::create_dir_all(&skill_dir)
            .with_context(|| format!("Failed to create skill directory at {:?}", skill_dir))?;

        // Write skill.yaml
        let yaml_str = serde_yaml::to_string(&manifest)
            .context("Failed to serialize skill manifest to YAML")?;
        let manifest_path = skill_dir.join("skill.yaml");
        fs::write(&manifest_path, yaml_str)
            .with_context(|| format!("Failed to write manifest at {:?}", manifest_path))?;

        // Write entrypoint code file
        let code_path = skill_dir.join(&manifest.entrypoint);
        fs::write(&code_path, code)
            .with_context(|| format!("Failed to write skill code file at {:?}", code_path))?;

        let loc = if target_in_workspace {
            SkillLocation::Workspace(skill_dir.clone())
        } else {
            SkillLocation::Global(skill_dir.clone())
        };

        let loaded = LoadedSkill::new(manifest.clone(), skill_dir, loc);
        self.skills.write().insert(manifest.id.clone(), loaded.clone());

        info!("Created new skill '{}' at {:?}", manifest.id, manifest_path);
        Ok(loaded)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Helper to resolve user home directory
fn get_user_home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOMEPATH"))
            .ok()
            .map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skill_registry_creation_and_execution() {
        let temp_dir = std::env::temp_dir().join(format!("locus_reg_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let registry = SkillRegistry::new(Some(temp_dir.clone()));

        // Create a new python skill
        let created = registry
            .create_skill(
                "calc_square",
                "Calculate Square",
                SkillRuntime::Script,
                "python",
                "Calculates number square",
                true,
            )
            .unwrap();

        assert_eq!(created.manifest.id, "calc_square");
        assert!(created.entrypoint_path.exists());

        // Check if listed
        let list = registry.list_skills();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].manifest.id, "calc_square");

        // Toggle skill
        registry.toggle_skill("calc_square", false).unwrap();
        assert!(!registry.get_skill("calc_square").unwrap().manifest.enabled);

        registry.toggle_skill("calc_square", true).unwrap();
        assert!(registry.get_skill("calc_square").unwrap().manifest.enabled);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
