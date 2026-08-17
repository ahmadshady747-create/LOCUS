use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub report_id: String,
    pub generated_at: String,
    pub locus_version: String,
    pub system_environment: SystemEnvironmentDto,
    pub workspace_status: WorkspaceDiagnosticDto,
    pub ai_engine_status: AiEngineDiagnosticDto,
    pub p2p_mesh_status: MeshDiagnosticDto,
    pub agents_pool_status: AgentsDiagnosticDto,
    pub sanitized_diagnostic_logs: Vec<DiagnosticLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEnvironmentDto {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub logical_cpu_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDiagnosticDto {
    pub has_workspace_loaded: bool,
    pub total_indexed_files: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEngineDiagnosticDto {
    pub selected_model: Option<String>,
    pub local_models_count: usize,
    pub local_models_detected: Vec<String>,
    pub fallback_strategy: String,
    pub fallback_enabled: bool,
    pub fallback_targets: Vec<String>,
    pub configured_cloud_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshDiagnosticDto {
    pub is_running: bool,
    pub discovered_peer_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsDiagnosticDto {
    pub active_processes_count: usize,
    pub max_memory_ceiling_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    pub timestamp: String,
    pub level: String,
    pub subsystem: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDiagnosticResult {
    pub success: bool,
    pub file_name: String,
    pub json_payload: String,
    pub summary: String,
}

/// Delegates to the shared core sanitization engine.
pub fn sanitize_text(input: &str) -> String {
    locus_core::diagnostics::sanitize_text(input)
}

#[tauri::command]
pub async fn system_get_diagnostics(
    state: State<'_, AppState>,
) -> Result<DiagnosticReport, String> {
    let now = chrono::Utc::now();
    let report_id = uuid::Uuid::new_v4().to_string();

    // 1. System info
    let os_str = std::env::consts::OS.to_string();
    let arch_str = std::env::consts::ARCH.to_string();
    let family_str = std::env::consts::FAMILY.to_string();
    let logical_cpus = num_cpus::get();

    // 2. Workspace info
    let fs = state.fs_engine.read().await;
    let ws_index = fs.get_index().await;
    let ws_diagnostic = WorkspaceDiagnosticDto {
        has_workspace_loaded: ws_index.total_files > 0,
        total_indexed_files: ws_index.total_files,
        total_size_bytes: ws_index.total_size,
    };

    // 3. LLM & Fallback router info
    let llm = state.llm.read().await;
    let local_models = llm.detect_available_models().await.unwrap_or_default();
    let local_models_detected: Vec<String> = local_models.iter().map(|m| m.name.clone()).collect();
    let fallback_cfg = llm.fallback_router().get_config().await;
    let configured_cloud_providers = locus_llm::KeyringStore::list_configured_providers()
        .into_iter()
        .filter(|p| p.is_configured)
        .map(|p| p.provider_id)
        .collect();

    let ai_diagnostic = AiEngineDiagnosticDto {
        selected_model: None,
        local_models_count: local_models_detected.len(),
        local_models_detected,
        fallback_strategy: format!("{:?}", fallback_cfg.strategy),
        fallback_enabled: fallback_cfg.enabled,
        fallback_targets: fallback_cfg.targets.iter().filter(|t| t.enabled).map(|t| t.id.clone()).collect(),
        configured_cloud_providers,
    };

    // 4. Mesh status
    let net = state.network.read().await;
    let mesh_diagnostic = if let Some(ref n) = *net {
        MeshDiagnosticDto {
            is_running: true,
            discovered_peer_count: n.discover_devices().await.len(),
        }
    } else {
        MeshDiagnosticDto {
            is_running: false,
            discovered_peer_count: 0,
        }
    };

    // 5. Active agents
    let active_agents = state.agents.list_active_agents();
    let agents_diagnostic = AgentsDiagnosticDto {
        active_processes_count: active_agents.len(),
        max_memory_ceiling_mb: 256,
    };

    // 6. Sanitized log traces
    let raw_logs = vec![
        ("INFO", "system", format!("LOCUS Engine v0.1.0 initialized on {} {}", os_str, arch_str)),
        ("INFO", "storage", format!("Workspace index contains {} files ({} bytes)", ws_diagnostic.total_indexed_files, ws_diagnostic.total_size_bytes)),
        ("INFO", "vector", format!("Semantic vector embeddings index initialized with 384-d feature hashing")),
        ("INFO", "security", format!("Keyring store accessed with hardware credential manager")),
        ("INFO", "fallback", format!("Auto-Fallback router configured with strategy: {}", ai_diagnostic.fallback_strategy)),
    ];

    let sanitized_diagnostic_logs: Vec<DiagnosticLogEntry> = raw_logs
        .into_iter()
        .map(|(lvl, sub, msg)| DiagnosticLogEntry {
            timestamp: now.to_rfc3339(),
            level: lvl.to_string(),
            subsystem: sub.to_string(),
            message: sanitize_text(&msg),
        })
        .collect();

    Ok(DiagnosticReport {
        report_id,
        generated_at: now.to_rfc3339(),
        locus_version: "v0.1.0-alpha".to_string(),
        system_environment: SystemEnvironmentDto {
            os: os_str,
            arch: arch_str,
            family: family_str,
            logical_cpu_cores: logical_cpus,
        },
        workspace_status: ws_diagnostic,
        ai_engine_status: ai_diagnostic,
        p2p_mesh_status: mesh_diagnostic,
        agents_pool_status: agents_diagnostic,
        sanitized_diagnostic_logs,
    })
}

#[tauri::command]
pub async fn system_export_diagnostics(
    state: State<'_, AppState>,
) -> Result<ExportDiagnosticResult, String> {
    let report = system_get_diagnostics(state).await?;
    let json_str = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Serialization error: {}", e))?;

    let file_name = format!(
        "locus-diagnostics-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );

    let summary = format!(
        "Diagnostic package created (ID: {}, {} indexed files, {} local models, OS: {} {})",
        &report.report_id[..8],
        report.workspace_status.total_indexed_files,
        report.ai_engine_status.local_models_count,
        report.system_environment.os,
        report.system_environment.arch,
    );

    info!("Exported sanitized diagnostics bundle: {}", file_name);

    Ok(ExportDiagnosticResult {
        success: true,
        file_name,
        json_payload: json_str,
        summary,
    })
}

static GLOBAL_COMPILER_PROBE: once_cell::sync::Lazy<locus_core::CompilerProbeEngine> =
    once_cell::sync::Lazy::new(locus_core::CompilerProbeEngine::new);

#[tauri::command]
pub async fn diagnostics_run_probe(
    workspace_root: String,
) -> Result<Vec<locus_core::DiagnosticItem>, String> {
    let p = std::path::PathBuf::from(workspace_root);
    GLOBAL_COMPILER_PROBE.probe_workspace(&p).await
}

#[tauri::command]
pub fn diagnostics_get_active_feed() -> Result<Vec<locus_core::DiagnosticItem>, String> {
    Ok(GLOBAL_COMPILER_PROBE.store().read().get_all_diagnostics())
}

