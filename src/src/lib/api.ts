import { invoke } from "@tauri-apps/api/core";
import type {
  AgentHandle,
  AgentStats,
  DiffHunk,
  FileContent,
  LocalDevice,
  LocalDeviceSimple,
  LocalModel,
  ModelSelection,
  SearchResult,
  SemanticSearchResult,
  StagedFileChange,
  Template,
  WorkspaceIndex,
} from "../types";

// ---- File System ----

export const fs = {
  scan: (root?: string) =>
    invoke<{ index: WorkspaceIndex; duration_ms: number }>("fs_scan", { root }),
  readFile: (path: string) => invoke<FileContent>("fs_read_file", { path }),
  writeFile: (path: string, content: string) =>
    invoke<void>("fs_write_file", { path, content }),
  modifyFile: (path: string, ops: unknown[]) =>
    invoke<void>("fs_modify_file", { path, ops }),
  search: (query: string) => invoke<SearchResult[]>("fs_search", { query }),
  getIndex: () => invoke<WorkspaceIndex>("fs_get_index"),
  stageChange: (path: string, proposedContent: string) =>
    invoke<StagedFileChange>("fs_stage_change", { path, proposedContent }),
  acceptChange: (changeId: string) =>
    invoke<void>("fs_accept_change", { changeId }),
  rejectChange: (changeId: string) =>
    invoke<void>("fs_reject_change", { changeId }),
  listStagedChanges: () =>
    invoke<StagedFileChange[]>("fs_list_staged_changes"),
  computeHunks: (original: string, proposed: string) =>
    invoke<DiffHunk[]>("fs_compute_hunks", { original, proposed }),
  acceptHunk: (changeId: string, hunkId: string) =>
    invoke<StagedFileChange | null>("fs_accept_hunk", { changeId, hunkId }),
  rejectHunk: (changeId: string, hunkId: string) =>
    invoke<StagedFileChange | null>("fs_reject_hunk", { changeId, hunkId }),
  rollbackLast: () =>
    invoke<import("../types").RollbackResult>("fs_rollback_last"),
  listSnapshots: () =>
    invoke<import("../types").FileSnapshot[]>("fs_list_snapshots"),
  applySearchReplace: (path: string, content: string) =>
    invoke<import("../types").ApplySearchReplaceResult>("fs_apply_search_replace", {
      path,
      content,
    }),
};

// ---- Templates ----

export const templates = {
  list: (category?: string) =>
    invoke<Template[]>("templates_list", { category: category ?? null }),
  search: (query: string) => invoke<Template[]>("templates_search", { query }),
  get: (category: string, name: string) =>
    invoke<Template | null>("templates_get", { category, name }),
  categories: () => invoke<string[]>("templates_categories"),
};

// ---- Context ----

export const context = {
  assemble: (args: {
    user_request: string;
    templates: Template[];
    errors?: { agent_id: string; error_message: string; timestamp: string }[];
  }) =>
    invoke<{
      full_prompt: string;
      token_estimate: number;
      sections: {
        system_prompt: string;
        user_prompt: string;
        template_context: string;
        error_context: string;
      };
    }>("context_assemble", args),
  estimateTokens: (text: string) =>
    invoke<number>("context_estimate_tokens", { text }),
  semanticSearch: (query: string, top_k: number = 8) =>
    invoke<SemanticSearchResult[]>("context_semantic_search", { query, topK: top_k }),
  indexFile: (filePath: string, content: string) =>
    invoke<number>("context_index_file", { filePath, content }),
  extractSkeleton: (code: string, extension: string) =>
    invoke<import("../types").ExtractSkeletonResponse>("context_extract_skeleton", {
      code,
      extension,
    }),
  querySymbolGraph: (symbol: string, path?: string) =>
    invoke<import("../types").SymbolNode[]>("context_query_symbol_graph", {
      symbol,
      path: path ?? null,
    }),
  bm25Search: (query: string, limit?: number) =>
    invoke<import("../types").Bm25SearchResult[]>("context_bm25_search", {
      query,
      limit: limit ?? null,
    }),
  buildHybrid: (prompt: string, files?: string[], maxTokens?: number) =>
    invoke<import("../types").HybridContextPayload>("context_build_hybrid", {
      prompt,
      files: files ?? null,
      maxTokens: maxTokens ?? null,
    }),
};

// ---- Network ----

export const network = {
  start: () => invoke<void>("network_start"),
  stop: () => invoke<void>("network_stop"),
  discover: () => invoke<LocalDeviceSimple[]>("get_local_devices"),
  getLocalDevice: () => invoke<LocalDevice>("network_get_local_device"),
  assignTask: (request: { prompt: string; task_type?: string }) =>
    invoke<{ response: string; used_local: boolean; duration_ms: number }>(
      "network_assign_task",
      { request },
    ),
};

