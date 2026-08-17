import { useEffect, useState } from "react";
import type {
  AppState,
  DetectedKeyReport,
  ExportDiagnosticResult,
  FallbackChainConfig,
  FallbackStrategy,
  PrivacyMode,
  ProviderStatus,
  ProviderTestResult,
} from "../types";
import { llm, network, system } from "../lib/api";
import { sounds } from "../lib/sound";
import { checkForAppUpdates, installAppUpdate } from "../lib/updater";
import {
  AVAILABLE_FONTS,
  getTypographySettings,
  saveTypographySettings,
  type TypographySettings,
} from "../lib/theme";
import SkillsManager from "./SkillsManager";
import LocalModelDiscoveryBanner from "./LocalModelDiscoveryBanner";
import FreeTierRadarBanner from "./FreeTierRadarBanner";

const PROVIDER_KEY_URLS: Record<string, { label: string; url: string }> = {
  gemini: { label: "Google AI Studio", url: "https://aistudio.google.com/app/apikey" },
  groq: { label: "Groq Console", url: "https://console.groq.com/keys" },
  openrouter: { label: "OpenRouter Keys", url: "https://openrouter.ai/keys" },
  deepseek: { label: "DeepSeek Platform", url: "https://platform.deepseek.com/api_keys" },
  openai: { label: "OpenAI API Keys", url: "https://platform.openai.com/api-keys" },
  anthropic: { label: "Anthropic Console", url: "https://console.anthropic.com/settings/keys" },
  mistral: { label: "Mistral Console", url: "https://console.mistral.ai/api-keys/" },
};

interface SettingsProps {
  state: AppState;
  setState: React.Dispatch<React.SetStateAction<AppState>>;
  setPrivacyMode: (mode: PrivacyMode) => void;
  onOpenOnboarding?: () => void;
}

