//! Architectural Decision Records (ADR) & Negative Memory Ledger
//!
//! Manages `.locus/adr.json` in the active workspace root, storing architectural decisions
//! and negative memory entries (past failed solutions, known anti-patterns, rejected approaches)
//! to automatically inject guardrail warnings into agent prompt contexts.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Accepted,
    Rejected,
    Deprecated,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NegativeSeverity {
    Warning,
    Forbidden,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdrRecord {
    pub id: String,
    pub title: String,
    pub status: DecisionKind,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NegativeMemoryEntry {
    pub id: String,
    pub pattern_name: String,
    pub severity: NegativeSeverity,
    pub target_module: String,
    pub reason: String,
    pub forbidden_snippets: Vec<String>,
    pub recommended_alternative: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdrLedger {
    pub records: Vec<AdrRecord>,
    pub negative_memories: Vec<NegativeMemoryEntry>,
}

pub struct AdrLedgerManager;

impl AdrLedgerManager {
    const ADR_FILE_NAME: &'static str = "adr.json";
    const LOCUS_DIR: &'static str = ".locus";

    fn get_adr_path(workspace_root: &Path) -> PathBuf {
        workspace_root.join(Self::LOCUS_DIR).join(Self::ADR_FILE_NAME)
    }

    /// Loads the ADR ledger from .locus/adr.json or creates a default one if absent
    pub fn load_or_create(workspace_root: &Path) -> Result<AdrLedger> {
        let adr_path = Self::get_adr_path(workspace_root);

        if adr_path.exists() {
            let content = fs::read_to_string(&adr_path)
                .with_context(|| format!("Failed to read ADR ledger at {:?}", adr_path))?;
            let ledger: AdrLedger = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse ADR ledger JSON at {:?}", adr_path))?;
            return Ok(ledger);
        }

        // Initialize default ledger with foundation decisions and negative memories
        let default_ledger = Self::build_default_ledger();
        let locus_dir = workspace_root.join(Self::LOCUS_DIR);
        fs::create_dir_all(&locus_dir)
            .with_context(|| format!("Failed to create .locus directory at {:?}", locus_dir))?;

        let serialized = serde_json::to_string_pretty(&default_ledger)?;
        fs::write(&adr_path, serialized)
            .with_context(|| format!("Failed to initialize ADR file at {:?}", adr_path))?;

        Ok(default_ledger)
    }

    /// Persists the ADR ledger to disk atomically
    pub fn save(workspace_root: &Path, ledger: &AdrLedger) -> Result<()> {
        let locus_dir = workspace_root.join(Self::LOCUS_DIR);
        fs::create_dir_all(&locus_dir)?;

        let adr_path = Self::get_adr_path(workspace_root);
        let tmp_path = adr_path.with_extension("tmp");

        let serialized = serde_json::to_string_pretty(ledger)?;
        fs::write(&tmp_path, serialized)?;
        fs::rename(&tmp_path, &adr_path)?;

        Ok(())
    }

    /// Injects negative memory warnings for a specific target module or file
    pub fn inject_negative_memory_warnings(
        ledger: &AdrLedger,
        target_module: &str,
    ) -> Vec<String> {
        let target_lower = target_module.to_lowercase();
        let mut warnings = Vec::new();

        for entry in &ledger.negative_memories {
            let entry_mod_lower = entry.target_module.to_lowercase();
            let matches = entry.target_module == "*"
                || target_lower.contains(&entry_mod_lower)
                || entry_mod_lower.contains(&target_lower);

            if matches {
                let severity_badge = match entry.severity {
                    NegativeSeverity::Critical => "⛔ CRITICAL FORBIDDEN PATTERN",
                    NegativeSeverity::Forbidden => "🚫 FORBIDDEN ANTI-PATTERN",
                    NegativeSeverity::Warning => "⚠️ ARCHITECTURAL WARNING",
                };

                warnings.push(format!(
                    "[{}] '{}' in '{}': {}. ALTERNATIVE: {}",
                    severity_badge,
                    entry.pattern_name,
                    entry.target_module,
                    entry.reason,
                    entry.recommended_alternative
                ));
            }
        }

        warnings
    }

    fn build_default_ledger() -> AdrLedger {
        let now = Utc::now();
        AdrLedger {
            records: vec![
                AdrRecord {
                    id: "ADR-001".to_string(),
                    title: "Local-First Ephemeral Task Orchestration".to_string(),
                    status: DecisionKind::Accepted,
                    context: "LOCUS requires sovereign local execution with zero cloud telemetry requirements.".to_string(),
                    decision: "Use local Kahn DAG task graphs and ephemeral sandboxed agent workers.".to_string(),
                    consequences: vec![
                        "Total privacy preserved".to_string(),
                        "Zero cloud dependency for core workflows".to_string(),
                    ],
                    created_at: now,
                    tags: vec!["architecture".to_string(), "orchestration".to_string()],
                },
            ],
            negative_memories: vec![
                NegativeMemoryEntry {
                    id: "NEG-001".to_string(),
                    pattern_name: "Direct Mutex Across Async Await Boundary".to_string(),
                    severity: NegativeSeverity::Critical,
                    target_module: "crates/locus-".to_string(),
                    reason: "Synchronous Mutex guard held across .await yields deadlock in tokio runtime".to_string(),
                    forbidden_snippets: vec!["let _guard = mutex.lock(); ... .await".to_string()],
                    recommended_alternative: "Use tokio::sync::Mutex or scope synchronous lock before .await".to_string(),
                    created_at: now,
                },
                NegativeMemoryEntry {
                    id: "NEG-002".to_string(),
                    pattern_name: "Direct React State Array Mutation".to_string(),
                    severity: NegativeSeverity::Forbidden,
                    target_module: "src/src/components".to_string(),
                    reason: "In-place mutation (state.push) bypasses React reconciler re-renders".to_string(),
                    forbidden_snippets: vec!["state.push(...)".to_string(), "state.splice(...)".to_string()],
                    recommended_alternative: "Use immutable spread syntax setState(prev => [...prev, item])".to_string(),
                    created_at: now,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_adr_ledger_load_and_save() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // 1. Load creates default .locus/adr.json
        let mut ledger = AdrLedgerManager::load_or_create(path).unwrap();
        assert_eq!(ledger.records.len(), 1);
        assert_eq!(ledger.negative_memories.len(), 2);

        // 2. Add custom ADR record and save
        ledger.records.push(AdrRecord {
            id: "ADR-002".to_string(),
            title: "Custom Caching Layer".to_string(),
            status: DecisionKind::Accepted,
            context: "Fast caching".to_string(),
            decision: "Use memory map".to_string(),
            consequences: vec!["High speed".to_string()],
            created_at: Utc::now(),
            tags: vec!["cache".to_string()],
        });

        AdrLedgerManager::save(path, &ledger).unwrap();

        // 3. Reload and verify persistence
        let reloaded = AdrLedgerManager::load_or_create(path).unwrap();
        assert_eq!(reloaded.records.len(), 2);
        assert_eq!(reloaded.records[1].id, "ADR-002");
    }

    #[test]
    fn test_negative_memory_warning_matching() {
        let ledger = AdrLedgerManager::build_default_ledger();

        // Match components module
        let warnings_ui = AdrLedgerManager::inject_negative_memory_warnings(&ledger, "src/src/components/MissionControl.tsx");
        assert_eq!(warnings_ui.len(), 1);
        assert!(warnings_ui[0].contains("Direct React State Array Mutation"));

        // Match crates module
        let warnings_rust = AdrLedgerManager::inject_negative_memory_warnings(&ledger, "crates/locus-agents/src/task.rs");
        assert_eq!(warnings_rust.len(), 1);
        assert!(warnings_rust[0].contains("Direct Mutex Across Async Await Boundary"));

        // Non-matching module
        let warnings_doc = AdrLedgerManager::inject_negative_memory_warnings(&ledger, "docs/README.md");
        assert_eq!(warnings_doc.len(), 0);
    }
}
