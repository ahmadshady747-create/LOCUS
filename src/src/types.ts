export interface FileMetadata {
  path: string;
  size: number;
  modified: string;
  hash: string;
  language: string | null;
  symbols: SymbolInfo[];
  is_binary: boolean;
}

export interface SymbolInfo {
  name: string;
  kind: string;
  line: number;
  column: number;
  signature: string | null;
}

export interface WorkspaceIndex {
  root: string;
  files: Record<string, FileMetadata>;
  updated_at: string;
  total_files: number;
  total_size: number;
}

export interface FileContent {
  path: string;
  content: string;
  encoding: string;
  line_count: number;
  is_binary: boolean;
}

export interface SearchResult {
  path: string;
  line: number;
  column: number;
  context: string;
  match_type: string;
}

export interface Template {
  id: string;
  category: string;
  name: string;
  description: string;
  code: string;
  language: string;
  security_level: "Safe" | "ReviewRequired" | "Dangerous";
  tags: string[];
  dependencies: string[];
  version: string;
}

export interface LocalDevice {
  id: string;
  name: string;
  hostname: string;
  ip_address: string;
  port: number;
  capabilities: DeviceCapabilities;
  last_seen: string;
  status: "Online" | "Busy" | "Offline" | "Unknown";
  device_type: "Main" | "Worker" | "Hybrid";
}

export interface LocalDeviceSimple {
  id: string;
  name: string;
  hostname: string;
  ip_address: string;
  port: number;
  status: string;
  device_type: string;
  models: string[];
  vram_gb: number | null;
  specializations: string[];
  performance_score?: number;
  capabilities?: {
    models: { name: string }[];
    specializations: string[];
    performance_score: number;
  };
}

export interface DeviceCapabilities {
  models: ModelInfo[];
  max_context_tokens: number;
  vram_gb: number | null;
  quantization: string[];
  cpu_cores: number;
  memory_gb: number;
  supports_gpu: boolean;
  specializations: string[];
  performance_score: number;
}

export interface ModelInfo {
  name: string;
  quantization: string;
  context_window: number;
  parameter_count: string;
  size_gb: number;
}

export interface LocalModel {
  name: string;
  size: string;
  digest: string;
  details: {
    format: string;
    family: string;
    families: string[] | null;
    parameter_size: string;
    quantization_level: string;
    parent_model: string | null;
  };
  modified_at: string;
  backend: "Ollama" | "LlamaCpp";
}

export interface AgentHandle {
  id: string;
  task_id: string;
  status: string;
  pid: number | null;
  started_at: string | null;
  completed_at: string | null;
  config: {
    memory_limit_mb: number;
    cpu_limit: number | null;
    timeout_seconds: number;
    network_allowed: boolean;
    read_only_fs: boolean;
    allowed_paths: string[];
    blocked_syscalls: string[];
  };
}

export interface AgentStats {
  cpu_percent: number;
  memory_mb: number;
  memory_peak_mb: number;
  disk_read_mb: number;
  disk_write_mb: number;
  network_rx_mb: number;
  network_tx_mb: number;
  uptime_seconds: number;
  thread_count: number;
}

export interface TaskResult {
  success: boolean;
  output: string;
  errors: string[];
  duration_ms: number;
  peak_memory_mb: number;
  exit_code: number | null;
  test_results: { passed: number; failed: number; output: string } | null;
}

export interface ModelSelection {
  model_name: string;
  backend: string;
  estimated_vram_mb: number;
  estimated_ram_mb: number;
  fits_in_vram: boolean;
  quantization: string;
  reasoning: string;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  status?: "pending" | "done" | "error";
  provider_stamp?: string;
  model_used?: string;
  latency_ms?: number;
  was_fallback?: boolean;
  fallback_reason?: string;
}

export type PrivacyMode = "local" | "hybrid";

export interface AppState {
  workspaceRoot: string | null;
  workspace: WorkspaceIndex | null;
  models: LocalModel[];
  devices: LocalDeviceSimple[];
  activeAgents: AgentHandle[];
  selectedModel: string | null;
  privacyMode: PrivacyMode;
  hybridPercent: number;
}

export interface SemanticSearchResult {
  document_id: string;
  file_path: string;
  symbol_name: string | null;
  symbol_kind: string;
  snippet: string;
  line_start: number;
  line_end: number;
  similarity: number;
  language: string | null;
  tags: string[];
}