// ---- Agents ----

export const agents = {
  spawn: (request: {
    context: string;
    language: string;
    timeout_seconds?: number;
    max_memory_mb?: number;
    test_command?: string;
  }) => invoke<AgentHandle>("agent_spawn", { request }),
  kill: (agentId: string) => invoke<void>("agent_kill", { agent_id: agentId }),
  listActive: () => invoke<AgentHandle[]>("agent_list_active"),
  monitor: (agentId: string) => invoke<AgentStats>("agent_monitor", { agent_id: agentId }),
  executeTask: (request: {
    context: string;
    language: string;
    timeout_seconds?: number;
    max_memory_mb?: number;
    test_command?: string;
  }) =>
    invoke<{
      success: boolean;
      output: string;
      errors: string[];
      duration_ms: number;
      exit_code: number | null;
    }>("agent_execute_task", { request }),
};

// ---- LLM ----

export const llm = {
  detectModels: () => invoke<LocalModel[]>("llm_detect_models"),
  generate: (request: {
    prompt: string;
    model?: string;
    temperature?: number;
    max_tokens?: number;
  }) => invoke<{ response: string; model: string; backend: string }>("llm_generate", { request }),
  chat: (request: {
    messages: { role: string; content: string }[];
    model?: string;
  }) => invoke<{ response: string; model: string; backend: string }>("llm_chat", { request }),
  selectBestModel: (taskType?: string) =>
    invoke<ModelSelection>("llm_select_best_model", { task_type: taskType ?? null }),
  setDefaultModel: (model: string, backend: string) =>
    invoke<void>("llm_set_default_model", { model, backend }),
  saveApiKey: (provider: string, apiKey: string) =>
    invoke<void>("llm_save_api_key", { provider, apiKey }),
  getApiKeyStatus: () =>
    invoke<import("../types").ProviderStatus[]>("llm_get_api_key_status"),
  deleteApiKey: (provider: string) =>
    invoke<void>("llm_delete_api_key", { provider }),
  testApiKey: (provider: string, apiKey?: string, baseUrl?: string) =>
    invoke<import("../types").ProviderTestResult>("llm_test_api_key", {
      provider,
      apiKey: apiKey ?? null,
      baseUrl: baseUrl ?? null,
    }),
  getFallbackChain: () =>
    invoke<import("../types").FallbackChainConfig>("llm_get_fallback_chain"),
  setFallbackChain: (config: import("../types").FallbackChainConfig) =>
    invoke<void>("llm_set_fallback_chain", { config }),
  setFallbackStrategy: (strategy: import("../types").FallbackStrategy) =>
    invoke<void>("llm_set_fallback_strategy", { strategy }),
  autoDetectKeys: () =>
    invoke<import("../types").DetectedKeyReport[]>("llm_auto_detect_keys"),
  getKeyPool: (provider: string) =>
    invoke<import("../types").KeySlotStatus[]>("llm_get_key_pool", { provider }),
  saveKeyPool: (provider: string, keys: string) =>
    invoke<void>("llm_save_key_pool", { provider, keys }),
};

// ---- System & Diagnostics ----

export const system = {
  getDiagnostics: () =>
    invoke<import("../types").DiagnosticReport>("system_get_diagnostics"),
  exportDiagnostics: () =>
    invoke<import("../types").ExportDiagnosticResult>("system_export_diagnostics"),
};

// ---- Skills Engine ----

export const skills = {
  list: () => invoke<import("../types").SkillDto[]>("skills_list"),
  rescan: () => invoke<import("../types").SkillDto[]>("skills_rescan"),
  toggle: (skillId: string, enabled: boolean) =>
    invoke<boolean>("skills_toggle", { skillId, enabled }),
  execute: (skillId: string, args: Record<string, any>) =>
    invoke<import("../types").SkillExecutionResultDto>("skills_execute", {
      skillId,
      args,
    }),
  create: (request: import("../types").CreateSkillRequest) =>
    invoke<import("../types").SkillDto>("skills_create", { request }),
};

// ---- Mission Control & Task Graph DAG ----

