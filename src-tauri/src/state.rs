use locus_agents::EphemeralAgentManager;
use locus_context::{ContextAssembler, EmbeddingConfig, SemanticIndexer};
use locus_fs::FileSystemEngine;
use locus_llm::LlmClient;
use locus_network::NetworkOrchestrator;
use locus_templates::TemplateStore;
use tokio_stream::wrappers::BroadcastStream;
use locus_core::types::FileEvent;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub fs_engine: Arc<RwLock<FileSystemEngine>>,
    pub template_store: Arc<TemplateStore>,
    pub context_assembler: Arc<ContextAssembler>,
    pub semantic_indexer: Arc<SemanticIndexer>,
    pub network: Arc<RwLock<Option<NetworkOrchestrator>>>,
    pub agents: Arc<EphemeralAgentManager>,
    pub skills: Arc<locus_agents::SkillRegistry>,
    pub skill_bridge: Arc<locus_llm::SkillBridge>,
    pub llm: Arc<RwLock<LlmClient>>,
    pub workspace_root: Arc<RwLock<Option<PathBuf>>>,
    pub hybrid_enabled: Arc<RwLock<bool>>,
    pub file_watcher: Arc<RwLock<Option<BroadcastStream<FileEvent>>>>,
}

impl AppState {
    pub async fn new(workspace_root: Option<PathBuf>) -> Self {
        let root = workspace_root.unwrap_or_else(|| PathBuf::from("."));

        let fs_engine = FileSystemEngine::new(
            root.clone(),
            vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "dist".to_string(),
                ".locus".to_string(),
            ],
        )
        .expect("Failed to create FileSystemEngine");

        let template_store = TemplateStore::new().expect("Failed to load templates");
        let context_assembler =
            ContextAssembler::new().expect("Failed to create ContextAssembler");
        let semantic_indexer = SemanticIndexer::new(EmbeddingConfig::default());
        let agents = EphemeralAgentManager::new();
        let llm = LlmClient::new().await.expect("Failed to create LLM Client");

        let mut network = None;
        if let Ok(net) = NetworkOrchestrator::new(
            None,
            locus_network::DeviceType::Hybrid,
            locus_network::DeviceCapabilities::default(),
            8080,
        )
        .await
        {
            network = Some(net);
        }

        let skills_registry = Arc::new(locus_agents::SkillRegistry::new(Some(root.clone())));
        let skill_bridge = Arc::new(locus_llm::SkillBridge::new(skills_registry.clone()));

        Self {
            fs_engine: Arc::new(RwLock::new(fs_engine)),
            template_store: Arc::new(template_store),
            context_assembler: Arc::new(context_assembler),
            semantic_indexer: Arc::new(semantic_indexer),
            network: Arc::new(RwLock::new(network)),
            agents: Arc::new(agents),
            skills: skills_registry,
            skill_bridge,
            llm: Arc::new(RwLock::new(llm)),
            workspace_root: Arc::new(RwLock::new(Some(root))),
            hybrid_enabled: Arc::new(RwLock::new(false)),
            file_watcher: Arc::new(RwLock::new(None)),
        }
    }
}