export interface StagedFileChange {
  change_id: string;
  file_path: string;
  original_content: string;
  proposed_content: string;
  created_at: string;
}

export type DiffLineType = "Context" | "Addition" | "Deletion";

export interface DiffLine {
  line_type: DiffLineType;
  content: string;
  old_line_no: number | null;
  new_line_no: number | null;
}

export interface DiffHunk {
  hunk_id: string;
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: DiffLine[];
}

export interface FileSnapshot {
  snapshot_id: string;
  created_at: string;
  file_path: string;
  previous_content: string;
  description: string;
}

export interface RollbackResult {
  success: boolean;
  snapshot_id: string;
  file_path: string;
  restored_bytes: number;
  message: string;
}

export interface KeySlotStatus {
  key_masked: string;
  is_active: boolean;
  in_cooldown: boolean;
  cooldown_remaining_secs: number;
  request_count: number;
}

export interface ProviderStatus {
  provider_id: string;
  name: string;
  is_configured: boolean;
  default_model: string;
  supports_custom_url: boolean;
  pool_size?: number;
  active_keys_count?: number;
  keys?: KeySlotStatus[];
}

export interface SkeletonStats {
  original_chars: number;
  skeleton_chars: number;
  original_tokens_est: number;
  skeleton_tokens_est: number;
  tokens_saved_est: number;
  reduction_percentage: number;
}

export interface ExtractSkeletonResponse {
  skeleton: string;
  stats: SkeletonStats;
}

export interface ApplySearchReplaceResult {
  success: boolean;
  applied_blocks_count: number;
  new_content: string;
}

export interface ProviderTestResult {
  success: boolean;
  provider_id: string;
  message: string;
  latency_ms: number;
  available_models: string[];
}

export type FallbackStrategy = "LocalFirst" | "CloudFirst" | "SpeedFirst" | "CustomOrder";

export interface FallbackTarget {
  id: string;
  label: string;
  is_local: boolean;
  enabled: boolean;
  preferred_model: string | null;
}

export interface FallbackChainConfig {
  enabled: boolean;
  strategy: FallbackStrategy;
  targets: FallbackTarget[];
  max_retries_per_target: number;
  timeout_seconds: number;
}

export interface DiagnosticReport {
  report_id: string;
  generated_at: string;
  locus_version: string;
  system_environment: {
    os: string;
    arch: string;
    family: string;
    logical_cpu_cores: number;
  };
  workspace_status: {
    has_workspace_loaded: boolean;
    total_indexed_files: number;
    total_size_bytes: number;
  };
  ai_engine_status: {
    selected_model: string | null;
    local_models_count: number;
    local_models_detected: string[];
    fallback_strategy: string;
    fallback_enabled: boolean;
    fallback_targets: string[];
    configured_cloud_providers: string[];
  };
  p2p_mesh_status: {
    is_running: boolean;
    discovered_peer_count: number;
  };
  agents_pool_status: {
    active_processes_count: number;
    max_memory_ceiling_mb: number;
  };
  sanitized_diagnostic_logs: Array<{
    timestamp: string;
    level: string;
    subsystem: string;
    message: string;
  }>;
}

export interface ExportDiagnosticResult {
  success: boolean;
  file_name: string;
  json_payload: string;
  summary: string;
}

export interface DetectedKeyReport {
  provider_id: string;
  provider_name: string;
  source: string;
  key_masked: string;
  imported: boolean;
  message: string;
}

// ---- Skills Engine Types ----

export interface SkillPermissionsDto {
  allow_network: boolean;
  allow_fs_read: boolean;
  allow_fs_write: boolean;
  env_whitelist: string[];
}

export interface SkillDto {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string | null;
  runtime: "wasm" | "script";
  entrypoint: string;
  permissions: SkillPermissionsDto;
  parameters: Record<string, any>;
  enabled: boolean;
  timeout_seconds: number;
  location_type: "workspace" | "global";
  dir_path: string;
  is_valid: boolean;
  load_error: string | null;
}

export interface SkillExecutionResultDto {
  success: boolean;
  stdout: string;
  stderr: string;
  parsed_json: any | null;
  exit_code: number | null;
  duration_ms: number;
  is_timeout: boolean;
  error: string | null;
}