export const taskGraph = {
  decompose: (goal: string, files?: string[]) =>
    invoke<import("../types").TaskGraph>("task_graph_decompose", {
      request: { goal, files: files ?? null },
    }),
  validate: (graph: import("../types").TaskGraph) =>
    invoke<string[]>("task_graph_validate", {
      request: { graph },
    }),
  updateNode: (
    mutGraph: import("../types").TaskGraph,
    nodeId: string,
    title?: string,
    description?: string,
    payload?: import("../types").TaskActionPayload,
    status?: import("../types").TaskNodeStatus
  ) =>
    invoke<import("../types").TaskGraph>("task_graph_update_node", {
      request: {
        mut_graph: mutGraph,
        node_id: nodeId,
        title: title ?? null,
        description: description ?? null,
        payload: payload ?? null,
        status: status ?? null,
      },
    }),
  executeNode: (mutGraph: import("../types").TaskGraph, nodeId: string) =>
    invoke<import("../types").TaskGraph>("task_graph_execute_node", {
      request: { mut_graph: mutGraph, node_id: nodeId },
    }),
};

export const spotlight = {
  toggle: () => invoke<boolean>("spotlight_toggle"),
  hide: () => invoke<void>("spotlight_hide"),
  show: () => invoke<void>("spotlight_show"),
  setPinned: (pinned: boolean) => invoke<boolean>("spotlight_set_pinned", { pinned }),
};

// ---- Specification Alignment & Tradeoff Gate ----

export const specAligner = {
  analyze: (goal: string, workspaceSummary?: string) =>
    invoke<import("../types").SpecAlignmentReport>("spec_aligner_analyze", {
      goal,
      workspaceSummary: workspaceSummary ?? null,
    }),
  applyTradeoffs: (
    report: import("../types").SpecAlignmentReport,
    selections: Record<string, string>
  ) =>
    invoke<import("../types").SpecAlignmentReport>("spec_aligner_apply_tradeoffs", {
      request: { report, selections },
    }),
};

// ---- Adversarial QA Agent ----

export const adversarialQa = {
  evaluate: (code: string, lang: string) =>
    invoke<import("../types").QaReport>("adversarial_qa_evaluate", {
      code,
      lang,
    }),
};

// ---- ADR & Negative Memory Ledger ----

export const adrLedger = {
  get: (workspaceRoot: string) =>
    invoke<import("../types").AdrLedger>("adr_ledger_get", {
      workspaceRoot,
    }),
  addNegative: (
    workspaceRoot: string,
    entry: import("../types").NegativeMemoryEntry
  ) =>
    invoke<import("../types").AdrLedger>("adr_ledger_add_negative", {
      request: { workspace_root: workspaceRoot, entry },
    }),
  addRecord: (workspaceRoot: string, record: import("../types").AdrRecord) =>
    invoke<import("../types").AdrLedger>("adr_ledger_add_record", {
      request: { workspace_root: workspaceRoot, record },
    }),
};

// ---- GitHub Device Flow & Git Sync ----

export const githubAuth = {
  requestDeviceCode: (scope?: string) =>
    invoke<import("../types").DeviceCodeResponse>("github_request_device_code", {
      scope,
    }),
  pollToken: (deviceCode: string) =>
    invoke<import("../types").DeviceFlowPollStatus>("github_poll_token", {
      deviceCode,
    }),
  getStatus: () =>
    invoke<import("../types").GitHubAuthStatus>("github_get_status"),
  logout: () => invoke<void>("github_logout"),
  listRepos: (page?: number, perPage?: number) =>
    invoke<import("../types").GitHubRepo[]>("github_list_repos", {
      page,
      perPage,
    }),
};

export const gitSync = {
  getStatus: (workspacePath: string) =>
    invoke<import("../types").GitStatusReport>("git_get_status", {
      workspacePath,
    }),
  cloneRepo: (options: import("../types").GitCloneOptions) =>
    invoke<string>("git_clone_repo", {
      options,
    }),
  smartCommit: (workspacePath: string, intent?: string, autoPush = false) =>
    invoke<import("../types").SmartCommitResult>("git_smart_commit", {
      request: {
        workspace_path: workspacePath,
        intent,
        auto_push: autoPush,
      },
    }),
  createPullRequest: (request: import("../types").CreatePrRequest) =>
    invoke<import("../types").PullRequestResult>("git_create_pull_request", {
      request,
    }),
};

export const cognitiveRouter = {
  route: (request: import("../types").RouteTaskRequest) =>
    invoke<import("../types").RoutingDecision>("cognitive_router_route", {
      request,
    }),
  classify: (prompt: string, fileCount = 1, contextTokens = 500) =>
    invoke<import("../types").CognitiveTaskComplexity>("cognitive_router_classify", {
      prompt,
      fileCount,
      contextTokens,
    }),
  getStrategy: () =>
    invoke<import("../types").BudgetStrategy>("cognitive_router_get_strategy"),
  setStrategy: (strategy: import("../types").BudgetStrategy) =>
    invoke<void>("cognitive_router_set_strategy", {
      strategy,
    }),
};

