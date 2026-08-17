use locus_agents::skill_manifest::{
    LoadedSkill, SkillLocation, SkillManifest, SkillPermissions, SkillRuntime,
};
use locus_agents::skill_runner::SkillExecutionResult;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// Frontend DTO for a Loaded Skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub runtime: String,
    pub entrypoint: String,
    pub permissions: SkillPermissions,
    pub parameters: serde_json::Value,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub location_type: String,
    pub dir_path: String,
    pub is_valid: bool,
    pub load_error: Option<String>,
}

impl From<LoadedSkill> for SkillDto {
    fn from(s: LoadedSkill) -> Self {
        let loc_type = match s.location {
            SkillLocation::Workspace(_) => "workspace",
            SkillLocation::Global(_) => "global",
        };
        let runtime_str = match s.manifest.runtime {
            SkillRuntime::Wasm => "wasm",
            SkillRuntime::Script => "script",
        };

        Self {
            id: s.manifest.id,
            name: s.manifest.name,
            version: s.manifest.version,
            description: s.manifest.description,
            author: s.manifest.author,
            runtime: runtime_str.to_string(),
            entrypoint: s.manifest.entrypoint,
            permissions: s.manifest.permissions,
            parameters: s.manifest.parameters,
            enabled: s.manifest.enabled,
            timeout_seconds: s.manifest.timeout_seconds,
            location_type: loc_type.to_string(),
            dir_path: s.dir_path.to_string_lossy().to_string(),
            is_valid: s.is_valid,
            load_error: s.load_error,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSkillRequest {
    pub id: String,
    pub name: String,
    pub runtime: String, // "wasm" | "script"
    pub language: String, // "python" | "javascript" | "powershell" | "shell" | "wasm"
    pub description: String,
    pub target_in_workspace: bool,
}

#[tauri::command]
pub async fn skills_list(state: State<'_, AppState>) -> Result<Vec<SkillDto>, String> {
    let skills = state.skills.list_skills();
    Ok(skills.into_iter().map(SkillDto::from).collect())
}

#[tauri::command]
pub async fn skills_rescan(state: State<'_, AppState>) -> Result<Vec<SkillDto>, String> {
    let skills = state.skills.rescan();
    Ok(skills.into_iter().map(SkillDto::from).collect())
}

#[tauri::command]
pub async fn skills_toggle(
    state: State<'_, AppState>,
    skill_id: String,
    enabled: bool,
) -> Result<bool, String> {
    state
        .skills
        .toggle_skill(&skill_id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn skills_execute(
    state: State<'_, AppState>,
    skill_id: String,
    args: serde_json::Value,
) -> Result<SkillExecutionResult, String> {
    state
        .skills
        .execute_skill(&skill_id, &args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn skills_create(
    state: State<'_, AppState>,
    request: CreateSkillRequest,
) -> Result<SkillDto, String> {
    let runtime = if request.runtime.eq_ignore_ascii_case("wasm") {
        SkillRuntime::Wasm
    } else {
        SkillRuntime::Script
    };

    let loaded = state
        .skills
        .create_skill(
            &request.id,
            &request.name,
            runtime,
            &request.language,
            &request.description,
            request.target_in_workspace,
        )
        .map_err(|e| e.to_string())?;

    Ok(SkillDto::from(loaded))
}