export interface CreateSkillRequest {
  id: string;
  name: string;
  runtime: "wasm" | "script";
  language: string;
  description: string;
  target_in_workspace: boolean;
}

// === Mission Control & Task Graph DAG ===

export type TaskNodeType = "code_edit" | "shell_command" | "create_file" | "skill_execution" | "analysis" | "test";
export type TaskNodeStatus = "pending" | "ready" | "running" | "completed" | "failed" | "skipped";
export type TaskGraphStatus = "draft" | "planning" | "in_progress" | "paused" | "completed" | "failed";

export interface TaskActionPayload {
  target_file?: string;
  proposed_content?: string;
  search_replace_block?: string;
  shell_command?: string;
  skill_name?: string;
  skill_params?: any;
}

export interface TaskNodeResult {
  success: boolean;
  output: string;
  diff_preview?: string;
  error?: string;
  duration_ms: number;
}

export interface TaskNode {
  id: string;
  title: string;
  description: string;
  node_type: TaskNodeType;
  dependencies: string[];
  status: TaskNodeStatus;
  payload: TaskActionPayload;
  result?: TaskNodeResult;
  auto_execute: boolean;
  created_at: string;
  updated_at: string;
}

export interface TaskGraph {
  id: string;
  goal: string;
  nodes: TaskNode[];
  status: TaskGraphStatus;
  created_at: string;
  updated_at: string;
}

export interface ClipboardAnalysis {
  hasCode: boolean;
  language: string;
  lineCount: number;
  content: string;
  preview: string;
}

// === Specification Alignment & Tradeoff Gate ===

export type TradeoffCategory =
  | "state_management"
  | "persistence"
  | "concurrency"
  | "network_transport"
  | "error_strategy";

export interface SpecTradeoffOption {
  id: string;
  title: string;
  description: string;
  pros: string[];
  cons: string[];
  recommended: boolean;
}

export interface SpecAmbiguity {
  id: string;
  category: TradeoffCategory;
  question: string;
  options: SpecTradeoffOption[];
  selected_option_id: string | null;
}

export interface SpecAlignmentReport {
  goal: string;
  has_ambiguity: boolean;
  ambiguities: SpecAmbiguity[];
  aligned_constraints: string[];
}

// === Adversarial QA Agent ===

export type QaRiskSeverity = "low" | "medium" | "high" | "critical";

export interface QaRiskItem {
  rule: string;
  severity: QaRiskSeverity;
  line_number?: number;
  description: string;
  suggested_fix: string;
}

export interface FuzzTestCase {
  input_name: string;
  input_value: string;
  expected_behavior: string;
}

export interface QaReport {
  score: number;
  is_approved: boolean;
  risks: QaRiskItem[];
  fuzz_cases: FuzzTestCase[];
  summary: string;
}

// === ADR & Negative Memory Ledger ===

export type DecisionKind = "accepted" | "rejected" | "deprecated" | "superseded";
export type NegativeSeverity = "warning" | "forbidden" | "critical";

export interface AdrRecord {
  id: string;
  title: string;
  status: DecisionKind;
  context: string;
  decision: string;
  consequences: string[];
  created_at: string;
  tags: string[];
}

export interface NegativeMemoryEntry {
  id: string;
  pattern_name: string;
  severity: NegativeSeverity;
  target_module: string;
  reason: string;
  forbidden_snippets: string[];
  recommended_alternative: string;
  created_at: string;
}

export interface AdrLedger {
  records: AdrRecord[];
  negative_memories: NegativeMemoryEntry[];
}

// === GitHub OAuth Device Flow & Git Sync ===

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface GitHubUser {
  login: string;
  id: number;
  avatar_url: string;
  name: string | null;
  email: string | null;
  public_repos: number;
  html_url: string;
}

export interface GitHubRepo {
  id: number;
  name: string;
  full_name: string;
  description: string | null;
  html_url: string;
  clone_url: string;
  private: boolean;
  default_branch: string;
  stargazers_count: number;
}

export interface GitHubAuthStatus {
  is_authenticated: boolean;
  user: GitHubUser | null;
  token_preview: string | null;
  error: string | null;
}