export const localDiscovery = {
  probeHardware: () =>
    invoke<import("../types").HardwareProfile>("local_discovery_probe_hardware"),
  scanEndpoints: () =>
    invoke<import("../types").LocalInferenceEndpoint[]>("local_discovery_scan_endpoints"),
  getReport: () =>
    invoke<import("../types").LocalDiscoveryReport>("local_discovery_get_report"),
};

export const modelPuller = {
  startPull: (modelName: string, endpointUrl?: string) =>
    invoke<string>("model_puller_start_pull", {
      modelName,
      endpointUrl,
    }),
  getProgress: (jobId: string) =>
    invoke<import("../types").ModelPullProgress | null>("model_puller_get_progress", {
      jobId,
    }),
  cancelPull: (jobId: string) =>
    invoke<void>("model_puller_cancel_pull", {
      jobId,
    }),
};

export const freeProviderRadar = {
  getSuggestions: () =>
    invoke<import("../types").FreeProviderSuggestion[]>("free_provider_radar_get_suggestions"),
  dismiss: (providerId: string) =>
    invoke<void>("free_provider_radar_dismiss", {
      providerId,
    }),
  saveAndActivate: (providerId: string, apiKey: string) =>
    invoke<void>("free_provider_radar_save_and_activate", {
      providerId,
      apiKey,
    }),
};

export const research = {
  fetchDocs: (query: string, ecosystem = "general", version?: string) =>
    invoke<import("../types").DocSearchResult>("research_fetch_docs", {
      query,
      ecosystem,
      version,
    }),
  resolveError: (errorSnippet: string) =>
    invoke<import("../types").ResolvedErrorSolution>("research_resolve_error", {
      errorSnippet,
    }),
  clearDocsCache: () =>
    invoke<number>("research_clear_docs_cache"),
};

export const editorBridge = {
  getStatus: () =>
    invoke<import("../types").EditorBridgeStatus>("editor_bridge_status"),
  syncFile: (path: string, content: string) =>
    invoke<import("../types").EditorSyncReport>("editor_bridge_sync_file", {
      path,
      content,
    }),
  openInEditor: (
    path: string,
    line?: number,
    column?: number,
    preferredEditor?: string
  ) =>
    invoke<boolean>("editor_bridge_open_in_editor", {
      path,
      line: line ?? null,
      column: column ?? null,
      preferredEditor: preferredEditor ?? null,
    }),
  detectEditors: () =>
    invoke<import("../types").DetectedEditor[]>("editor_bridge_detect_editors"),
};

export const security = {
  scanSnippet: (codeSnippet: string, language?: string) =>
    invoke<import("../types").SecurityScanResult>("security_scan_snippet", {
      codeSnippet,
      language: language ?? null,
    }),
  scanDiff: (diff: string) =>
    invoke<import("../types").SecurityScanResult>("security_scan_diff", {
      diff,
    }),
};

export const ambient = {
  getSnapshot: (selectedText?: string) =>
    invoke<import("../types").AmbientSnapshot>("ambient_get_snapshot", {
      selectedText: selectedText ?? null,
    }),
  pasteToActive: (text: string) =>
    invoke<boolean>("ambient_paste_to_active", { text }),
};

export const slots = {
  getConfig: () =>
    invoke<import("../types").SlotsConfig>("slots_get_config"),
  setDriver: (slotType: import("../types").SlotType, driverId: string) =>
    invoke<import("../types").SlotsConfig>("slots_set_driver", {
      slotType,
      driverId,
    }),
  listAvailable: () =>
    invoke<import("../types").SlotDescriptor[]>("slots_list_available"),
};

export const pluginsTools = {
  listLocalTools: (workspacePath?: string) =>
    invoke<import("../types").LocalToolManifest[]>("plugins_list_local_tools", {
      workspacePath: workspacePath ?? null,
    }),
  runLocalTool: (toolId: string, args: string[], workspacePath?: string) =>
    invoke<import("../types").ToolExecutionOutput>("plugins_run_local_tool", {
      toolId,
      args,
      workspacePath: workspacePath ?? null,
    }),
  getCircuitStatus: () =>
    invoke<Record<string, import("../types").CircuitState>>("plugins_get_circuit_status"),
  resetCircuit: (toolId: string) =>
    invoke<boolean>("plugins_reset_circuit", { toolId }),
};

