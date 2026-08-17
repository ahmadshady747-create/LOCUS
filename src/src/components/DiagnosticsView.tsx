import { useEffect, useState } from "react";
import type {
  AdrLedger,
  AppState,
  DiagnosticReport,
  ExportDiagnosticResult,
  NegativeMemoryEntry,
} from "../types";
import { adrLedger, system } from "../lib/api";
import { sounds } from "../lib/sound";
import SkillsManager from "./SkillsManager";
import NetworkView from "./NetworkView";

interface DiagnosticsViewProps {
  state?: AppState;
  onNavigate?: (tab: string) => void;
}

export default function DiagnosticsView({ state, onNavigate }: DiagnosticsViewProps) {
  const [subTab, setSubTab] = useState<"diagnostics" | "skills" | "network" | "adr">("diagnostics");
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportDiagnosticResult | null>(null);
  const [copied, setCopied] = useState(false);

  // ADR Ledger state
  const [ledger, setLedger] = useState<AdrLedger | null>(null);
  const [loadingAdr, setLoadingAdr] = useState(false);
  const [newPatternName, setNewPatternName] = useState("");
  const [newTargetModule, setNewTargetModule] = useState("");
  const [newReason, setNewReason] = useState("");
  const [newAlternative, setNewAlternative] = useState("");
  const [showAddNegativeModal, setShowAddNegativeModal] = useState(false);

  const loadDiagnostics = async () => {
    setLoading(true);
    try {
      const res = await system.getDiagnostics();
      setReport(res);
    } catch (e) {
      console.error("Failed to get diagnostics", e);
    } finally {
      setLoading(false);
    }
  };

  const loadAdrLedger = async () => {
    const ws = state?.workspaceRoot || ".";
    setLoadingAdr(true);
    try {
      const res = await adrLedger.get(ws);
      setLedger(res);
    } catch (e) {
      console.error("Failed to load ADR ledger", e);
    } finally {
      setLoadingAdr(false);
    }
  };

  useEffect(() => {
    loadDiagnostics();
    loadAdrLedger();
  }, [state?.workspaceRoot]);

  const handleAddNegative = async () => {
    if (!newPatternName.trim() || !newReason.trim()) return;
    const ws = state?.workspaceRoot || ".";
    sounds.playClick();

    const entry: NegativeMemoryEntry = {
      id: `NEG-${Date.now().toString().slice(-4)}`,
      pattern_name: newPatternName,
      severity: "forbidden",
      target_module: newTargetModule || "*",
      reason: newReason,
      forbidden_snippets: [],
      recommended_alternative: newAlternative || "Follow standard patterns.",
      created_at: new Date().toISOString(),
    };

    try {
      const updated = await adrLedger.addNegative(ws, entry);
      setLedger(updated);
      setNewPatternName("");
      setNewTargetModule("");
      setNewReason("");
      setNewAlternative("");
      setShowAddNegativeModal(false);
      sounds.playSuccess();
    } catch (err) {
      console.error("Failed to add negative memory:", err);
    }
  };

  const handleExport = async () => {
    sounds.playClick();
    setExporting(true);
    try {
      const res = await system.exportDiagnostics();
      setExportResult(res);
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to export diagnostics", e);
    } finally {
      setExporting(false);
    }
  };

  const handleCopyJson = () => {
    if (!exportResult && !report) return;
    sounds.playClick();
    const payload = exportResult?.json_payload ?? JSON.stringify(report, null, 2);
    navigator.clipboard.writeText(payload);
    setCopied(true);
    setTimeout(() => setCopied(false), 2500);
  };

  const handleDownload = () => {
    if (!exportResult && !report) return;
    sounds.playClick();
    const payload = exportResult?.json_payload ?? JSON.stringify(report, null, 2);
    const fileName = exportResult?.file_name ?? `locus-diagnostics-${Date.now()}.json`;
    const blob = new Blob([payload], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = fileName;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="h-full flex flex-col overflow-hidden bg-[#07090e]">
      {/* Sub-Navigation Header */}
      <div className="px-6 pt-4 pb-3 border-b border-white/5 bg-[#090b12] flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <div className="flex items-center bg-black/40 p-0.5 rounded-lg border border-white/10 text-xs font-mono">
            <button
              onClick={() => {
                sounds.playClick();
                setSubTab("diagnostics");
              }}
              className={`px-3 py-1.5 rounded-md transition-colors flex items-center gap-1.5 ${
                subTab === "diagnostics"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>🩺</span>
              <span>Privacy Diagnostics</span>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setSubTab("skills");
              }}
              className={`px-3 py-1.5 rounded-md transition-colors flex items-center gap-1.5 ${
                subTab === "skills"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>🧩</span>
              <span>Modular Skills Engine</span>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setSubTab("network");
              }}
              className={`px-3 py-1.5 rounded-md transition-colors flex items-center gap-1.5 ${
                subTab === "network"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>🌐</span>
              <span>P2P Mesh Network</span>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setSubTab("adr");
              }}
              className={`px-3 py-1.5 rounded-md transition-colors flex items-center gap-1.5 ${
                subTab === "adr"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>📜</span>
              <span>ADR & Negative Memory</span>
            </button>
          </div>
        </div>

        {subTab === "diagnostics" && (
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                sounds.playClick();
                loadDiagnostics();
              }}
              disabled={loading}
              className="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 font-mono"
            >
              {loading ? "↻ Auditing…" : "🔄 Refresh"}
            </button>

            <button
              onClick={handleExport}
              disabled={exporting}
              className="btn-primary text-xs py-1.5 px-3 flex items-center gap-1.5 font-mono"
            >
              {exporting ? "⏳ Exporting…" : "📦 Export Redacted Bundle"}
            </button>
          </div>
        )}

        {subTab === "adr" && (
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                sounds.playClick();
                loadAdrLedger();
              }}
              disabled={loadingAdr}
              className="btn-secondary text-xs py-1.5 px-3 flex items-center gap-1.5 font-mono"
            >
              {loadingAdr ? "↻ Syncing…" : "🔄 Refresh"}
            </button>
            <button
              onClick={() => {
                sounds.playClick();
                setShowAddNegativeModal(true);
              }}
              className="btn-primary text-xs py-1.5 px-3 flex items-center gap-1.5 font-mono"
            >
              + Add Forbidden Anti-Pattern
            </button>
          </div>
        )}
      </div>

      {/* SubTab Content */}
      <div className="flex-1 overflow-hidden">
        {subTab === "skills" && <SkillsManager />}
        {subTab === "network" && state && <NetworkView state={state} onNavigate={onNavigate as any} />}
        {subTab === "adr" && (
          <div className="h-full overflow-y-auto p-6 space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-sm font-bold text-white font-mono flex items-center gap-2">
                  <span>📜</span> Workspace Architectural Decision Records & Negative Memory Ledger
                </h2>
                <p className="text-xs text-zinc-400 mt-0.5">
                  Stored at <code className="text-violet-300 font-mono">.locus/adr.json</code>. Enforces architectural choices and automatically warns against past failed anti-patterns.
                </p>
              </div>
            </div>

            {/* Negative Memories (Anti-patterns Guardrails) */}
            <div className="space-y-3">
              <h3 className="text-xs font-bold text-red-400 font-mono uppercase tracking-wider flex items-center gap-2">
                <span>🚫</span> Negative Memories & Forbidden Anti-Patterns ({ledger?.negative_memories.length ?? 0})
              </h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {ledger?.negative_memories.map((entry) => (
                  <div
                    key={entry.id}
                    className="p-4 rounded-xl bg-[#140c0c]/80 border border-red-500/30 space-y-2 text-xs font-mono"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-bold text-white">{entry.pattern_name}</span>
                      <span className="text-[9px] uppercase px-1.5 py-0.5 rounded bg-red-500/20 text-red-300 font-bold">
                        {entry.severity}
                      </span>
                    </div>
                    <div className="text-zinc-400 text-[11px]">
                      <span className="text-zinc-500">Target Module:</span> {entry.target_module}
                    </div>
                    <p className="text-red-300/90 text-[11px]">{entry.reason}</p>
                    <div className="p-2 rounded bg-black/40 border border-white/5 text-[10px] text-emerald-300">
                      <span className="font-bold text-zinc-400">Alternative:</span> {entry.recommended_alternative}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Architectural Decision Records (ADRs) */}
            <div className="space-y-3 pt-4">
              <h3 className="text-xs font-bold text-violet-400 font-mono uppercase tracking-wider flex items-center gap-2">
                <span>🏛️</span> Architectural Decision Records ({ledger?.records.length ?? 0})
              </h3>
              <div className="space-y-3">
                {ledger?.records.map((rec) => (
                  <div
                    key={rec.id}
                    className="p-4 rounded-xl bg-[#0c0f1a]/80 border border-violet-500/30 space-y-2 text-xs font-mono"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="text-violet-300 font-bold">{rec.id}: {rec.title}</span>
                        <span className="text-[9px] uppercase px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                          {rec.status}
                        </span>
                      </div>
                      <div className="flex gap-1">
                        {rec.tags.map((t) => (
                          <span key={t} className="text-[9px] px-1.5 py-0.5 rounded bg-white/5 text-zinc-400">
                            #{t}
                          </span>
                        ))}
                      </div>
                    </div>
                    <p className="text-zinc-300 text-[11px]">{rec.decision}</p>
                    <div className="text-[10px] text-zinc-500">
                      Context: {rec.context}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Modal to Add Forbidden Anti-Pattern */}
            {showAddNegativeModal && (
              <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
                <div className="bg-[#0b0e17] border border-red-500/40 rounded-2xl w-full max-w-lg p-5 space-y-4 shadow-2xl animate-fade-in font-mono text-xs">
                  <div className="flex items-center justify-between border-b border-white/10 pb-3">
                    <h3 className="text-sm font-bold text-red-300 flex items-center gap-2">
                      <span>🚫</span> Register Forbidden Anti-Pattern
                    </h3>
                    <button
                      onClick={() => setShowAddNegativeModal(false)}
                      className="text-zinc-500 hover:text-white"
                    >
                      ✕
                    </button>
                  </div>

                  <div className="space-y-3">
                    <div>
                      <label className="text-[11px] text-zinc-400 block mb-1">Pattern Name</label>
                      <input
                        type="text"
                        value={newPatternName}
                        onChange={(e) => setNewPatternName(e.target.value)}
                        placeholder="e.g. Unscoped Mutex across await"
                        className="w-full bg-black/50 border border-white/10 rounded-lg p-2 text-white text-xs"
                      />
                    </div>

                    <div>
                      <label className="text-[11px] text-zinc-400 block mb-1">Target Module Path</label>
                      <input
                        type="text"
                        value={newTargetModule}
                        onChange={(e) => setNewTargetModule(e.target.value)}
                        placeholder="e.g. crates/locus-agents or * for all"
                        className="w-full bg-black/50 border border-white/10 rounded-lg p-2 text-white text-xs"
                      />
                    </div>

                    <div>
                      <label className="text-[11px] text-zinc-400 block mb-1">Reason for Rejection</label>
                      <textarea
                        value={newReason}
                        onChange={(e) => setNewReason(e.target.value)}
                        placeholder="e.g. Causes deadlocks in async runtime threads"
                        rows={2}
                        className="w-full bg-black/50 border border-white/10 rounded-lg p-2 text-white text-xs"
                      />
                    </div>

                    <div>
                      <label className="text-[11px] text-zinc-400 block mb-1">Recommended Alternative</label>
                      <input
                        type="text"
                        value={newAlternative}
                        onChange={(e) => setNewAlternative(e.target.value)}
                        placeholder="e.g. Use tokio::sync::Mutex or scope before await"
                        className="w-full bg-black/50 border border-white/10 rounded-lg p-2 text-white text-xs"
                      />
                    </div>
                  </div>

                  <div className="flex justify-end gap-2 pt-2 border-t border-white/10">
                    <button
                      onClick={() => setShowAddNegativeModal(false)}
                      className="px-3 py-1.5 bg-white/5 hover:bg-white/10 text-zinc-300 rounded-lg text-xs"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleAddNegative}
                      disabled={!newPatternName.trim() || !newReason.trim()}
                      className="px-4 py-1.5 bg-red-600 hover:bg-red-500 disabled:opacity-40 text-white rounded-lg text-xs font-semibold"
                    >
                      Save Anti-Pattern
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
        {subTab === "diagnostics" && (
          <div className="h-full overflow-y-auto p-6 space-y-6">
            {/* Redaction Notice */}
            <div className="glass-panel p-4 rounded-xl border border-emerald-500/20 bg-emerald-950/10 flex items-center gap-3">
              <span className="text-xl">🛡️</span>
              <div className="text-xs text-zinc-300">
                <span className="font-bold text-emerald-400">Zero-Leak Guarantee: </span>
                All API keys, Bearer tokens, private paths, and IP addresses are masked via deterministic regex pipelines before leaving memory.
              </div>
            </div>

            {/* Export Summary Banner */}
            {exportResult && (
              <div className="glass-panel p-4 rounded-xl border border-violet-500/40 bg-violet-950/20 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-emerald-400 text-base">✓</span>
                    <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                      Exported Anonymous Snapshot
                    </h3>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleCopyJson}
                      className="px-2.5 py-1 rounded-md bg-white/10 hover:bg-white/20 text-xs font-mono text-zinc-200 transition-colors"
                    >
                      {copied ? "✓ Copied JSON" : "📋 Copy JSON"}
                    </button>
                    <button
                      onClick={handleDownload}
                      className="px-2.5 py-1 rounded-md bg-violet-600 hover:bg-violet-500 text-xs font-mono text-white transition-colors"
                    >
                      💾 Download File
                    </button>
                  </div>
                </div>
                <div className="text-[11px] font-mono text-zinc-400 bg-black/40 p-2.5 rounded-lg border border-white/5">
                  <div>Output File: {exportResult.file_name}</div>
                  <div>Summary: {exportResult.summary}</div>
                </div>
              </div>
            )}

            {/* Diagnostic Report Cards */}
            {report && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* System Specs */}
                <div className="glass-panel p-4 rounded-xl border border-white/10 space-y-3">
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider flex items-center gap-2">
                    <span>💻</span> Hardware & OS
                  </h3>
                  <div className="space-y-1.5 text-xs font-mono text-zinc-300">
                    <div className="flex justify-between py-1 border-b border-white/5">
                      <span className="text-zinc-500">OS</span>
                      <span>{report.system_environment.os} ({report.system_environment.arch})</span>
                    </div>
                    <div className="flex justify-between py-1 border-b border-white/5">
                      <span className="text-zinc-500">CPU Cores</span>
                      <span>{report.system_environment.logical_cpu_cores} cores</span>
                    </div>
                    <div className="flex justify-between py-1 border-b border-white/5">
                      <span className="text-zinc-500">Workspace Loaded</span>
                      <span>{report.workspace_status.has_workspace_loaded ? "Yes" : "No"}</span>
                    </div>
                    <div className="flex justify-between py-1">
                      <span className="text-zinc-500">Total Indexed Files</span>
                      <span>{report.workspace_status.total_indexed_files} files</span>
                    </div>
                  </div>
                </div>

                {/* Privacy & Keyring Status */}
                <div className="glass-panel p-4 rounded-xl border border-white/10 space-y-3">
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider flex items-center gap-2">
                    <span>🔑</span> Configured Providers
                  </h3>
                  <div className="space-y-1.5 text-xs font-mono text-zinc-300">
                    <div className="flex justify-between py-1 border-b border-white/5">
                      <span className="text-zinc-500">Cloud Providers</span>
                      <span>{report.ai_engine_status.configured_cloud_providers.length} configured</span>
                    </div>
                    <div className="flex justify-between py-1 border-b border-white/5">
                      <span className="text-zinc-500">Active Fallback Chain</span>
                      <span>{report.ai_engine_status.fallback_strategy}</span>
                    </div>
                    <div className="flex justify-between py-1">
                      <span className="text-zinc-500">Local Models Detected</span>
                      <span>{report.ai_engine_status.local_models_count} model(s)</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