function Settings({ state, setState, setPrivacyMode, onOpenOnboarding }: SettingsProps) {
  const [workspacePath, setWorkspacePath] = useState(state.workspaceRoot ?? "");
  const [scanning, setScanning] = useState(false);
  const [meshBusy, setMeshBusy] = useState(false);
  const [typography, setTypography] = useState<TypographySettings>(getTypographySettings());
  const [autoSelectBusy, setAutoSelectBusy] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(sounds.isEnabled());
  const [modelTestStatus, setModelTestStatus] = useState<string | null>(null);
  const [testingModel, setTestingModel] = useState(false);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [hasUpdate, setHasUpdate] = useState(false);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);

  const [showNewton, setShowNewton] = useState(() => {
    try {
      return localStorage.getItem("locus_show_newton_companion") !== "false";
    } catch {
      return true;
    }
  });

  // Diagnostic Logs Export state
  const [exportingDiagnostics, setExportingDiagnostics] = useState(false);
  const [diagnosticResult, setDiagnosticResult] = useState<ExportDiagnosticResult | null>(null);
  const [diagnosticCopied, setDiagnosticCopied] = useState(false);

  // Cloud API Keyring state
  const [providers, setProviders] = useState<ProviderStatus[]>([]);
  const [inputKeys, setInputKeys] = useState<Record<string, string>>({});
  const [customUrls, setCustomUrls] = useState<Record<string, string>>({});
  const [testResults, setTestResults] = useState<Record<string, ProviderTestResult>>({});
  const [testingProvider, setTestingProvider] = useState<string | null>(null);
  const [savingProvider, setSavingProvider] = useState<string | null>(null);
  const [autoDetecting, setAutoDetecting] = useState(false);
  const [detectedReports, setDetectedReports] = useState<DetectedKeyReport[] | null>(null);

  // Auto-Fallback Chain state
  const [fallbackChain, setFallbackChain] = useState<FallbackChainConfig | null>(null);

  const loadProviders = async () => {
    try {
      const list = await llm.getApiKeyStatus();
      setProviders(list);
    } catch (e) {
      console.error("Failed to load cloud providers status", e);
    }
  };

  const loadFallbackConfig = async () => {
    try {
      const cfg = await llm.getFallbackChain();
      setFallbackChain(cfg);
    } catch (e) {
      console.error("Failed to load fallback chain config", e);
    }
  };

  const openExternalLink = async (url: string) => {
    sounds.playClick();
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (e) {
      window.open(url, "_blank");
    }
  };

  const handlePasteKey = async (providerId: string) => {
    try {
      const clip = await navigator.clipboard.readText();
      if (clip && clip.trim()) {
        sounds.playClick();
        setInputKeys((prev) => ({
          ...prev,
          [providerId]: clip.trim(),
        }));
      }
    } catch (err) {
      console.warn("Failed to read clipboard:", err);
    }
  };

  const handleAutoDetectKeys = async () => {
    sounds.playClick();
    setAutoDetecting(true);
    setDetectedReports(null);
    try {
      const reports = await llm.autoDetectKeys();
      setDetectedReports(reports);
      if (reports.some((r) => r.imported)) {
        sounds.playSuccess();
        await loadProviders();
      }
    } catch (e) {
      console.error("Failed to auto-detect keys", e);
    } finally {
      setAutoDetecting(false);
    }
  };

  useEffect(() => {
    loadProviders();
    loadFallbackConfig();
  }, []);

  const handleSaveKey = async (providerId: string) => {
    const key = inputKeys[providerId];
    if (!key || !key.trim()) return;

    sounds.playClick();
    setSavingProvider(providerId);
    try {
      await llm.saveApiKey(providerId, key);
      sounds.playSuccess();
      setInputKeys((prev) => ({ ...prev, [providerId]: "" }));
      await loadProviders();
    } catch (e) {
      console.error("Failed to save key in OS keyring", e);
    } finally {
      setSavingProvider(null);
    }
  };

  const handleDeleteKey = async (providerId: string) => {
    sounds.playClick();
    try {
      await llm.deleteApiKey(providerId);
      sounds.playSuccess();
      await loadProviders();
      setTestResults((prev) => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
    } catch (e) {
      console.error("Failed to delete key", e);
    }
  };

  const handleTestKey = async (providerId: string) => {
    sounds.playClick();
    setTestingProvider(providerId);
    try {
      const key = inputKeys[providerId];
      const baseUrl = customUrls[providerId];
      const result = await llm.testApiKey(providerId, key, baseUrl);
      if (result.success) {
        sounds.playSuccess();
      } else {
        sounds.playReceive();
      }
      setTestResults((prev) => ({ ...prev, [providerId]: result }));
    } catch (e) {
      console.error("Key test failed", e);
    } finally {
      setTestingProvider(null);
    }
  };

  const handleStrategyChange = async (strategy: FallbackStrategy) => {
    sounds.playClick();
    try {
      await llm.setFallbackStrategy(strategy);
      sounds.playSuccess();
      await loadFallbackConfig();
    } catch (e) {
      console.error("Failed to update fallback strategy", e);
    }
  };

  const handleToggleFallbackTarget = async (targetId: string) => {
    if (!fallbackChain) return;
    sounds.playClick();
    const updated = {
      ...fallbackChain,
      targets: fallbackChain.targets.map((t) =>
        t.id === targetId ? { ...t, enabled: !t.enabled } : t,
      ),
    };
    setFallbackChain(updated);
    try {
      await llm.setFallbackChain(updated);
    } catch (e) {
      console.error("Failed to update fallback chain", e);
    }
  };

  const handleToggleFallbackEnabled = async () => {
    if (!fallbackChain) return;
    sounds.playClick();
    const updated = {
      ...fallbackChain,
      enabled: !fallbackChain.enabled,
    };
    setFallbackChain(updated);
    try {
      await llm.setFallbackChain(updated);
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to update fallback chain", e);
    }
  };

  const handleMoveTarget = async (index: number, direction: "up" | "down") => {
    if (!fallbackChain) return;
    const targets = [...fallbackChain.targets];
    const targetIdx = direction === "up" ? index - 1 : index + 1;
    if (targetIdx < 0 || targetIdx >= targets.length) return;

    sounds.playClick();
    const [moved] = targets.splice(index, 1);
    targets.splice(targetIdx, 0, moved);

    const updated = {
      ...fallbackChain,
      strategy: "CustomOrder" as FallbackStrategy,
      targets,
    };
    setFallbackChain(updated);
    try {
      await llm.setFallbackChain(updated);
    } catch (e) {
      console.error("Failed to re-order fallback chain", e);
    }
  };

  const handleCheckUpdates = async () => {
    sounds.playClick();
    setCheckingUpdate(true);
    setUpdateStatus("Connecting to release channel...");
    try {
      const res = await checkForAppUpdates();
      if (res.updateAvailable) {
        sounds.playReceive();
        setHasUpdate(true);
        setUpdateStatus(`✨ New version ${res.version} available!`);
      } else {
        sounds.playSuccess();
        setHasUpdate(false);
        setUpdateStatus("LOCUS is up to date (latest release).");
      }
    } catch (err: unknown) {
      setUpdateStatus(err instanceof Error ? err.message : "Failed to query releases");
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleInstallUpdate = async () => {
    sounds.playClick();
    setInstallingUpdate(true);
    setUpdateStatus("Downloading update package...");
    try {
      const res = await installAppUpdate((downloaded, total) => {
        if (total) {
          const pct = Math.round((downloaded / total) * 100);
          setDownloadProgress(pct);
          setUpdateStatus(`Downloading: ${pct}%`);
        }
      });
      if (res.success) {
        sounds.playSuccess();
        setUpdateStatus("Update installed! Restarting application...");
      } else {
        setUpdateStatus(res.message ?? "Update installation failed");
      }
    } catch (err: unknown) {
      setUpdateStatus(err instanceof Error ? err.message : "Update failed");
    } finally {
      setInstallingUpdate(false);
    }
  };

  const handleExportDiagnostics = async () => {
    sounds.playClick();
    setExportingDiagnostics(true);
    try {
      const res = await system.exportDiagnostics();
      setDiagnosticResult(res);
      sounds.playSuccess();

      // Trigger automatic browser download of the JSON bundle
      const blob = new Blob([res.json_payload], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = res.file_name;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Failed to export diagnostic logs", e);
    } finally {
      setExportingDiagnostics(false);
    }
  };

  const handleCopyDiagnosticJson = () => {
    if (!diagnosticResult) return;
    sounds.playClick();
    navigator.clipboard.writeText(diagnosticResult.json_payload);
    setDiagnosticCopied(true);
    setTimeout(() => setDiagnosticCopied(false), 2500);
  };

  useEffect(() => {
    setSoundEnabled(sounds.isEnabled());
  }, []);

  const toggleSound = () => {
    const next = sounds.toggle();
    setSoundEnabled(next);
  };

  const scanWorkspace = async () => {
    if (!workspacePath.trim()) return;
    sounds.playClick();
    setScanning(true);
    try {
      const result = await import("../lib/api").then((m) => m.fs.scan(workspacePath));
      sounds.playSuccess();
      setState((s) => ({ ...s, workspace: result.index, workspaceRoot: workspacePath }));
    } catch (e) {
      console.error(e);
    } finally {
      setScanning(false);
    }
  };

  const startMesh = async () => {
    sounds.playClick();
    setMeshBusy(true);
    try {
      await network.start();
      const devices = await network.discover();
      sounds.playSuccess();
      setState((s) => ({ ...s, devices }));
    } catch (e) {
      console.error("Failed to start mesh", e);
    } finally {
      setMeshBusy(false);
    }
  };

  const stopMesh = async () => {
    sounds.playClick();
    setMeshBusy(true);
    try {
      await network.stop();
      setState((s) => ({ ...s, devices: [] }));
    } catch (e) {
      console.error("Failed to stop mesh", e);
    } finally {
      setMeshBusy(false);
    }
  };

  const autoSelectModel = async () => {
    sounds.playClick();
    setAutoSelectBusy(true);
    try {
      const selection = await llm.selectBestModel("codegen");
      await llm.setDefaultModel(selection.model_name, selection.backend);
      const freshModels = await llm.detectModels().catch(() => state.models);
      sounds.playSuccess();
      setState((s) => ({
        ...s,
        selectedModel: selection.model_name,
        models: freshModels,
      }));
    } catch (e) {
      console.error(e);
    } finally {
      setAutoSelectBusy(false);
    }
  };

  const testModelConnection = async () => {
    if (testingModel) return;
    sounds.playClick();
    setTestingModel(true);
    setModelTestStatus("Pinging local LLM backend…");

    const t0 = performance.now();
    try {
      await llm.generate({
        prompt: "Respond with the single word 'READY'.",
        model: state.selectedModel ?? undefined,
        temperature: 0.1,
        max_tokens: 16,
      });
      const t1 = performance.now();
      sounds.playSuccess();
      setModelTestStatus(`✅ Model '${state.selectedModel ?? "Default"}' is responsive (${Math.round(t1 - t0)}ms latency)`);
    } catch (e) {
      setModelTestStatus(`⚠️ Model connection failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setTestingModel(false);
    }
  };

  const handleUpdateTypography = (updates: Partial<TypographySettings>) => {
    const next = { ...typography, ...updates };
    setTypography(next);
    saveTypographySettings(next);
  };

  return (
    <div className="flex-1 overflow-y-auto p-5 space-y-6 max-w-4xl mx-auto w-full">
      <div className="flex items-center justify-between flex-wrap gap-3">
        <div>
          <h2 className="text-xl font-bold tracking-tight text-white">Application Settings</h2>
          <p className="text-xs text-locus-muted mt-0.5">Manage workspaces, AI model inference, developer typography, mesh clustering, and audio</p>
        </div>
        <div className="flex items-center gap-2">
          {onOpenOnboarding && (
            <button
              onClick={() => {
                sounds.playClick();
                onOpenOnboarding();
              }}
              className="btn-secondary text-xs py-1 px-3 font-mono text-violet-300 border-violet-500/30 hover:border-violet-500/60 flex items-center gap-1.5 shadow-sm"
              title="Re-open First-Run Onboarding Flow"
            >
              <span>🚀 Launch Setup Wizard</span>
            </button>
          )}
          <span className="text-[11px] font-mono text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1 rounded-full flex items-center gap-1.5">
            <span className="status-dot-online" />
            Zero External Telemetry
          </span>
        </div>
      </div>

      <div className="space-y-4">
        {/* Workspace Management */}
        <section className="panel p-5 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="section-title mb-0">Project Workspace</h3>
            <span className="text-[10px] text-locus-muted font-mono">Live File Watcher Active</span>
          </div>

          <div className="flex items-center gap-2">
            <div className="flex-1">
              <input
                className="input-dark font-mono text-xs"
                placeholder="Absolute path to your local project folder"
                value={workspacePath}
                onChange={(e) => setWorkspacePath(e.target.value)}
              />
            </div>
            <button
              className="btn-secondary text-xs shrink-0"
              onClick={() => {
                sounds.playClick();
                import("@tauri-apps/plugin-dialog").then(({ open }) =>
                  open({ directory: true, multiple: false }).then((path) => {
                    if (path && typeof path === "string") {
                      setWorkspacePath(path);
                    }
                  }),
                );
              }}
            >
              📂 Browse
            </button>
            <button
              className="btn-primary text-xs shrink-0"
              onClick={scanWorkspace}
              disabled={scanning || !workspacePath.trim()}
            >
              {scanning ? "Indexing…" : "⚡ Scan & Index"}
            </button>
          </div>

          {state.workspace && (
            <div className="mt-4 pt-3 border-t border-white/5 grid grid-cols-3 gap-3 text-center">
              <div className="p-3 rounded-lg bg-black/30 border border-white/5">
                <div className="text-xl font-mono font-bold text-white">{state.workspace.total_files}</div>
                <div className="text-[10px] text-locus-muted uppercase tracking-wider mt-0.5">Total Files</div>
              </div>
              <div className="p-3 rounded-lg bg-black/30 border border-white/5">
                <div className="text-xl font-mono font-bold text-violet-400">
                  {(state.workspace.total_size / 1024 / 1024).toFixed(1)} MB
                </div>
                <div className="text-[10px] text-locus-muted uppercase tracking-wider mt-0.5">Index Size</div>
              </div>
              <div className="p-3 rounded-lg bg-black/30 border border-white/5">
                <div className="text-xl font-mono font-bold text-emerald-400">
                  {Object.values(state.workspace.files).filter((f) => f.language).length}
                </div>
                <div className="text-[10px] text-locus-muted uppercase tracking-wider mt-0.5">Source Files</div>
              </div>
            </div>
          )}
        </section>

        {/* AI Model Management */}
        <section className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="section-title mb-0">Local AI Models</h3>
            <button
              onClick={autoSelectModel}
              disabled={autoSelectBusy}
              className="btn-secondary text-xs py-1 px-2.5"
            >
              {autoSelectBusy ? "Detecting…" : "✨ Auto-Detect Best Model"}
            </button>
          </div>

          {/* Local Hardware & Streaming Model Discovery Banner */}
          <LocalModelDiscoveryBanner
            onModelInstalled={() => {
              llm.detectModels().then((models) => {
                setState((s) => ({ ...s, models }));
              });
            }}
          />

          <div className="space-y-2">
            <label className="text-xs font-semibold text-locus-text">Selected Inference Model</label>
            <div className="flex items-center gap-2">
              <select
                className="input-dark text-xs font-mono py-2 bg-[#0a0c12]"
                value={state.selectedModel ?? ""}
                onChange={(e) => {
                  sounds.playClick();
                  const val = e.target.value;
                  setState((s) => ({ ...s, selectedModel: val || null }));
                }}
              >
                {state.models.length === 0 && <option value="">No local models detected (check Ollama / llama.cpp)</option>}
                {state.models.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name} ({m.backend}) {m.size ? `· ${m.size}` : ""}
                  </option>
                ))}
              </select>

              <button
                onClick={testModelConnection}
                disabled={testingModel}
                className="btn-secondary text-xs shrink-0 py-2"
              >
                {testingModel ? "Pinging…" : "🔍 Test Model"}
              </button>
            </div>

            {modelTestStatus && (
              <div className="text-xs font-mono p-2.5 rounded-lg bg-black/40 border border-white/10 text-zinc-300 animate-fade-in">
                {modelTestStatus}
              </div>
            )}
          </div>
        </section>

        {/* Cloud AI Providers & Keyring Vault */}
        <section className="panel p-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="section-title mb-0">Cloud AI Providers & Secure Keyring Vault</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-violet-500/15 text-violet-300 border border-violet-500/30">
                  🔐 OS Credential Manager (keyring-rs)
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Securely store and test your cloud API keys with hardware-level encryption (Windows Credential Manager / macOS Keychain / Linux Secret Service).
              </p>
            </div>
            <button
              onClick={() => {
                sounds.playClick();
                loadProviders();
              }}
              className="btn-ghost text-xs py-1 px-3 flex items-center gap-1.5 font-mono"
              title="Refresh provider status"
            >
              🔄 Refresh Status
            </button>
          </div>

          {/* Free Provider Radar & Permanent Free Quotas */}
          <FreeTierRadarBanner onKeyConfigured={loadProviders} />

          {/* Auto-Detect from Environment & .env Banner */}
          <div className="p-4 rounded-xl border border-violet-500/30 bg-gradient-to-r from-violet-950/40 via-indigo-950/20 to-black/40 flex flex-col md:flex-row items-start md:items-center justify-between gap-4 shadow-sm">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <span className="text-base">🪄</span>
                <h4 className="text-sm font-bold text-white font-mono">Auto-Detect & Import Keys</h4>
                <span className="text-[10px] px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 font-mono">
                  Zero Copy-Paste
                </span>
              </div>
              <p className="text-xs text-zinc-400">
                Scans system environment variables (e.g. <code className="text-violet-300">GEMINI_API_KEY</code>, <code className="text-violet-300">GROQ_API_KEY</code>) and local <code className="text-violet-300">.env</code> files, securely importing keys into OS Keyring.
              </p>
            </div>
            <button
              onClick={handleAutoDetectKeys}
              disabled={autoDetecting}
              className="btn-primary text-xs py-2 px-4 shrink-0 flex items-center gap-2 font-mono"
            >
              {autoDetecting ? (
                <>
                  <span className="animate-spin text-white">↻</span> Scanning…
                </>
              ) : (
                <>🔍 Scan & Import Keys</>
              )}
            </button>
          </div>

          {/* Auto-Detect Results Feed */}
          {detectedReports && (
            <div className="p-3.5 rounded-xl bg-black/40 border border-violet-500/30 space-y-2 animate-fade-in text-xs font-mono">
              <div className="flex items-center justify-between text-zinc-300">
                <span className="font-bold flex items-center gap-1.5">
                  <span>✨</span> Key Discovery Results:
                </span>
                <span className="text-[10px] text-zinc-500">
                  {detectedReports.length} key(s) detected
                </span>
              </div>
              {detectedReports.length === 0 ? (
                <div className="text-zinc-500 text-[11px] py-1">
                  No cloud API keys found in system environment or .env file.
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1">
                  {detectedReports.map((r) => (
                    <div
                      key={r.provider_id}
                      className={`p-2.5 rounded-lg border flex items-center justify-between ${
                        r.imported
                          ? "bg-emerald-950/20 border-emerald-500/30 text-emerald-300"
                          : "bg-zinc-900 border-white/5 text-zinc-400"
                      }`}
                    >
                      <div className="space-y-0.5 truncate">
                        <div className="font-bold text-white text-[11px]">{r.provider_name}</div>
                        <div className="text-[10px] opacity-75">{r.source}</div>
                      </div>
                      <div className="text-right shrink-0">
                        <div className="font-mono text-[10px]">{r.key_masked}</div>
                        <div className="text-[9px] text-emerald-400">✓ Saved to Keyring</div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5 pt-1">
            {providers.map((p) => {
              const result = testResults[p.provider_id];
              const isSaving = savingProvider === p.provider_id;
              const isTesting = testingProvider === p.provider_id;
              const keyUrlInfo = PROVIDER_KEY_URLS[p.provider_id];

              const getProviderIcon = (id: string) => {
                switch (id) {
                  case "gemini":
                    return "🤖";
                  case "groq":
                    return "⚡";
                  case "openrouter":
                    return "🌐";
                  case "deepseek":
                    return "🧠";
                  case "openai":
                    return "✨";
                  case "anthropic":
                    return "🎭";
                  default:
                    return "🔌";
                }
              };

              return (
                <div
                  key={p.provider_id}
                  className={`p-4 rounded-xl border transition-all flex flex-col justify-between ${
                    p.is_configured
                      ? "bg-[#0a0d16] border-violet-500/30 shadow-sm"
                      : "bg-black/30 border-white/5 opacity-85 hover:opacity-100"
                  }`}
                >
                  <div>
                    <div className="flex flex-wrap items-center justify-between gap-2 mb-2.5">
                      <div className="flex items-center gap-2">
                        <span className="text-base">{getProviderIcon(p.provider_id)}</span>
                        <span className="text-sm font-bold text-white font-mono">
                          {p.name}
                        </span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        {keyUrlInfo && (
                          <button
                            type="button"
                            onClick={() => openExternalLink(keyUrlInfo.url)}
                            className="text-[10px] font-mono px-2 py-0.5 rounded bg-violet-500/10 hover:bg-violet-500/25 text-violet-300 border border-violet-500/20 transition-all flex items-center gap-1"
                            title={`Open ${keyUrlInfo.label} in browser to get a free API key`}
                          >
                            <span>Get Key</span>
                            <span>↗</span>
                          </button>
                        )}
                        {p.is_configured ? (
                          <div className="flex items-center gap-1.5">
                            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                              ✓ {p.pool_size && p.pool_size > 1 ? `${p.pool_size} Keys in Pool` : "Configured"}
                            </span>
                            {p.keys && p.keys.some((k) => k.in_cooldown) && (
                              <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 border border-amber-500/30">
                                ⏳ Cooldown Active
                              </span>
                            )}
                          </div>
                        ) : (
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-zinc-500/15 text-zinc-400 border border-zinc-500/30">
                            Unconfigured
                          </span>
                        )}
                      </div>
                    </div>

                    <div className="text-[11px] font-mono text-zinc-400 mb-2 flex items-center justify-between">
                      <span>Default Model:</span>
                      <span className="text-violet-300 font-semibold bg-black/40 px-2 py-0.5 rounded border border-white/5">
                        {p.default_model}
                      </span>
                    </div>

                    {/* Key Pool Slots Status Display */}
                    {p.keys && p.keys.length > 0 && (
                      <div className="mb-2.5 p-2 rounded-lg bg-black/40 border border-white/5 space-y-1">
                        <div className="text-[10px] font-mono text-zinc-400 uppercase tracking-wider flex items-center justify-between">
                          <span>Active Key Pool ({p.keys.length}):</span>
                          <span className="text-emerald-400 font-semibold">{p.active_keys_count ?? p.keys.length} Ready</span>
                        </div>
                        <div className="flex flex-wrap gap-1.5 pt-0.5">
                          {p.keys.map((slot, sIdx) => (
                            <span
                              key={sIdx}
                              className={`text-[10px] font-mono px-2 py-0.5 rounded border flex items-center gap-1 ${
                                slot.in_cooldown
                                  ? "bg-amber-500/10 text-amber-300 border-amber-500/30"
                                  : "bg-white/5 text-zinc-300 border-white/10"
                              }`}
                              title={slot.in_cooldown ? `In 429 Cooldown (${slot.cooldown_remaining_secs}s remaining)` : "Active and ready for rotation"}
                            >
                              <span>{slot.in_cooldown ? "⏳" : "🔑"}</span>
                              <span>{slot.key_masked}</span>
                              {slot.in_cooldown && (
                                <span className="text-amber-400 font-bold ml-0.5">{slot.cooldown_remaining_secs}s</span>
                              )}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Input Controls */}
                    <div className="space-y-2">
                      <div className="flex items-start gap-2">
                        <div className="relative flex-1">
                          <textarea
                            rows={2}
                            className="input-dark text-xs font-mono py-2 pr-20 bg-[#06080d] border-white/10 w-full resize-none leading-relaxed"
                            placeholder={
                              p.is_configured
                                ? "Paste additional or replacement keys (separated by commas or newlines)…"
                                : `Enter ${p.name} API Key(s) (supports multiple keys for rotation)…`
                            }
                            value={inputKeys[p.provider_id] ?? ""}
                            onChange={(e) =>
                              setInputKeys((prev) => ({
                                ...prev,
                                [p.provider_id]: e.target.value,
                              }))
                            }
                          />
                          <div className="absolute right-2 top-2 flex flex-col gap-1">
                            <button
                              type="button"
                              onClick={() => handlePasteKey(p.provider_id)}
                              className="text-[10px] font-mono text-violet-400 hover:text-violet-300 bg-white/5 hover:bg-white/10 px-1.5 py-0.5 rounded transition-colors"
                              title="Paste API Key from Clipboard"
                            >
                              📋 Paste
                            </button>
                          </div>
                        </div>

                        <button
                          onClick={() => handleSaveKey(p.provider_id)}
                          disabled={isSaving || !inputKeys[p.provider_id]?.trim()}
                          className="btn-primary text-xs shrink-0 py-3 px-3 disabled:opacity-40"
                          title="Save key(s) to pool"
                        >
                          {isSaving ? "Saving…" : "💾 Save"}
                        </button>
                      </div>

                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => handleTestKey(p.provider_id)}
                          disabled={isTesting || (!p.is_configured && !inputKeys[p.provider_id]?.trim())}
                          className="btn-secondary text-xs flex-1 py-1.5 px-3 disabled:opacity-40 flex items-center justify-center gap-1.5 font-mono"
                        >
                          {isTesting ? (
                            <>
                              <span className="animate-spin text-violet-400">↻</span> Testing Ping…
                            </>
                          ) : (
                            <>⚡ Test Connection</>
                          )}
                        </button>

                        {p.is_configured && (
                          <button
                            onClick={() => handleDeleteKey(p.provider_id)}
                            className="text-xs px-2.5 py-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 border border-rose-500/30 transition-all shrink-0 font-mono"
                            title="Remove key from OS keyring"
                            aria-label={`Delete ${p.name} key from Keyring`}
                          >
                            🗑️ Delete
                          </button>
                        )}
                      </div>

                      {p.supports_custom_url && (
                        <input
                          className="input-dark text-xs font-mono py-1.5 bg-[#06080d] border-white/10 mt-1"
                          placeholder="Custom Base URL (e.g. http://localhost:8000/v1)"
                          value={customUrls[p.provider_id] ?? ""}
                          onChange={(e) =>
                            setCustomUrls((prev) => ({
                              ...prev,
                              [p.provider_id]: e.target.value,
                            }))
                          }
                        />
                      )}
                    </div>
                  </div>

                  {/* Live Test Results Display */}
                  {result && (
                    <div
                      className={`p-3 rounded-lg border text-xs font-mono space-y-1.5 mt-3 animate-fade-in ${
                        result.success
                          ? "bg-emerald-950/25 border-emerald-500/30 text-emerald-200"
                          : "bg-rose-950/25 border-rose-500/30 text-rose-200"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <span className={result.success ? "text-emerald-400 font-bold" : "text-rose-400 font-bold"}>
                            {result.success ? "✓ Active & Connected" : "✕ Connection Failed"}
                          </span>
                        </div>
                        {result.latency_ms > 0 && (
                          <span className="text-[10px] px-2 py-0.5 rounded bg-black/50 border border-white/10 text-emerald-300 font-bold">
                            ⏱️ {result.latency_ms}ms
                          </span>
                        )}
                      </div>

                      <div className="text-[11px] opacity-90">{result.message}</div>

                      {result.available_models.length > 0 && (
                        <div className="flex flex-wrap gap-1 pt-1">
                          {result.available_models.map((m) => (
                            <span
                              key={m}
                              className="text-[10px] px-2 py-0.5 rounded bg-black/60 text-zinc-300 border border-white/10"
                            >
                              {m}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </section>

        {/* Auto-Fallback Chain Priority Router */}
        {fallbackChain && (
          <section className="panel p-5 space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="section-title mb-0">Auto-Fallback Chain Router</h3>
                  <button
                    onClick={handleToggleFallbackEnabled}
                    className={`text-[10px] font-mono px-2.5 py-0.5 rounded-full border transition-all ${
                      fallbackChain.enabled
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40 font-bold"
                        : "bg-zinc-500/20 text-zinc-400 border-zinc-500/30"
                    }`}
                  >
                    {fallbackChain.enabled ? "● Auto-Failover: Active" : "○ Disabled"}
                  </button>
                </div>
                <p className="text-xs text-locus-muted mt-0.5">
                  Seamlessly re-routes inference to backup providers if your primary local or cloud model encounters rate limits or timeouts.
                </p>
              </div>

              {/* Strategy Preset Switcher */}
              <div className="flex items-center bg-black/40 rounded-xl p-1 border border-white/10 text-xs font-mono">
                <button
                  onClick={() => handleStrategyChange("LocalFirst")}
                  className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 ${
                    fallbackChain.strategy === "LocalFirst"
                      ? "bg-violet-600 text-white font-bold shadow-glow-violet"
                      : "text-zinc-400 hover:text-white"
                  }`}
                >
                  <span>🔒</span> Local First
                </button>
                <button
                  onClick={() => handleStrategyChange("SpeedFirst")}
                  className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 ${
                    fallbackChain.strategy === "SpeedFirst"
                      ? "bg-violet-600 text-white font-bold shadow-glow-violet"
                      : "text-zinc-400 hover:text-white"
                  }`}
                >
                  <span>⚡</span> Speed First
                </button>
                <button
                  onClick={() => handleStrategyChange("CloudFirst")}
                  className={`px-3 py-1.5 rounded-lg transition-all flex items-center gap-1.5 ${
                    fallbackChain.strategy === "CloudFirst"
                      ? "bg-violet-600 text-white font-bold shadow-glow-violet"
                      : "text-zinc-400 hover:text-white"
                  }`}
                >
                  <span>🌐</span> Cloud First
                </button>
              </div>
            </div>

            {/* Visual Failover Flow Pipeline */}
            <div className="p-4 rounded-xl bg-black/40 border border-white/5 space-y-3">
              <div className="text-[11px] font-mono text-zinc-400 flex items-center justify-between">
                <span className="font-semibold uppercase tracking-wider text-zinc-300">
                  Visual Failover Sequence:
                </span>
                <span className="text-[10px] text-violet-300">
                  {fallbackChain.strategy === "LocalFirst"
                    ? "🛡️ On-Device First → Cloud Backup"
                    : fallbackChain.strategy === "SpeedFirst"
                    ? "⚡ Ultra-Fast Cloud (Groq) → Local → Free Tier"
                    : "🌐 Cloud First → Local Offline Fallback"}
                </span>
              </div>

              <div className="flex flex-wrap items-center gap-2">
                {fallbackChain.targets
                  .filter((t) => t.enabled)
                  .map((target, idx, arr) => (
                    <div key={target.id || `target-${idx}`} className="flex items-center gap-2">
                      <div className="px-3 py-1.5 rounded-lg bg-[#0d111d] border border-violet-500/30 flex items-center gap-2 text-xs font-mono">
                        <span className="w-4 h-4 rounded-full bg-violet-600/30 text-violet-300 border border-violet-500/40 flex items-center justify-center text-[10px] font-bold">
                          {idx + 1}
                        </span>
                        <span className="text-white font-semibold">{target.label}</span>
                        {target.is_local ? (
                          <span className="text-[9px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                            Local
                          </span>
                        ) : (
                          <span className="text-[9px] px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-300 border border-blue-500/20">
                            Cloud
                          </span>
                        )}
                      </div>
                      {idx < arr.length - 1 && (
                        <span className="text-violet-400 font-mono text-xs font-bold">
                          ⟶
                        </span>
                      )}
                    </div>
                  ))}
              </div>
            </div>

            {/* Target Re-Ordering & Toggles List */}
            <div className="space-y-2 pt-1">
              <div className="text-[11px] font-semibold uppercase tracking-wider text-zinc-400 px-1">
                Configure Chain Priority & Target Models:
              </div>

              <div className="space-y-1.5">
                {fallbackChain.targets.map((target, idx) => (
                  <div
                    key={target.id || `target-cfg-${idx}`}
                    className={`flex items-center justify-between p-3 rounded-xl border transition-all ${
                      target.enabled
                        ? "bg-[#0a0d16] border-white/10 hover:border-violet-500/30"
                        : "bg-black/30 border-white/5 opacity-50"
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span className="w-5 h-5 rounded-full bg-black/60 border border-white/10 flex items-center justify-center text-[10px] font-mono text-violet-400 font-bold">
                        {idx + 1}
                      </span>
                      <input
                        type="checkbox"
                        checked={target.enabled}
                        onChange={() => handleToggleFallbackTarget(target.id)}
                        className="rounded border-zinc-700 bg-black text-violet-600 focus:ring-0 w-4 h-4 cursor-pointer"
                      />
                      <div>
                        <span className="text-xs font-mono font-bold text-white">
                          {target.label}
                        </span>
                        <span className="text-[10px] font-mono text-zinc-400 ml-2">
                          {target.is_local ? "(On-Device Engine)" : "(Cloud Remote API)"}
                        </span>
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      {target.preferred_model && (
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-black/40 text-violet-300 border border-white/10">
                          {target.preferred_model}
                        </span>
                      )}
                      <button
                        onClick={() => handleMoveTarget(idx, "up")}
                        disabled={idx === 0}
                        className="px-2 py-1 rounded bg-white/5 hover:bg-white/10 text-zinc-300 disabled:opacity-20 text-xs font-mono"
                        title="Move Up in priority"
                      >
                        ▲
                      </button>
                      <button
                        onClick={() => handleMoveTarget(idx, "down")}
                        disabled={idx === fallbackChain.targets.length - 1}
                        className="px-2 py-1 rounded bg-white/5 hover:bg-white/10 text-zinc-300 disabled:opacity-20 text-xs font-mono"
                        title="Move Down in priority"
                      >
                        ▼
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>
        )}

        {/* Privacy & Mesh Compute */}
        <section className="panel p-5 space-y-4">
          <h3 className="section-title">Privacy & Compute Architecture</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <button
              onClick={() => {
                sounds.playClick();
                setPrivacyMode("local");
              }}
              className={`p-4 rounded-xl border text-left transition-all ${
                state.privacyMode === "local"
                  ? "bg-violet-500/15 border-violet-500/50 shadow-glow-violet"
                  : "bg-locus-card/60 border-locus-border hover:border-locus-muted"
              }`}
            >
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-sm font-bold text-white flex items-center gap-2">
                  🔒 Air-Gapped Local
                </span>
                {state.privacyMode === "local" && <span className="tag-active">Active</span>}
              </div>
              <p className="text-xs text-locus-muted leading-relaxed">
                100% of code parsing, LLM generation, and agent sandboxing runs strictly on this physical machine.
              </p>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setPrivacyMode("hybrid");
              }}
              className={`p-4 rounded-xl border text-left transition-all ${
                state.privacyMode === "hybrid"
                  ? "bg-emerald-500/15 border-emerald-500/50 shadow-glow-emerald"
                  : "bg-locus-card/60 border-locus-border hover:border-locus-muted"
              }`}
            >
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-sm font-bold text-white flex items-center gap-2">
                  ⚡ P2P Local Mesh
                </span>
                {state.privacyMode === "hybrid" && <span className="tag-active text-emerald-400 border-emerald-500/40 bg-emerald-500/15">Active</span>}
              </div>
              <p className="text-xs text-locus-muted leading-relaxed">
                Shares heavy agent compiling and benchmark tasks across your trusted LAN devices.
              </p>
            </button>
          </div>

          <div className="pt-2 flex items-center justify-between border-t border-white/5">
            <div>
              <div className="text-xs font-semibold text-white">LAN P2P Discovery Service</div>
              <div className="text-[11px] text-locus-muted">
                {state.devices.length} peer node(s) currently discovered
              </div>
            </div>
            <button
              onClick={state.devices.length > 0 ? stopMesh : startMesh}
              disabled={meshBusy}
              className="btn-secondary text-xs py-1.5 px-3"
            >
              {meshBusy ? "Working…" : state.devices.length > 0 ? "Disconnect Mesh" : "🌐 Start Mesh Discovery"}
            </button>
          </div>
        </section>

        {/* Developer Typography & Themes Settings */}
        <section className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-white">Developer Typography & Themes</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-violet-500/10 text-violet-300 border border-violet-500/20">
                  ✨ Instant Dynamic Switch
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Customize coding monospace typeface, font size, and ligatures across Chat, DiffViewer, and editor blocks without page reload.
              </p>
            </div>
          </div>

          {/* Font Selector Cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2.5 pt-1">
            {AVAILABLE_FONTS.map((f) => {
              const isSelected = typography.monoFont === f.id;
              return (
                <button
                  key={f.id}
                  type="button"
                  onClick={() => {
                    sounds.playClick();
                    handleUpdateTypography({ monoFont: f.id });
                  }}
                  className={`p-3 rounded-xl border text-left transition-all ${
                    isSelected
                      ? "bg-locus-violet/20 border-locus-violet/60 shadow-glow-violet ring-1 ring-locus-violet/50"
                      : "bg-black/40 border-white/10 hover:border-white/20"
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs font-bold text-white">{f.name}</span>
                    {isSelected && <span className="text-xs text-locus-violet font-bold">✓ Active</span>}
                  </div>
                  <p className="text-[11px] text-zinc-400 leading-snug">{f.description}</p>
                </button>
              );
            })}
          </div>

          {/* Font Size & Ligatures Controls */}
          <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <span className="text-xs font-semibold text-zinc-300">
                Code & Diff Font Size: <span className="font-mono text-violet-400 font-bold">{typography.codeFontSize}px</span>
              </span>
              <div className="flex items-center gap-3">
                <span className="text-[11px] font-mono text-zinc-500">12px</span>
                <input
                  type="range"
                  min={12}
                  max={18}
                  step={1}
                  value={typography.codeFontSize}
                  onChange={(e) =>
                    handleUpdateTypography({
                      codeFontSize: parseInt(e.target.value, 10),
                      chatFontSize: parseInt(e.target.value, 10),
                    })
                  }
                  className="w-48 accent-locus-violet cursor-pointer"
                />
                <span className="text-[11px] font-mono text-zinc-500">18px</span>
              </div>
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-white/5">
              <label className="flex items-center gap-2 text-xs text-zinc-300 cursor-pointer">
                <input
                  type="checkbox"
                  checked={typography.ligatures}
                  onChange={(e) => handleUpdateTypography({ ligatures: e.target.checked })}
                  className="rounded bg-black border-white/20 text-locus-violet focus:ring-locus-violet"
                />
                <span>Enable Programming Ligatures (<code className="font-mono text-violet-300">=&gt;</code>, <code className="font-mono text-violet-300">!=</code>, <code className="font-mono text-violet-300">===</code>, <code className="font-mono text-violet-300">&lt;=</code>)</span>
              </label>
            </div>
          </div>

          {/* Interactive Live Code Preview */}
          <div className="space-y-1.5">
            <span className="text-[10px] uppercase font-bold text-zinc-500 tracking-wider">
              Live Preview
            </span>
            <div
              className="p-3.5 rounded-xl bg-[#06080e] border border-white/10 text-zinc-200 overflow-x-auto code-editor-font transition-all"
              style={{
                fontFamily: AVAILABLE_FONTS.find((f) => f.id === typography.monoFont)?.family,
                fontSize: `${typography.codeFontSize}px`,
              }}
            >
              <pre className="m-0 leading-relaxed font-mono">
                {`pub async fn execute_task(ctx: &Context) -> Result<()> {\n    let is_valid = hash != 0x00 && status === "Ready";\n    tracing::info!("LOCUS Dynamic Typography => {}", is_valid);\n    Ok(())\n}`}
              </pre>
            </div>
          </div>
        </section>

        {/* Modular Skills & Agent Tool Calling */}
        <section className="panel p-5 space-y-4">
          <SkillsManager />
        </section>

        {/* UI & Audio Feedback */}
        <section className="panel p-5 flex items-center justify-between">
          <div>
            <h3 className="text-xs font-semibold text-white">Interactive Audio Feedback</h3>
            <p className="text-[11px] text-locus-muted mt-0.5">Synthesized micro-tones for message events, clicks, and test verification</p>
          </div>
          <button
            onClick={toggleSound}
            className={`px-3.5 py-1.5 rounded-lg border text-xs font-mono font-semibold transition-all ${
              soundEnabled
                ? "bg-violet-500/20 border-violet-500/50 text-violet-300 shadow-glow-violet"
                : "bg-white/5 border-white/10 text-locus-muted"
            }`}
          >
            {soundEnabled ? "🔊 Sound: Enabled" : "🔇 Sound: Muted"}
          </button>
        </section>

        {/* Tauri Updater & Security Signatures */}
        <section className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-white">Software Updates & Release Channel</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                  🔐 Minisign Ed25519 Verified
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Cryptographically signed binary updates streamed from official GitHub Releases.
              </p>
            </div>
            <button
              onClick={handleCheckUpdates}
              disabled={checkingUpdate || installingUpdate}
              className="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 font-mono"
            >
              {checkingUpdate ? (
                <>
                  <span className="animate-spin text-violet-400">↻</span> Checking...
                </>
              ) : (
                <>🔄 Check for Updates</>
              )}
            </button>
          </div>

          <div className="p-3.5 rounded-xl bg-black/30 border border-white/5 flex items-center justify-between font-mono text-xs">
            <div className="space-y-1">
              <div className="text-zinc-400">
                Current Build: <span className="text-white font-semibold">v0.1.0-alpha</span>
              </div>
              {updateStatus && (
                <div className="text-[11px] text-violet-300 flex items-center gap-1.5">
                  <span>●</span> {updateStatus}
                </div>
              )}
              {downloadProgress !== null && (
                <div className="w-48 bg-white/10 rounded-full h-1.5 overflow-hidden mt-1">
                  <div
                    className="bg-violet-500 h-full transition-all duration-300"
                    style={{ width: `${downloadProgress}%` }}
                  />
                </div>
              )}
            </div>

            {hasUpdate && (
              <button
                onClick={handleInstallUpdate}
                disabled={installingUpdate}
                className="btn-primary text-xs py-1.5 px-3.5 font-semibold animate-pulse"
              >
                {installingUpdate ? "Downloading & Installing..." : "Install Update & Restart"}
              </button>
            )}
          </div>
        </section>

        {/* Interactive Newton & Living Architecture Tree */}
        <section className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-white">Interactive Newton & Architecture Tree</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-violet-500/10 text-violet-300 border border-violet-500/20">
                  🍎 Pure SVG Assistant
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Displays the living codebase architecture tree and animated Newton companion at the bottom-left with real-time Task DAG execution reflection.
              </p>
            </div>

            <button
              onClick={() => {
                sounds.playClick();
                const next = !showNewton;
                setShowNewton(next);
                try {
                  localStorage.setItem("locus_show_newton_companion", String(next));
                } catch {
                  // ignore
                }
              }}
              className={`px-4 py-2 rounded-xl text-xs font-mono font-semibold transition-all border ${
                showNewton
                  ? "bg-violet-600/30 text-violet-300 border-violet-500/50 shadow-glow-violet"
                  : "bg-white/5 text-zinc-400 border-white/10 hover:text-white"
              }`}
            >
              {showNewton ? "✓ Enabled (Visible)" : "✕ Disabled (Hidden)"}
            </button>
          </div>
        </section>

        {/* Spring Physics Motion & Animation Engine */}
        <section className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-white">Spring Physics & High-FPS Motion Engine</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                  ⚡ 60 FPS GPU-Accelerated
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Uses cubic-bezier spring curves (120ms - 180ms) and hardware GPU layers for instant feedback. Toggle to disable if you prefer reduced motion.
              </p>
            </div>

            <button
              onClick={() => {
                sounds.playClick();
                const isCurrentlyReduced = document.documentElement.classList.contains("reduce-motion");
                const next = !isCurrentlyReduced;
                if (next) {
                  document.documentElement.classList.add("reduce-motion");
                  try { localStorage.setItem("locus_reduce_motion", "true"); } catch {}
                } else {
                  document.documentElement.classList.remove("reduce-motion");
                  try { localStorage.setItem("locus_reduce_motion", "false"); } catch {}
                }
              }}
              className="px-4 py-2 rounded-xl text-xs font-mono font-semibold transition-all border bg-white/5 text-zinc-300 border-white/10 hover:text-white"
            >
              ⚡ Toggle Motion / Reduced Mode
            </button>
          </div>
        </section>

        {/* Diagnostic Logs & Technical Support */}
        <section className="panel p-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-white">Diagnostic Logs & Technical Support</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                  🛡️ Zero Privacy Leakage · Auto-Masked
                </span>
              </div>
              <p className="text-xs text-locus-muted mt-0.5">
                Collects system hardware info, runtime status, and sanitized error traces with automatic redaction of API keys, IPs, and user paths.
              </p>
            </div>

            <button
              onClick={handleExportDiagnostics}
              disabled={exportingDiagnostics}
              className="btn-primary text-xs py-2 px-4 flex items-center gap-2 font-mono shadow-sm hover:scale-[1.02] active:scale-[0.98] transition-all"
            >
              {exportingDiagnostics ? (
                <>
                  <span className="animate-spin text-white">↻</span> Packaging Diagnostics…
                </>
              ) : (
                <>📦 Export Diagnostic Logs (.json)</>
              )}
            </button>
          </div>

          {/* Privacy & Security Guarantees Bar */}
          <div className="grid grid-cols-3 gap-2.5 pt-1 text-[11px] font-mono">
            <div className="p-2.5 rounded-lg bg-black/30 border border-white/5 flex items-center gap-2 text-zinc-300">
              <span className="text-emerald-400">✓</span>
              <span>API Keys Redacted (`[REDACTED_API_KEY]`)</span>
            </div>
            <div className="p-2.5 rounded-lg bg-black/30 border border-white/5 flex items-center gap-2 text-zinc-300">
              <span className="text-emerald-400">✓</span>
              <span>User Paths Scrubbed (`Users/[USER]`)</span>
            </div>
            <div className="p-2.5 rounded-lg bg-black/30 border border-white/5 flex items-center gap-2 text-zinc-300">
              <span className="text-emerald-400">✓</span>
              <span>Non-Sensitive System & Mesh Stats</span>
            </div>
          </div>

          {/* Export Results Box */}
          {diagnosticResult && (
            <div className="p-4 rounded-xl bg-[#090c14] border border-violet-500/30 space-y-3 animate-fade-in">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-mono font-bold text-emerald-400">✓ Export Ready:</span>
                  <span className="text-xs font-mono text-white bg-black/40 px-2 py-0.5 rounded border border-white/10">
                    {diagnosticResult.file_name}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleCopyDiagnosticJson}
                    className="text-xs px-3 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-zinc-200 border border-white/10 font-mono transition-all"
                  >
                    {diagnosticCopied ? "✓ Copied JSON!" : "📋 Copy JSON"}
                  </button>
                  <button
                    onClick={() => {
                      sounds.playClick();
                      const blob = new Blob([diagnosticResult.json_payload], { type: "application/json" });
                      const url = URL.createObjectURL(blob);
                      const a = document.createElement("a");
                      a.href = url;
                      a.download = diagnosticResult.file_name;
                      document.body.appendChild(a);
                      a.click();
                      document.body.removeChild(a);
                      URL.revokeObjectURL(url);
                    }}
                    className="text-xs px-3 py-1 rounded-lg bg-violet-600/30 hover:bg-violet-600/50 text-violet-200 border border-violet-500/40 font-mono transition-all"
                  >
                    ⬇️ Download Again
                  </button>
                </div>
              </div>

              <div className="text-[11px] font-mono text-zinc-400 bg-black/30 p-2.5 rounded-lg border border-white/5">
                {diagnosticResult.summary}
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

export default Settings;