import { Page, expect } from "@playwright/test";

export interface MockStateOptions {
  workspaceRoot?: string;
  filesCount?: number;
  modelsCount?: number;
  devicesCount?: number;
  initialKeys?: Record<string, string>;
  simulateChatError?: boolean;
  simulateTestFail?: boolean;
}

export function createInitialMockState(options: MockStateOptions = {}) {
  return {
    workspaceRoot: options.workspaceRoot ?? "D:/LOCUS/projects/demo-project",
    totalFiles: options.filesCount ?? 28,
    totalSize: 4194304,
    models: [
      { name: "qwen2.5-coder:7b", size: "4.7 GB", digest: "sha256:abc12345" },
      { name: "deepseek-coder-v2:16b", size: "8.9 GB", digest: "sha256:def67890" },
      { name: "llama3.2:3b", size: "2.0 GB", digest: "sha256:11223344" },
    ],
    selectedModel: "qwen2.5-coder:7b",
    devices: [
      {
        id: "dev-local-worker-1",
        name: "MacBook-M3-Pro",
        hostname: "macbook.local",
        ip_address: "192.168.1.105",
        port: 8080,
        status: "Online",
        device_type: "Worker",
        models: ["deepseek-coder-v2:16b"],
        vram_gb: 18,
        specializations: ["CodeGeneration", "Refactoring"],
        performance_score: 94,
      },
      {
        id: "dev-local-worker-2",
        name: "RTX-4090-Rig",
        hostname: "desktop-gpu.local",
        ip_address: "192.168.1.110",
        port: 8080,
        status: "Online",
        device_type: "Worker",
        models: ["qwen2.5-coder:32b"],
        vram_gb: 24,
        specializations: ["SemanticSearch", "CodeGeneration"],
        performance_score: 99,
      },
    ],
    keys: {
      gemini: options.initialKeys?.gemini ?? "AIzaSyMockKeyForGeminiTesting12345",
      groq: options.initialKeys?.groq ?? "",
      openrouter: options.initialKeys?.openrouter ?? "",
      deepseek: options.initialKeys?.deepseek ?? "",
      openai: options.initialKeys?.openai ?? "",
      anthropic: options.initialKeys?.anthropic ?? "",
    },
    stagedChanges: [
      {
        change_id: "diff-001",
        file_path: "src/lib/router.rs",
        original_content: `pub fn route_request(strategy: &str) -> &'static str {\n    "local"\n}\n`,
        proposed_content: `pub fn route_request(strategy: &str) -> &'static str {\n    match strategy {\n        "CloudFirst" => "cloud",\n        "SpeedFirst" => "fastest",\n        _ => "local",\n    }\n}\n`,
        created_at: new Date().toISOString(),
      },
      {
        change_id: "diff-002",
        file_path: "src/components/Chat.tsx",
        original_content: `export function Chat() {\n  return <div>Chat</div>;\n}`,
        proposed_content: `export function Chat() {\n  return <div className="chat-container">LOCUS Neural Chat</div>;\n}`,
        created_at: new Date().toISOString(),
      },
    ],
    fallbackChain: {
      enabled: true,
      strategy: "LocalFirst",
      targets: [
        {
          id: "target-gemini",
          label: "Google Gemini",
          is_local: false,
          enabled: true,
          preferred_model: "gemini-2.5-flash",
        },
        {
          id: "target-groq",
          label: "Groq LPU",
          is_local: false,
          enabled: true,
          preferred_model: "llama-3.3-70b-versatile",
        },
        {
          id: "target-openrouter",
          label: "OpenRouter",
          is_local: false,
          enabled: true,
          preferred_model: "deepseek/deepseek-r1",
        },
      ],
      max_retries_per_target: 2,
      timeout_seconds: 15,
    },
    p2pRunning: false,
    activeAgents: [
      {
        id: "agent-syntax-auditor",
        task: "Semantic AST Verification",
        status: "Running",
        started_at: new Date().toISOString(),
      },
    ],
    simulateChatError: options.simulateChatError ?? false,
    simulateTestFail: options.simulateTestFail ?? false,
  };
}

