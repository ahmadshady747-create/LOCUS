use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityLevel {
    Safe,
    ReviewRequired,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub hash: String,
    pub language: Option<String>,
    pub symbols: Vec<SymbolInfo>,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Class,
    Interface,
    Type,
    Const,
    Module,
    Variable,
    Method,
    Field,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    pub root: PathBuf,
    pub files: HashMap<PathBuf, FileMetadata>,
    pub updated_at: DateTime<Utc>,
    pub total_files: usize,
    pub total_size: u64,
}

impl WorkspaceIndex {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: HashMap::new(),
            updated_at: Utc::now(),
            total_files: 0,
            total_size: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationOp {
    Insert { line: usize, column: usize, text: String },
    Delete { start_line: usize, start_column: usize, end_line: usize, end_column: usize },
    Replace { start_line: usize, start_column: usize, end_line: usize, end_column: usize, text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub context: String,
    pub match_type: SearchMatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchMatchType {
    Exact,
    Fuzzy,
    Regex,
    Symbol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: PathBuf,
    pub content: String,
    pub encoding: String,
    pub line_count: usize,
    pub is_binary: bool,
}

impl FileContent {
    pub fn new(path: PathBuf, content: String) -> Self {
        let line_count = content.lines().count();
        let is_binary = content.as_bytes().iter().any(|&b| b == 0);
        Self {
            path,
            content,
            encoding: "utf-8".to_string(),
            line_count,
            is_binary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: String,
    pub description: String,
    pub default: Option<serde_json::Value>,
    pub validation: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub variables: Vec<TemplateVariable>,
    pub dependencies: Vec<String>,
    pub source: TemplateSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateSource {
    Local { path: PathBuf },
    Git { url: String, rev: Option<String> },
    Registry { name: String, version: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub info: TemplateInfo,
    pub files: Vec<TemplateFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub path: String,
    pub content: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSpec {
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub variables: Vec<TemplateVariable>,
    pub files: Vec<TemplateFile>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub user_prompt: String,
    pub intent: Option<String>,
    pub file_refs: Vec<PathBuf>,
    pub template_refs: Vec<String>,
    pub max_tokens: usize,
    pub include_git_history: bool,
    pub conversation_history: Vec<ConversationTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: PathBuf,
    pub content: String,
    pub relevance_score: f32,
    pub symbols: Vec<SymbolInfo>,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    pub system_prompt: String,
    pub user_prompt: String,
    pub template_context: String,
    pub file_context: Vec<ContextFile>,
    pub conversation_context: Vec<ConversationTurn>,
    pub metadata: ContextMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub total_tokens: usize,
    pub template_tokens: usize,
    pub file_tokens: usize,
    pub conversation_tokens: usize,
    pub truncated: bool,
    pub model_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub id: Uuid,
    pub name: String,
    pub models: Vec<ModelInfo>,
    pub max_context: usize,
    pub vram_gb: Option<f32>,
    pub quantization: Vec<String>,
    pub performance_score: f32,
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub quantization: String,
    pub context_window: usize,
    pub parameter_count: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: Uuid,
    pub name: String,
    pub capabilities: NodeCapabilities,
    pub last_seen: DateTime<Utc>,
    pub status: PeerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerStatus {
    Online,
    Busy,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMTask {
    pub id: Uuid,
    pub prompt: String,
    pub model_preference: Option<String>,
    pub temperature: f32,
    pub max_tokens: usize,
    pub stream: bool,
    pub context: Option<AssembledContext>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub id: Uuid,
    pub response: String,
    pub tokens_used: usize,
    pub latency_ms: u64,
    pub peer_id: Uuid,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerEvent {
    Discovered(PeerInfo),
    Lost(Uuid),
    StatusChanged { peer_id: Uuid, status: PeerStatus },
    TaskAssigned { task_id: Uuid, peer_id: Uuid },
    TaskCompleted { task_id: Uuid, peer_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: Uuid,
    pub image: AgentImage,
    pub env: HashMap<String, String>,
    pub mounts: Vec<MountSpec>,
    pub limits: ResourceLimits,
    pub network: NetworkMode,
    pub working_dir: PathBuf,
    pub entrypoint: Option<Vec<String>>,
    pub profile: AgentProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentImage {
    Python { version: String },
    Node { version: String },
    Rust { version: String },
    Go { version: String },
    Shell,
    Custom { image: String },
    Docker { image: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountSpec {
    pub source: PathBuf,
    pub target: PathBuf,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_cores: Option<f32>,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub timeout_sec: u64,
    pub pids_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMode {
    None,
    Localhost,
    Host,
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentProfile {
    Minimal,
    Development,
    Testing,
    Full,
    Custom(String),
}

impl Default for AgentProfile {
    fn default() -> Self {
        AgentProfile::Development
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandle {
    pub id: Uuid,
    pub spec: AgentSpec,
    pub status: AgentStatus,
    pub pid: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub peak_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub uptime_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputEvent {
    Stdout(String),
    Stderr(String),
    Exit(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedFileChange {
    pub change_id: String,
    pub file_path: PathBuf,
    pub original_content: String,
    pub proposed_content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffHunk {
    pub hunk_id: String,
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub snapshot_id: String,
    pub created_at: DateTime<Utc>,
    pub file_path: PathBuf,
    pub previous_content: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub success: boolean_or_bool::Bool,
    pub snapshot_id: String,
    pub file_path: PathBuf,
    pub restored_bytes: usize,
    pub message: String,
}

mod boolean_or_bool {
    pub type Bool = bool;
}