export const pluginsRegistry = {
  list: () =>
    invoke<import("../types").InstalledAddon[]>("plugins_registry_list"),
  installGit: (repoUrl: string) =>
    invoke<import("../types").InstalledAddon>("plugins_registry_install_git", { repoUrl }),
  toggle: (addonId: string, enabled: boolean) =>
    invoke<boolean>("plugins_registry_toggle", { addonId, enabled }),
  uninstall: (addonId: string) =>
    invoke<boolean>("plugins_registry_uninstall", { addonId }),
};

export const airgap = {
  generateSyncFrames: () =>
    invoke<string[]>("airgap_generate_sync_frames"),
  ingestFrame: (frameData: string) =>
    invoke<import("../types").AirGapIngestProgress>("airgap_ingest_frame", { frameData }),
  applySyncedPayload: (sessionId?: string) =>
    invoke<boolean>("airgap_apply_synced_payload", { sessionId: sessionId ?? null }),
  resetReceiver: (sessionId?: string) =>
    invoke<boolean>("airgap_reset_receiver", { sessionId: sessionId ?? null }),
};

export const ergonomics = {
  parseDiffHunks: (original: string, modified: string) =>
    invoke<import("../types").DiffHunk[]>("fs_parse_diff_hunks", { original, modified }),
  applySelectedHunks: (
    filePath: string,
    originalContent: string,
    hunks: import("../types").DiffHunk[],
    selectedIds: string[]
  ) =>
    invoke<import("../types").PatchResult>("fs_apply_selected_hunks", {
      filePath,
      originalContent,
      hunks,
      selectedIds,
    }),
  processTerminalFailure: (command: string, exitCode: number, stderr: string) =>
    invoke<import("../types").TerminalFailureReport>("terminal_process_failure", {
      command,
      exitCode,
      stderr,
    }),
  queryMentions: (query: string, workspaceRoot?: string, filterType?: string) =>
    invoke<import("../types").MentionCandidate[]>("context_query_mentions", {
      query,
      workspaceRoot: workspaceRoot ?? null,
      filterType: filterType ?? null,
    }),
};

export const fim = {
  requestInlineCompletion: (req: import("../types").FimCompletionRequest) =>
    invoke<import("../types").FimCompletionResponse>("fim_request_inline_completion", { req }),
};

export const compilerDiagnostics = {
  runProbe: (workspaceRoot: string) =>
    invoke<import("../types").DiagnosticItem[]>("diagnostics_run_probe", { workspaceRoot }),
  getActiveFeed: () =>
    invoke<import("../types").DiagnosticItem[]>("diagnostics_get_active_feed"),
};

export const i18nApi = {
  getLocale: () =>
    invoke<string>("i18n_get_locale"),
  setLocale: (locale: string) =>
    invoke<boolean>("i18n_set_locale", { locale }),
};

export const verifier = {
  proveContract: (
    filePath: string,
    patchContent: string,
    contract?: import("../types").CodeContract
  ) =>
    invoke<import("../types").VerificationVerdict>("verifier_prove_contract", {
      filePath,
      patchContent,
      contract: contract ?? null,
    }),
  getActiveInvariants: (workspaceRoot: string) =>
    invoke<string[]>("verifier_get_active_invariants", { workspaceRoot }),
};

export const overlayApi = {
  toggleSpotlight: () =>
    invoke<boolean>("toggle_spotlight"),
  getAmbientTelemetry: () =>
    invoke<import("../types").AmbientTelemetry>("get_ambient_telemetry"),
  dismiss: () =>
    invoke<boolean>("ambient_controller_dismiss"),
  parseOmnibarInput: (input: string, clipboard?: string | null) =>
    invoke<import("../types").OmniIntent>("parse_omnibar_input", {
      input,
      clipboard: clipboard ?? null,
    }),
  queryOmniSearch: (query: string, rootPath?: string | null) =>
    invoke<import("../types").OmniSearchResult[]>("query_omni_search", {
      query,
      rootPath: rootPath ?? null,
    }),
  searchChatMemory: (query: string, limit?: number | null) =>
    invoke<import("../types").ChatMemoryMatch[]>("search_chat_memory", {
      query,
      limit: limit ?? null,
    }),
  injectTextToActive: (text: string, restoreClipboard?: boolean | null) =>
    invoke<import("../types").InjectionReport>("inject_text_to_active", {
      text,
      restoreClipboard: restoreClipboard ?? null,
    }),
  executeAmbientAgent: (prompt: string, targetCode?: string | null) =>
    invoke<import("../types").AmbientActionResult>("execute_ambient_agent", {
      prompt,
      targetCode: targetCode ?? null,
    }),
  runQuickFormalVerify: (target: string, codeContext?: string | null) =>
    invoke<import("../types").QuickVerifyReport>("run_quick_formal_verify", {
      target,
      codeContext: codeContext ?? null,
    }),
};