export type DeviceFlowPollStatus =
  | { pending: null }
  | { slow_down: number }
  | { expired: null }
  | { denied: null }
  | { success: string }
  | { error: string };

export interface GitStatusReport {
  branch: string;
  has_staged_changes: boolean;
  has_unstaged_changes: boolean;
  staged_files: string[];
  modified_files: string[];
  untracked_files: string[];
  ahead_commits: number;
  behind_commits: number;
}

export interface SmartCommitResult {
  commit_hash: string;
  commit_message: string;
  pushed: boolean;
  files_committed: number;
}

export interface PullRequestResult {
  pr_url: string;
  pr_number: number;
  title: string;
  state: string;
  html_url: string;
}

export interface GitCloneOptions {
  repo_url: string;
  target_dir: string;
  branch?: string;
  depth?: number;
}

export interface CreatePrRequest {
  auth_token: string;
  repo_owner: string;
  repo_name: string;
  title: string;
  body: string;
  base: string;
  head: string;
}

// === Cognitive Router & Dynamic Cost-to-Power Matrix ===

export type CognitiveTaskComplexity = "micro" | "standard" | "architectural";
export type BudgetStrategy = "max_speed" | "balanced" | "max_power";
export type CostTier = "free" | "low" | "high";

export interface RoutingDecision {
  selected_model: string;
  provider: string;
  complexity: CognitiveTaskComplexity;
  cost_tier: CostTier;
  budget_strategy: BudgetStrategy;
  rationale: string;
}

export interface RouteTaskRequest {
  prompt: string;
  file_count?: number;
  context_tokens?: number;
  strategy?: BudgetStrategy;
}

// === Local Discovery & Streaming Model Puller ===

export interface HardwareProfile {
  total_ram_gb: number;
  available_ram_gb: number;
  cpu_cores: number;
  os: string;
  arch: string;
  has_gpu: boolean;
  gpu_name: string | null;
  vram_gb: number | null;
}

export interface RecommendedModelSpec {
  model_id: string;
  display_name: string;
  parameter_size: string;
  download_size_gb: number;
  min_ram_gb: number;
  recommended_ram_gb: number;
  tier: string;
  rationale: string;
  is_installed: boolean;
}

export interface LocalInferenceEndpoint {
  name: string;
  url: string;
  is_reachable: boolean;
  version: string | null;
  models_count: number;
  installed_models: string[];
}

export interface LocalDiscoveryReport {
  hardware: HardwareProfile;
  recommendation: RecommendedModelSpec;
  endpoints: LocalInferenceEndpoint[];
}

export interface ModelPullProgress {
  job_id: string;
  model_name: string;
  status: string;
  digest: string | null;
  completed_bytes: number;
  total_bytes: number;
  percentage: number;
  speed_mb_per_sec: number;
  eta_seconds: number | null;
  is_done: boolean;
  error: string | null;
}

// === Free Provider Radar & Quota Intelligence ===

export interface FreeProviderInfo {
  id: string;
  name: string;
  badge: string;
  free_tier_limits: string;
  speed_tier: string;
  key_url: string;
  recommended_model: string;
  description: string;
  card_required: boolean;
}

export interface FreeProviderSuggestion {
  provider: FreeProviderInfo;
  potential_token_savings: string;
  is_dismissed: boolean;
}

// === Research & Semantic Docs Extractor ===

export type ResearchEcosystem = "rust" | "typescript" | "python" | "general";

export interface ResearchPackageMetadata {
  name: string;
  version: string;
  description: string;
  repository_url: string | null;
  documentation_url: string | null;
  license: string | null;
  downloads: number | null;
  ecosystem: ResearchEcosystem;
}

export interface DocSearchResult {
  package: ResearchPackageMetadata;
  summary_markdown: string;
  signatures: string[];
  cached: boolean;
  source_url: string;
}

export interface ResolvedErrorSolution {
  error_code: string;
  error_title: string;
  language: string;
  explanation: string;
  recommended_fix_markdown: string;
  negative_memory_pattern: string;
  references: string[];
}

// === Hybrid Context Retrieval (SymbolGraph & BM25) ===

export type SymbolKind =
  | "struct"
  | "function"
  | "trait"
  | "type_alias"
  | "class"
  | "interface"
  | "enum"
  | "constant"
  | "variable"
  | "module";

