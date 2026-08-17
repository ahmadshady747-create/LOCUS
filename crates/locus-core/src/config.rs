use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::types::AgentProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocusConfig {
    pub workspace: WorkspaceConfig,
    pub templates: TemplatesConfig,
    pub network: NetworkConfig,
    pub agents: AgentsConfig,
    pub context: ContextConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    pub ignore: Vec<String>,
    pub watch: bool,
    pub index_symbols: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatesConfig {
    pub paths: Vec<PathBuf>,
    pub auto_discover: bool,
    pub builtin_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub service_name: String,
    pub advertise_local: bool,
    pub preferred_model: Option<String>,
    pub port: u16,
    pub discovery_interval_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub default_profile: AgentProfile,
    pub profiles: HashMap<String, AgentProfileConfig>,
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileConfig {
    pub memory_mb: u64,
    pub timeout_sec: u64,
    pub network: bool,
    pub cpu_cores: Option<f32>,
    pub disk_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_tokens: usize,
    pub compression_threshold: f32,
    pub include_git_history: bool,
    pub symbol_weight: f32,
    pub recency_weight: f32,
}

impl Default for LocusConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("minimal".to_string(), AgentProfileConfig {
            memory_mb: 100,
            timeout_sec: 10,
            network: false,
            cpu_cores: Some(0.5),
            disk_mb: 50,
        });
        profiles.insert("development".to_string(), AgentProfileConfig {
            memory_mb: 2048,
            timeout_sec: 300,
            network: true,
            cpu_cores: Some(2.0),
            disk_mb: 1024,
        });
        profiles.insert("testing".to_string(), AgentProfileConfig {
            memory_mb: 1024,
            timeout_sec: 120,
            network: false,
            cpu_cores: Some(1.0),
            disk_mb: 512,
        });
        profiles.insert("full".to_string(), AgentProfileConfig {
            memory_mb: 4096,
            timeout_sec: 600,
            network: true,
            cpu_cores: Some(4.0),
            disk_mb: 2048,
        });

        Self {
            workspace: WorkspaceConfig {
                root: PathBuf::from("."),
                ignore: vec![
                    ".git".to_string(),
                    "target".to_string(),
                    "node_modules".to_string(),
                    "dist".to_string(),
                    ".locus".to_string(),
                ],
                watch: true,
                index_symbols: true,
            },
            templates: TemplatesConfig {
                paths: vec![
                    PathBuf::from("~/.locus/templates"),
                    PathBuf::from(".locus/templates"),
                ],
                auto_discover: true,
                builtin_enabled: true,
            },
            network: NetworkConfig {
                enabled: true,
                service_name: "_locus-llm._tcp.local.".to_string(),
                advertise_local: true,
                preferred_model: Some("llama3.1:8b".to_string()),
                port: 0,
                discovery_interval_sec: 30,
            },
            agents: AgentsConfig {
                default_profile: AgentProfile::Development,
                profiles,
                max_concurrent: 4,
            },
            context: ContextConfig {
                max_tokens: 32000,
                compression_threshold: 0.8,
                include_git_history: false,
                symbol_weight: 0.7,
                recency_weight: 0.3,
            },
        }
    }
}

impl LocusConfig {
    pub fn load(path: Option<&PathBuf>) -> crate::Result<Self> {
        let default_path = PathBuf::from("locus.toml");
        let config_path = path.unwrap_or(&default_path);
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let mut config: LocusConfig = toml::from_str(&content)?;
            config.expand_paths();
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &PathBuf) -> crate::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn expand_paths(&mut self) {
        let expand = |p: &mut PathBuf| {
            if p.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    *p = home.join(p.strip_prefix("~").unwrap());
                }
            }
        };

        expand(&mut self.workspace.root);
        for p in &mut self.templates.paths {
            expand(p);
        }
        for (_, profile) in &mut self.agents.profiles {
            // profiles don't have paths to expand
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            ignore: vec![],
            watch: true,
            index_symbols: true,
        }
    }
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            paths: vec![],
            auto_discover: true,
            builtin_enabled: true,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "_locus-llm._tcp.local.".to_string(),
            advertise_local: true,
            preferred_model: None,
            port: 0,
            discovery_interval_sec: 30,
        }
    }
}

impl Default for AgentsConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("minimal".to_string(), AgentProfileConfig {
            memory_mb: 100,
            timeout_sec: 10,
            network: false,
            cpu_cores: Some(0.5),
            disk_mb: 50,
        });
        profiles.insert("development".to_string(), AgentProfileConfig {
            memory_mb: 2048,
            timeout_sec: 300,
            network: true,
            cpu_cores: Some(2.0),
            disk_mb: 1024,
        });
        Self {
            default_profile: AgentProfile::Development,
            profiles,
            max_concurrent: 4,
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 32000,
            compression_threshold: 0.8,
            include_git_history: false,
            symbol_weight: 0.7,
            recency_weight: 0.3,
        }
    }
}