/**
 * Attaches the strict console error monitor and registers complete Tauri IPC mock handlers.
 */
export async function setupTauriMocks(page: Page, options: MockStateOptions = {}) {
  // 1. Strict Console Error Enforcement
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      const text = msg.text();
      if (
        !text.includes("favicon.ico") &&
        !text.includes("The play() request was interrupted") &&
        !text.includes("AudioContext was not allowed to start") &&
        !text.includes("AudioContext")
      ) {
        consoleErrors.push(text);
      }
    }
  });

  page.on("pageerror", (err) => {
    consoleErrors.push(err.message);
  });

  (page as any).__consoleErrors = consoleErrors;

  // 2. Inject Mock IPC State Machine
  const initialState = createInitialMockState(options);

  await page.addInitScript((state) => {
    const mockState = { ...state };

    const invokeHandler = async (cmd: string, args: any = {}) => {
      switch (cmd) {
        // --- Filesystem & Workspace ---
        case "fs_scan":
        case "fs_get_index":
          return {
            index: {
              root_path: mockState.workspaceRoot,
              total_files: mockState.totalFiles,
              total_size: mockState.totalSize,
              last_indexed: new Date().toISOString(),
              files: {
                "src/lib/router.rs": {
                  path: "src/lib/router.rs",
                  size: 1024,
                  modified: new Date().toISOString(),
                  language: "Rust",
                },
                "src/components/Chat.tsx": {
                  path: "src/components/Chat.tsx",
                  size: 2048,
                  modified: new Date().toISOString(),
                  language: "TypeScript",
                },
                "crates/locus-core/src/diagnostics.rs": {
                  path: "crates/locus-core/src/diagnostics.rs",
                  size: 4096,
                  modified: new Date().toISOString(),
                  language: "Rust",
                },
              },
            },
            duration_ms: 12,
          };

        case "fs_list_staged_changes":
          return mockState.stagedChanges;

        case "fs_stage_change": {
          const filePath = args.path || args.file_path || "src/lib/optimized.rs";
          const proposed = args.proposedContent || args.proposed_content || "// ⚡ Verified & Optimized by LOCUS Engine\n";
          const newDiff = {
            change_id: `diff-${Date.now()}`,
            file_path: filePath,
            original_content: "// Original file content\n",
            proposed_content: proposed,
            created_at: new Date().toISOString(),
          };
          mockState.stagedChanges.push(newDiff);
          return newDiff;
        }

        case "fs_accept_change": {
          const cid = args.changeId || args.change_id;
          mockState.stagedChanges = mockState.stagedChanges.filter(
            (c: any) => c.change_id !== cid
          );
          return;
        }

        case "fs_reject_change": {
          const cid = args.changeId || args.change_id;
          mockState.stagedChanges = mockState.stagedChanges.filter(
            (c: any) => c.change_id !== cid
          );
          return;
        }

        case "fs_compute_hunks":
          return [
            {
              hunk_id: "hunk-1",
              old_start: 1,
              old_lines: 5,
              new_start: 1,
              new_lines: 7,
              header: "@@ -1,5 +1,7 @@",
              lines: [
                { line_type: "Context", content: "pub fn hello_locus() {", old_line_no: 1, new_line_no: 1 },
                { line_type: "Addition", content: '    println!("Hunk #1 Accepted");', old_line_no: null, new_line_no: 2 },
              ],
            },
          ];

        case "fs_accept_hunk": {
          const cid = args.changeId || args.change_id;
          // In mock, accept hunk either clears or updates
          mockState.stagedChanges = mockState.stagedChanges.filter(
            (c: any) => c.change_id !== cid
          );
          return null;
        }

        case "fs_reject_hunk": {
          const cid = args.changeId || args.change_id;
          mockState.stagedChanges = mockState.stagedChanges.filter(
            (c: any) => c.change_id !== cid
          );
          return null;
        }

        case "fs_rollback_last":
          return {
            success: true,
            snapshot_id: "snap-mock-01",
            file_path: "src/lib/router.rs",
            restored_bytes: 512,
            message: "Successfully rolled back router.rs to previous snapshot",
          };

        case "fs_list_snapshots":
          return [
            {
              snapshot_id: "snap-mock-01",
              created_at: new Date().toISOString(),
              file_path: "src/lib/router.rs",
              previous_content: "// Previous router content",
              description: "Before applying diff hunk #1",
            },
          ];

        case "fs_read_file":
          return {
            file_path: args.path || args.file_path,
            content: `// LOCUS File Content for ${args.path || args.file_path}\npub fn hello_locus() {\n    println!("Local-first IDE ready");\n}\n`,
          };

        case "fs_search":
          return [
            {
              file_path: "src/lib/router.rs",
              line_number: 10,
              line_content: "pub fn route_request(strategy: &str) -> &'static str {",
              match_start: 7,
              match_end: 20,
            },
          ];

        // --- Context & Templates ---
        case "context_assemble": {
          const userReq = args.user_request || args.request?.user_request || "Hello LOCUS";
          return {
            full_prompt: userReq,
            token_estimate: 120,
            sections: {
              system_prompt: "You are LOCUS AI assistant.",
              user_prompt: userReq,
              template_context: "",
              error_context: "",
            },
          };
        }

        case "context_estimate_tokens":
          return Math.max(1, Math.round((args.text?.length || 0) / 4));

        case "context_semantic_search":
        case "semantic_search":
          return [
            {
              document_id: "doc-1",
              file_path: "crates/locus-core/src/diagnostics.rs",
              symbol_name: "export_system_diagnostics",
              symbol_kind: "Function",
              snippet: "pub fn export_system_diagnostics() -> DiagnosticReport { ... }",
              line_start: 45,
              line_end: 65,
              similarity: 0.94,
              language: "rust",
              tags: ["diagnostics", "sanitization"],
            },
            {
              document_id: "doc-2",
              file_path: "crates/locus-llm/src/keyring.rs",
              symbol_name: "KeyringStore",
              symbol_kind: "Struct",
              snippet: "pub struct KeyringStore { ... }",
              line_start: 12,
              line_end: 35,
              similarity: 0.88,
              language: "rust",
              tags: ["security", "keyring"],
            },
          ];

        case "context_index_file":
          return 42;

        case "templates_list":
        case "templates_search":
          return [];

        case "templates_get":
          return null;

        case "templates_categories":
          return ["Rust", "TypeScript", "Python"];

        // --- LLM & AI Engine ---
        case "llm_detect_models":
          return mockState.models;

        case "llm_select_best_model":
          return {
            model_name: mockState.selectedModel,
            backend: "ollama",
            estimated_vram_mb: 4800,
            estimated_ram_mb: 6144,
            fits_in_vram: true,
            quantization: "Q4_K_M",
            reasoning: "Selected based on available GPU VRAM and lowest token latency.",
          };

        case "llm_set_default_model":
          mockState.selectedModel = args.model || args.model_name;
          return;

        case "llm_generate":
        case "llm_chat":
          if (mockState.simulateChatError) {
            throw new Error("Simulated LLM Provider Timeout: 504 Gateway Timeout");
          }
          return {
            response: `### LOCUS AI Response\n\nHere is the high-performance implementation:\n\n\`\`\`rust\npub fn fibonacci_fast(n: u64) -> u64 {\n    let mut a = 0;\n    let mut b = 1;\n    for _ in 0..n {\n        let tmp = a + b;\n        a = b;\n        b = tmp;\n    }\n    a\n}\n\`\`\`\n\n* Execution latency: 42ms\n* Security: 100% Zero-Leak Guaranteed`,
            model: mockState.selectedModel || "qwen2.5-coder:7b",
            backend: "ollama",
            provider_stamp: "🔒 Local (qwen2.5-coder:7b)",
            latency_ms: 42,
            token_count: 148,
            duration_ms: 185,
            was_fallback: false,
            fallback_reason: undefined,
          };

        case "llm_get_api_key_status":
          return [
            {
              provider_id: "gemini",
              name: "Google Gemini",
              is_configured: !!mockState.keys.gemini,
              key_preview: mockState.keys.gemini ? "AIza••••••••1234" : undefined,
              default_model: "gemini-2.5-flash",
              supports_custom_url: false,
            },
            {
              provider_id: "groq",
              name: "Groq LPU",
              is_configured: !!mockState.keys.groq,
              key_preview: mockState.keys.groq ? "gsk_••••••••5678" : undefined,
              default_model: "llama-3.3-70b-versatile",
              supports_custom_url: false,
            },
            {
              provider_id: "openrouter",
              name: "OpenRouter",
              is_configured: !!mockState.keys.openrouter,
              key_preview: mockState.keys.openrouter ? "sk-or-••••••••9012" : undefined,
              default_model: "deepseek/deepseek-r1",
              supports_custom_url: false,
            },
            {
              provider_id: "deepseek",
              name: "DeepSeek Direct",
              is_configured: !!mockState.keys.deepseek,
              key_preview: mockState.keys.deepseek ? "sk-••••••••3456" : undefined,
              default_model: "deepseek-chat",
              supports_custom_url: false,
            },
            {
              provider_id: "openai",
              name: "OpenAI Compatible",
              is_configured: !!mockState.keys.openai,
              key_preview: mockState.keys.openai ? "sk-proj-••••••••7890" : undefined,
              default_model: "gpt-4o",
              supports_custom_url: true,
            },
            {
              provider_id: "anthropic",
              name: "Anthropic Claude",
              is_configured: !!mockState.keys.anthropic,
              key_preview: mockState.keys.anthropic ? "sk-ant-••••••••1122" : undefined,
              default_model: "claude-3-5-sonnet-20241022",
              supports_custom_url: false,
            },
          ];

        case "llm_save_api_key":
          mockState.keys[args.provider || args.provider_id] = args.apiKey || args.api_key || args.key;
          return;

        case "llm_delete_api_key":
          mockState.keys[args.provider || args.provider_id] = "";
          return;

        case "llm_test_api_key":
          if (mockState.simulateTestFail) {
            return {
              success: false,
              provider_id: args.provider || args.provider_id || "gemini",
              latency_ms: 1250,
              available_models: [],
              message: "Invalid API key: 401 Unauthorized",
            };
          }
          return {
            success: true,
            provider_id: args.provider || args.provider_id || "gemini",
            latency_ms: 95,
            available_models: ["gemini-2.5-flash", "gemini-2.5-pro"],
            message: "Connection verified with hardware-accelerated latency (95ms).",
          };

        case "llm_auto_detect_keys":
        case "auto_detect_api_keys":
          mockState.keys.gemini = "AIzaSyMockKeyForGeminiTesting12345";
          mockState.keys.groq = "gsk_MockKeyForGroqFastTesting67890";
          return [
            {
              provider_id: "gemini",
              provider_name: "Google Gemini",
              key_masked: "AIza••••••••2345",
              source: "System Environment (GEMINI_API_KEY)",
              imported: true,
            },
            {
              provider_id: "groq",
              provider_name: "Groq LPU",
              key_masked: "gsk_••••••••7890",
              source: "Local .env (GROQ_API_KEY)",
              imported: true,
            },
          ];

        case "llm_get_fallback_chain":
          return mockState.fallbackChain;

        case "llm_set_fallback_chain":
          mockState.fallbackChain = args.config || args.chain;
          return;

        case "llm_set_fallback_strategy":
          mockState.fallbackChain.strategy = args.strategy;
          return;

        // --- Network & P2P Mesh ---
        case "get_local_devices":
        case "network_discover":
          return mockState.devices;

        case "network_get_local_device":
          return {
            id: "dev-main-host",
            name: "LOCUS-Master-Host",
            hostname: "locus-host.local",
            ip_address: "192.168.1.50",
            port: 8080,
            status: "Online",
            device_type: "Main",
            last_seen: new Date().toISOString(),
            capabilities: {
              models: mockState.models,
              max_context_tokens: 32768,
              vram_gb: 16,
              quantization: ["Q4_K_M", "Q8_0"],
              cpu_cores: 16,
              memory_gb: 32,
              supports_gpu: true,
              specializations: ["Orchestrator", "CodeGeneration"],
              performance_score: 98,
            },
          };

        case "network_start":
          mockState.p2pRunning = true;
          return;

        case "network_stop":
          mockState.p2pRunning = false;
          return;

        case "network_assign_task":
          return {
            response: "P2P Task executed on RTX-4090-Rig peer node in 112ms",
            used_local: false,
            duration_ms: 112,
          };

        // --- Agents ---
        case "agent_list_active":
          return mockState.activeAgents;

        case "agent_execute_task":
          return {
            success: true,
            output: "SYNTAX OK: Sandbox verification succeeded.",
            errors: [],
            duration_ms: 24,
            exit_code: 0,
          };

        case "agent_spawn": {
          const handle = {
            id: `agent-${Date.now()}`,
            task: args.request?.context || "General Agent Task",
            status: "Running",
            started_at: new Date().toISOString(),
          };
          mockState.activeAgents.push(handle);
          return handle;
        }

        case "agent_kill":
          mockState.activeAgents = mockState.activeAgents.filter(
            (a: any) => a.id !== (args.agent_id || args.agentId)
          );
          return;

        case "agent_monitor":
          return {
            agent_id: args.agent_id || args.agentId,
            memory_bytes: 42000000,
            cpu_percent: 4.2,
            active_threads: 2,
            status: "Healthy",
          };

        // --- System Diagnostics ---
        case "system_get_diagnostics":
          return {
            system_environment: {
              os: "windows",
              arch: "x86_64",
              total_physical_ram_gb: 32.0,
              logical_cpu_cores: 16,
              locus_version: "0.1.0-alpha",
              timestamp: new Date().toISOString(),
            },
            ai_engine_status: {
              fallback_strategy: mockState.fallbackChain.strategy,
              local_models_count: mockState.models.length,
              configured_cloud_providers: Object.keys(mockState.keys).filter((k) => !!mockState.keys[k]),
              active_routing_target: "Gemini 2.5 Flash",
            },
            workspace_status: {
              is_loaded: true,
              total_indexed_files: mockState.totalFiles,
              total_size_bytes: mockState.totalSize,
            },
            p2p_mesh_status: {
              is_running: mockState.p2pRunning,
              discovered_peer_count: mockState.devices.length,
            },
            sanitized_diagnostic_logs: [
              {
                timestamp: new Date().toISOString(),
                level: "INFO",
                subsystem: "locus-core",
                message: "LOCUS engine initialized with hardware credential storage.",
              },
              {
                timestamp: new Date().toISOString(),
                level: "INFO",
                subsystem: "locus-llm",
                message: "Auto-detected fallback chain: Gemini -> Groq -> OpenRouter.",
              },
              {
                timestamp: new Date().toISOString(),
                level: "INFO",
                subsystem: "locus-network",
                message: "P2P UDP discovery listening on port 8080.",
              },
            ],
          };

        case "system_export_diagnostics":
          return {
            file_name: `locus-diagnostic-report-${Date.now()}.json`,
            json_payload: JSON.stringify(
              {
                locus_version: "0.1.0-alpha",
                sanitized_at: new Date().toISOString(),
                os: "windows-x86_64",
                privacy_guarantee: "100% zero PII redacted",
              },
              null,
              2
            ),
            summary: "12 log events exported. Zero API keys, passwords, or personal user paths included.",
          };

        // --- Tauri Plugins ---
        case "plugin:opener|open_url":
          return;

        case "plugin:dialog|open":
          return "D:/LOCUS/projects/demo-workspace";

        default:
          console.warn("[UNHANDLED TAURI MOCK COMMAND]", cmd, args);
          return null;
      }
    };

    // Attach to window
    (window as any).__TAURI_INTERNALS__ = {
      invoke: invokeHandler,
      convertFileSrc: (path: string) => path,
    };
    (window as any).__TAURI__ = {
      core: {
        invoke: invokeHandler,
      },
    };
    (window as any).isTauri = true;
  }, initialState);
}

/**
 * Asserts that no console errors or unhandled exceptions occurred during test execution.
 */
export function assertNoConsoleErrors(page: Page) {
  const errors = (page as any).__consoleErrors || [];
  expect(errors, `Expected 0 console errors, but received: \n${errors.join("\n")}`).toEqual([]);
}