export interface SymbolNode {
  name: string;
  kind: SymbolKind;
  file_path: string;
  line_number: number;
  signature: string;
  doc_comment: string | null;
}

export interface Bm25SearchResult {
  id: string;
  file_path: string;
  title: string;
  score: number;
  matched_terms: string[];
  snippet: string;
}

export interface HybridContextPayload {
  query: string;
  symbols: SymbolNode[];
  bm25_results: Bm25SearchResult[];
  dense_context: string;
  token_estimate: number;
  latency_ms: number;
}

// === Universal Silent Editor Bridge ===

export interface DetectedEditor {
  id: string;
  name: string;
  executable: string;
  is_installed: boolean;
  is_running: boolean;
}

export interface EditorBridgeStatus {
  connected_editor: DetectedEditor | null;
  active_file: string | null;
  active_line: number | null;
  active_column: number | null;
  last_sync_timestamp: string;
  sync_mode: string;
  detected_editors: DetectedEditor[];
}

export interface EditorSyncReport {
  file_path: string;
  bytes_synced: number;
  atomic_swap: boolean;
  timestamp: string;
  duration_ms: number;
}

// === Zero-Shot Micro-SAST Security Gate ===

export type SecuritySeverity = "info" | "warning" | "critical" | "blocker";

export type SecurityViolationCategory =
  | "secret_leak"
  | "sql_injection"
  | "command_injection"
  | "path_traversal"
  | "undocumented_unsafe"
  | "hardcoded_credentials";

export interface SecurityViolation {
  id: string;
  category: SecurityViolationCategory;
  severity: SecuritySeverity;
  title: string;
  description: string;
  line_number: number | null;
  snippet: string;
  remediation_advice: string;
}

export interface SecurityScanResult {
  is_safe: boolean;
  violations: SecurityViolation[];
  entropy_alerts: number;
  scan_duration_micros: number;
  summary: string;
}

// === Ambient OS Context Engine ===

export type AppCategory =
  | "ide"
  | "terminal"
  | "browser"
  | "database"
  | "design"
  | "document"
  | "other";

export interface ActiveWindowContext {
  app_name: string;
  window_title: string;
  category: AppCategory;
  process_id: number | null;
}

export interface AmbientSnapshot {
  window: ActiveWindowContext;
  selected_text: string | null;
  clipboard_text: string | null;
  timestamp: string;
}

// === Swappable Core Slots Engine ===

export type SlotType = "context" | "sandbox";

export interface SlotDescriptor {
  id: string;
  name: string;
  slot_type: SlotType;
  description: string;
  is_active: boolean;
  is_builtin: boolean;
}

export interface SlotsConfig {
  active_context_driver: string;
  active_sandbox_driver: string;
  descriptors: SlotDescriptor[];
}

export interface ContextSearchResult {
  file_path: string;
  snippet: string;
  score: number;
}

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

// === Zero-Panic Local Tools & Circuit Breaker ===

export interface ToolParameter {
  name: string;
  description: string;
  required: boolean;
  default_value: string | null;
}

export interface LocalToolManifest {
  id: string;
  name: string;
  description: string;
  script_path: string;
  shebang: string;
  parameters: ToolParameter[];
  timeout_secs: number;
  is_global: boolean;
}

export interface ToolExecutionOutput {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  timed_out: boolean;
}

export type CircuitState =
  | { state: "closed" }
  | { state: "open"; failure_count: number; last_error: string; opened_at: string }
  | { state: "half_open" };

export type HookEvent =
  | "prompt_received"
  | "before_diff_apply"
  | "after_dag_execution"
  | "on_tool_failed";

// === Decentralized Addon Registry & Git Installer ===

export interface AddonManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  repository: string;
  entrypoint: string;
  required_slots: string[];
  permissions: string[];
}

export interface InstalledAddon {
  manifest: AddonManifest;
  install_path: string;
  enabled: boolean;
  installed_at: string;
  last_updated: string;
}

// === Air-Gapped Animated QR Sync Engine ===

export interface SyncPayload {
  version: string;
  created_at: string;
  checksum_sha256: string;
  config_json: string;
  slots_config?: string | null;
  active_addons: string[];
  custom_data?: string | null;
}

export interface AirGapIngestProgress {
  session_id: string;
  received_chunks: number;
  total_chunks: number;
  percent_complete: number;
  is_ready: boolean;
  error?: string | null;
}

// === Sovereign Ergonomics Suite ===

export interface DiagnosticLocation {
  file_path: string;
  line?: number | null;
  column?: number | null;
  error_type: string;
  message: string;
}

export interface TerminalFailureReport {
  command: string;
  exit_code: i32_or_number;
  clean_stderr_snippet: string;
  primary_error?: DiagnosticLocation | null;
  all_diagnostics: DiagnosticLocation[];
  stack_trace_lines: string[];
}

type i32_or_number = number;

export interface MentionCandidate {
  mention_type: "command" | "file" | "folder" | "symbol" | "context" | string;
  label: string;
  value: string;
  description: string;
  icon: string;
}

export interface PatchResult {
  patched_content: string;
  applied_hunks_count: number;
  total_hunks_count: number;
  syntax_warning?: string | null;
}

// === Fill-In-the-Middle (FIM) & Compiler Probe ===

export type FimTemplateFormat = "StarCoderDeepSeek" | "QwenCodeLlama" | "Llama3Generic";

export interface FimCompletionRequest {
  request_id: number;
  file_path: string;
  prefix: string;
  suffix: string;
  cursor_line: number;
  cursor_col: number;
  max_tokens: number;
  format?: FimTemplateFormat | null;
}

export interface FimCompletionResponse {
  request_id: number;
  suggested_text: string;
  latency_ms: number;
  model_used: string;
  stop_reason: string;
}

export type DiagnosticSeverity = "Error" | "Warning" | "Information";

export interface DiagnosticItem {
  file_path: string;
  line: number;
  col: number;
  severity: DiagnosticSeverity;
  message: string;
  source: string;
  code?: string | null;
}

// === Directive-Bound Bidirectional Formal Verifier ===

export type ConstraintExpression =
  | { type: "RangeBound"; data: { var: string; min?: number | null; max?: number | null } }
  | { type: "ArrayBound"; data: { array_var: string; index_var: string } }
  | { type: "NonZero"; data: { var: string } }
  | { type: "NonNull"; data: { var: string } }
  | { type: "CustomPredicate"; data: { expr: string } };

export interface CodeContract {
  requires: ConstraintExpression[];
  ensures: ConstraintExpression[];
  invariants: ConstraintExpression[];
  directive?: string | null;
}

export interface Counterexample {
  failing_var: string;
  failing_val: string;
  violation_expr: string;
  trace_summary: string;
}

export interface VerificationVerdict {
  forward_safety_proved: boolean;
  backward_intent_proved: boolean;
  is_bidirectionally_verified: boolean;
  confidence: number;
  proof_time_ms: number;
  steps_evaluated: number;
  counterexample?: Counterexample | null;
  violated_contract?: string | null;
}

export interface AmbientTelemetry {
  ram_usage_mb: number;
  latency_ms: number;
  tokens_saved_pct: number;
  estimated_cost_saved_usd: number;
}

export type OmniIntent =
  | { type: "LocalSearch"; data: { query: string } }
  | { type: "WebSearch"; data: { query: string } }
  | { type: "TerminalCommand"; data: { command: string } }
  | { type: "ChatMemory"; data: { description: string } }
  | { type: "FormalVerify"; data: { target: string } }
  | { type: "AgentAction"; data: { prompt: string; target_code?: string | null } };

export interface OmniSearchResult {
  title: string;
  subtitle: string;
  category: "File" | "Code" | "Terminal" | "Web" | "Action" | string;
  score: number;
}

export interface ChatMemoryEntry {
  id: string;
  session_id: string;
  role: string;
  content: string;
  timestamp: number;
  tags: string[];
}

export interface ChatMemoryMatch {
  entry: ChatMemoryEntry;
  snippet: string;
  score: number;
}

export interface InjectionReport {
  bytes_injected: number;
  elapsed_ms: number;
  clipboard_restored: boolean;
}

export interface AmbientActionResult {
  prompt: string;
  generated_patch?: string | null;
  explanation: string;
  verification_passed: boolean;
  latency_ms: number;
}

export interface QuickVerifyReport {
  target_function: string;
  is_safe: boolean;
  forward_safety_score: number;
  backward_intent_score: number;
  counterexample?: string | null;
  execution_time_ms: number;
}




















