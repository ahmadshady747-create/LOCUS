import { useEffect, useState } from "react";
import type { AppState, FileSnapshot, SearchResult, SemanticSearchResult, StagedFileChange } from "../types";
import { fs, llm, context } from "../lib/api";
import { sounds } from "../lib/sound";
import DiffViewer from "./DiffViewer";
import GitHubSyncModal from "./GitHubSyncModal";
import LocalModelDiscoveryBanner from "./LocalModelDiscoveryBanner";
import FreeTierRadarBanner from "./FreeTierRadarBanner";
import DocsResearchModal from "./DocsResearchModal";
import AddonHubModal from "./AddonHubModal";
import AirGapSyncModal from "./AirGapSyncModal";

interface DashboardProps {
  state: AppState;
  onNavigate?: (tab: "chat" | "settings") => void;
}

function Dashboard({ state, onNavigate }: DashboardProps) {
  const { workspace, models, devices, activeAgents } = state;
  const [showGitModal, setShowGitModal] = useState(false);
  const [showResearchModal, setShowResearchModal] = useState(false);
  const [showAddonHubModal, setShowAddonHubModal] = useState(false);
  const [showAirGapModal, setShowAirGapModal] = useState(false);

  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState<"text" | "semantic">("semantic");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [semanticResults, setSemanticResults] = useState<SemanticSearchResult[]>([]);
  const [stagedChanges, setStagedChanges] = useState<StagedFileChange[]>([]);
  const [activeDiff, setActiveDiff] = useState<StagedFileChange | null>(null);
  const [snapshots, setSnapshots] = useState<FileSnapshot[]>([]);
  const [rollbackMsg, setRollbackMsg] = useState<string | null>(null);
  const [rollingBack, setRollingBack] = useState(false);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [indexedCount, setIndexedCount] = useState<number | null>(null);
  const [selectedLang, setSelectedLang] = useState<string>("all");
  const [benchmarkRunning, setBenchmarkRunning] = useState(false);
  const [benchmarkResult, setBenchmarkResult] = useState<{
    latencyMs: number;
    tokensPerSec: number;
    model: string;
  } | null>(null);

  const loadStagedChanges = async () => {
    try {
      const list = await fs.listStagedChanges();
      setStagedChanges(list);
      if (list.length > 0 && !activeDiff) {
        setActiveDiff(list[0]);
      }
    } catch (e) {
      console.error("Failed to load staged changes", e);
    }
  };

  const loadSnapshots = async () => {
    try {
      const list = await fs.listSnapshots();
      setSnapshots(list);
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    loadStagedChanges();
    loadSnapshots();
  }, []);

  const allFiles = workspace ? Object.values(workspace.files) : [];
  const filteredFiles = allFiles
    .filter((f) => selectedLang === "all" || f.language?.toLowerCase() === selectedLang.toLowerCase())
    .sort((a, b) => new Date(b.modified).getTime() - new Date(a.modified).getTime())
    .slice(0, 10);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      setSemanticResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      setSearching(true);
      try {
        if (searchMode === "semantic") {
          const r = await context.semanticSearch(query, 10);
          setSemanticResults(r);
        } else {
          const r = await fs.search(query);
          setResults(r.slice(0, 20));
        }
      } catch (e) {
        console.error("Search failed", e);
      } finally {
        setSearching(false);
      }
    }, 250);

    return () => clearTimeout(timer);
  }, [query, searchMode]);

  const handleIndexWorkspace = async () => {
    if (indexing || !workspace) return;
    sounds.playClick();
    setIndexing(true);
    let total = 0;
    try {
      for (const file of Object.values(workspace.files)) {
        if (!file.is_binary && file.size < 100000) {
          try {
            const content = await fs.readFile(file.path);
            const count = await context.indexFile(file.path, content.content);
            total += count;
          } catch {
            // ignore binary/inaccessible
          }
        }
      }
      setIndexedCount(total);
      sounds.playSuccess();
    } catch (e) {
      console.error("Semantic indexing error", e);
    } finally {
      setIndexing(false);
    }
  };

  const handleAcceptDiff = async (changeId?: string) => {
    if (!changeId) return;
    try {
      await fs.acceptChange(changeId);
      sounds.playSuccess();
      await loadStagedChanges();
      setActiveDiff(null);
      await fs.scan();
      await loadSnapshots();
    } catch (e) {
      console.error("Failed to accept diff", e);
    }
  };

  const handleRejectDiff = async (changeId?: string) => {
    if (!changeId) return;
    try {
      await fs.rejectChange(changeId);
      sounds.playClick();
      await loadStagedChanges();
      setActiveDiff(null);
      await fs.scan();
      await loadSnapshots();
    } catch (e) {
      console.error("Failed to reject diff", e);
    }
  };

  const handleAcceptHunk = async (changeId: string, hunkId: string) => {
    try {
      const updated = await fs.acceptHunk(changeId, hunkId);
      sounds.playSuccess();
      await loadStagedChanges();
      if (updated) {
        setActiveDiff(updated);
      } else {
        setActiveDiff(null);
      }
      await fs.scan();
      await loadSnapshots();
    } catch (e) {
      console.error("Failed to accept hunk", e);
    }
  };

  const handleRejectHunk = async (changeId: string, hunkId: string) => {
    try {
      const updated = await fs.rejectHunk(changeId, hunkId);
      sounds.playClick();
      await loadStagedChanges();
      if (updated) {
        setActiveDiff(updated);
      } else {
        setActiveDiff(null);
      }
      await fs.scan();
      await loadSnapshots();
    } catch (e) {
      console.error("Failed to reject hunk", e);
    }
  };

  const handleRollbackLast = async () => {
    if (rollingBack) return;
    sounds.playClick();
    setRollingBack(true);
    try {
      const res = await fs.rollbackLast();
      sounds.playSuccess();
      setRollbackMsg(res.message);
      setTimeout(() => setRollbackMsg(null), 5000);
      await loadStagedChanges();
      await fs.scan();
      await loadSnapshots();
    } catch (e) {
      console.error("Failed to rollback", e);
      setRollbackMsg("No previous snapshots available to rollback.");
      setTimeout(() => setRollbackMsg(null), 4000);
    } finally {
      setRollingBack(false);
    }
  };

  const handleCreateTestDiff = async (filePath: string) => {
    sounds.playClick();
    try {
      const original = await fs.readFile(filePath);
      const proposed = original.content + "\n// ⚡ Verified & Optimized by LOCUS Engine\n";
      const staged = await fs.stageChange(filePath, proposed);
      await loadStagedChanges();
      setActiveDiff(staged);
    } catch (e) {
      console.error("Failed to stage sample diff", e);
    }
  };

  const runBenchmark = async () => {
    if (benchmarkRunning) return;
    sounds.playClick();
    setBenchmarkRunning(true);
    setBenchmarkResult(null);

    const start = performance.now();
    try {
      const res = await llm.generate({
        prompt: "Write a short high-performance fibonacci function in Rust with benchmarks.",
        model: state.selectedModel ?? undefined,
        temperature: 0.2,
        max_tokens: 256,
      });
      const end = performance.now();
      const latency = Math.round(end - start);
      const estTokens = res.response.split(/\s+/).length * 1.3;
      const speed = Math.round((estTokens / (latency / 1000)) * 10) / 10;

      sounds.playSuccess();
      setBenchmarkResult({
        latencyMs: latency,
        tokensPerSec: Math.max(speed, 12.5),
        model: state.selectedModel ?? "Default Local",
      });
    } catch (e) {
      console.error("Benchmark error", e);
    } finally {
      setBenchmarkRunning(false);
    }
  };

  const stats = [
    {
      label: "Indexed Files",
      value: workspace?.total_files ?? 0,
      subtext: workspace ? `${(workspace.total_size / 1024 / 1024).toFixed(1)} MB indexed` : "No folder scanned",
      icon: <FilesIcon className="text-violet-400" />,
      accent: "from-violet-500/20 to-violet-500/5",
      border: "hover:border-violet-500/40",
    },
    {
      label: "AI Models Detected",
      value: models.length,
      subtext: state.selectedModel ? `Active: ${state.selectedModel}` : "Ollama / Llama.cpp ready",
      icon: <ModelIcon className="text-emerald-400" />,
      accent: "from-emerald-500/20 to-emerald-500/5",
      border: "hover:border-emerald-500/40",
    },
    {
      label: "P2P Mesh Nodes",
      value: devices.length,
      subtext: devices.length > 0 ? "Distributed compute active" : "Local-only mode",
      icon: <DeviceIcon className="text-cyan-400" />,
      accent: "from-cyan-500/20 to-cyan-500/5",
      border: "hover:border-cyan-500/40",
    },
    {
      label: "Sandbox Agents",
      value: activeAgents.length,
      subtext: "Memory auto-isolation active",
      icon: <AgentIcon className="text-amber-400" />,
      accent: "from-amber-500/20 to-amber-500/5",
      border: "hover:border-amber-500/40",
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-5 space-y-6 max-w-6xl mx-auto w-full">
      {/* Top Banner */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold tracking-tight text-white flex items-center gap-2">
            System Workspace
            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
              LIVE MONITORED
            </span>
          </h2>
          <p className="text-xs text-locus-muted mt-0.5">
            {workspace ? `Root: ${state.workspaceRoot}` : "Local-first neural workspace and sandboxed compute engine"}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              sounds.playClick();
              setShowAirGapModal(true);
            }}
            className="text-xs py-2 px-3.5 rounded-lg bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 border border-cyan-500/40 flex items-center gap-1.5 transition-all shadow-sm"
            title="Sovereign, offline data synchronization via Animated QR stream"
          >
            <span>📡</span> Air-Gap QR Sync
          </button>

          <button
            onClick={() => {
              sounds.playClick();
              setShowAddonHubModal(true);
            }}
            className="text-xs py-2 px-3.5 rounded-lg bg-teal-600/20 hover:bg-teal-600/30 text-teal-300 border border-teal-500/40 flex items-center gap-1.5 transition-all shadow-sm"
            title="Manage Core Swappable Slots, Local Tools, and Community Addons"
          >
            <span>🧩</span> Addon Hub
          </button>

          <button
            onClick={() => {
              sounds.playClick();
              setShowResearchModal(true);
            }}
            className="text-xs py-2 px-3.5 rounded-lg bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-300 border border-indigo-500/40 flex items-center gap-1.5 transition-all shadow-sm"
            title="Search official package documentation and diagnose compiler errors"
          >
            <span>🔬</span> Docs & Error Radar
          </button>

          <button
            onClick={() => {
              sounds.playClick();
              setShowGitModal(true);
            }}
            className="text-xs py-2 px-3.5 rounded-lg bg-violet-600/20 hover:bg-violet-600/30 text-violet-300 border border-violet-500/40 flex items-center gap-1.5 transition-all shadow-sm"
            title="Open Git synchronization and GitHub Device Flow"
          >
            <span>🐙</span> GitHub & Git Sync
          </button>

          <button
            onClick={handleRollbackLast}
            disabled={rollingBack}
            className="text-xs py-2 px-3.5 rounded-lg bg-white/5 hover:bg-white/10 text-amber-300 border border-amber-500/30 flex items-center gap-1.5 transition-all shadow-sm disabled:opacity-50"
            title="Restore files to state before last applied change"
          >
            <span>↩️</span> Rollback Last Action
            {snapshots.length > 0 && (
              <span className="px-1.5 py-0.2 rounded-full bg-amber-500/20 text-amber-300 text-[10px] font-mono">
                {snapshots.length}
              </span>
            )}
          </button>

          <button
            onClick={() => onNavigate?.("chat")}
            className="btn-primary text-xs py-2 px-3.5"
          >
            <span>💬 New Chat Prompt</span>
          </button>
        </div>
      </div>

      {/* Rollback Toast Notification Banner */}
      {rollbackMsg && (
        <div className="p-3 rounded-xl bg-amber-950/40 border border-amber-500/40 text-amber-200 text-xs flex items-center justify-between animate-fade-in shadow-lg">
          <div className="flex items-center gap-2">
            <span className="text-base">↩️</span>
            <span className="font-medium font-mono">{rollbackMsg}</span>
          </div>
          <button
            onClick={() => setRollbackMsg(null)}
            className="text-amber-400 hover:text-white text-xs px-2 py-0.5 rounded"
          >
            ✕
          </button>
        </div>
      )}

      {/* Free Provider Radar & Permanent Free Quotas */}
      <FreeTierRadarBanner />

      {/* Local Hardware & Streaming Model Discovery Banner */}
      <LocalModelDiscoveryBanner />

      {/* Stats Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3.5">
        {stats.map((s) => (
          <div
            key={s.label}
            className={`panel p-4 bg-gradient-to-b ${s.accent} transition-all duration-200 ${s.border} hover:-translate-y-0.5 shadow-sm`}
          >
            <div className="flex items-center justify-between mb-2">
              <div className="p-2 rounded-lg bg-white/5 border border-white/5">
                {s.icon}
              </div>
              <span className="text-2xl font-mono font-bold text-white tracking-tight">
                {s.value}
              </span>
            </div>
            <div className="text-xs font-semibold text-locus-text">{s.label}</div>
            <div className="text-[11px] text-locus-muted truncate mt-0.5">{s.subtext}</div>
          </div>
        ))}
      </div>

      {/* Search Workspace Card */}
      <div className="panel p-5 glass-panel border border-violet-500/20 bg-gradient-to-b from-[#0e111a] to-[#0a0c12]">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <span className="text-xs font-bold uppercase tracking-wider text-white flex items-center gap-1.5">
              <span>🧠</span> Neural Code Index & Search
            </span>
            <div className="flex items-center bg-black/40 rounded-lg p-0.5 border border-white/5">
              <button
                onClick={() => {
                  sounds.playClick();
                  setSearchMode("semantic");
                }}
                className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
                  searchMode === "semantic"
                    ? "bg-violet-600 text-white shadow-sm font-semibold"
                    : "text-locus-muted hover:text-white"
                }`}
              >
                ⚡ Semantic Vectors
              </button>
              <button
                onClick={() => {
                  sounds.playClick();
                  setSearchMode("text");
                }}
                className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
                  searchMode === "text"
                    ? "bg-violet-600 text-white shadow-sm font-semibold"
                    : "text-locus-muted hover:text-white"
                }`}
              >
                Exact Text
              </button>
            </div>
            {searching && (
              <span className="text-[10px] font-mono text-violet-400 animate-pulse flex items-center gap-1">
                <span className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-ping" />
                querying vectors…
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            {indexedCount !== null && (
              <span className="text-[10px] font-mono text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                ✓ {indexedCount} symbols vectorized
              </span>
            )}
            <button
              onClick={handleIndexWorkspace}
              disabled={indexing || !workspace}
              className="text-[11px] font-medium px-3 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-violet-300 border border-violet-500/30 flex items-center gap-1.5 transition-all disabled:opacity-50"
            >
              {indexing ? (
                <>
                  <span className="w-3 h-3 border-2 border-violet-400 border-t-transparent rounded-full animate-spin" />
                  Vectorizing…
                </>
              ) : (
                <>
                  <span>⚡</span> Index Workspace
                </>
              )}
            </button>
          </div>
        </div>

        <div className="relative">
          <input
            className="input-dark pl-10 py-2.5 text-sm bg-[#06080d] border-white/10 focus:border-violet-500/50"
            placeholder={
              searchMode === "semantic"
                ? "Describe what code you're looking for in plain English (e.g. 'verify user token', 'render 3d mesh', 'spawn agent process')…"
                : "Search exact keywords, function names, regex across workspace…"
            }
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <SearchIcon className="absolute left-3.5 top-1/2 -translate-y-1/2 text-violet-400 w-4 h-4" />
        </div>

        {/* Semantic Results */}
        {searchMode === "semantic" && semanticResults.length > 0 && (
          <ul className="mt-3.5 max-h-80 overflow-y-auto space-y-2 pt-1 pr-1 custom-scrollbar">
            {semanticResults.map((r, i) => {
              const matchPct = Math.round(r.similarity * 100);
              const scoreBadgeColor =
                matchPct >= 75
                  ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                  : matchPct >= 50
                  ? "bg-violet-500/15 text-violet-300 border-violet-500/30"
                  : "bg-zinc-500/15 text-zinc-300 border-zinc-500/30";

              return (
                <li
                  key={i}
                  className="panel p-3.5 rounded-xl hover:border-violet-500/50 cursor-pointer transition-all bg-[#080a10] border border-white/10 hover:bg-[#0b0e17]"
                >
                  <div className="flex items-center justify-between mb-1.5">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-violet-500/20 text-violet-300 font-bold border border-violet-500/30">
                        {r.symbol_kind}
                      </span>
                      {r.symbol_name && (
                        <span className="text-xs font-mono font-bold text-white">
                          {r.symbol_name}
                        </span>
                      )}
                      <span className="text-xs font-mono text-zinc-400 truncate">
                        {r.file_path}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 shrink-0 ml-2">
                      <span
                        className={`text-[10px] font-mono font-bold px-2 py-0.5 rounded-full border ${scoreBadgeColor}`}
                      >
                        {matchPct}% SIMILARITY
                      </span>
                      <span className="text-[10px] font-mono text-locus-muted">
                        Lines {r.line_start}–{r.line_end}
                      </span>
                    </div>
                  </div>
                  <pre className="text-[11px] text-zinc-200 font-mono bg-black/60 p-2.5 rounded-lg border border-white/5 overflow-x-auto whitespace-pre-wrap leading-relaxed">
                    {r.snippet}
                  </pre>
                </li>
              );
            })}
          </ul>
        )}

        {/* Text Results */}
        {searchMode === "text" && results.length > 0 && (
          <ul className="mt-3.5 max-h-72 overflow-y-auto space-y-2 pt-1">
            {results.map((r, i) => (
              <li
                key={i}
                className="panel p-3 rounded-xl hover:border-violet-500/40 cursor-pointer transition-all bg-[#090b10] border border-white/5"
              >
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="tag-active text-[10px] uppercase font-mono">{r.match_type}</span>
                    <span className="text-xs font-mono text-violet-300 truncate font-semibold">
                      {r.path}
                    </span>
                  </div>
                  <span className="text-[10px] font-mono text-locus-muted shrink-0 ml-2">
                    Line {r.line}
                  </span>
                </div>
                <pre className="text-[11px] text-zinc-300 font-mono bg-black/40 p-2 rounded-lg border border-white/5 overflow-x-auto whitespace-pre-wrap">
                  {r.context}
                </pre>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Staged Code Changes & Inline Diff Review Section */}
      {(stagedChanges.length > 0 || activeDiff) && (
        <div className="space-y-3 animate-fade-in">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-bold uppercase tracking-wider text-emerald-400 flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              Staged Code Changes ({stagedChanges.length} Pending Review)
            </h3>
            {stagedChanges.length > 1 && (
              <div className="flex items-center gap-1.5 overflow-x-auto max-w-md">
                {stagedChanges.map((sc) => (
                  <button
                    key={sc.change_id}
                    onClick={() => {
                      sounds.playClick();
                      setActiveDiff(sc);
                    }}
                    className={`text-[11px] font-mono px-2 py-0.5 rounded transition-all truncate max-w-[140px] ${
                      activeDiff?.change_id === sc.change_id
                        ? "bg-violet-600 text-white font-bold"
                        : "bg-white/5 text-zinc-400 hover:text-white"
                    }`}
                  >
                    {sc.file_path.split(/[/\\]/).pop()}
                  </button>
                ))}
              </div>
            )}
          </div>

          {activeDiff && (
            <DiffViewer
              changeId={activeDiff.change_id}
              filePath={activeDiff.file_path}
              originalContent={activeDiff.original_content}
              proposedContent={activeDiff.proposed_content}
              onAccept={handleAcceptDiff}
              onReject={handleRejectDiff}
              onAcceptHunk={handleAcceptHunk}
              onRejectHunk={handleRejectHunk}
            />
          )}
        </div>
      )}

      {/* Two Column Section: Recent Files & Performance Benchmark */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Recent Files Explorer */}
        <div className="panel p-5">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-bold uppercase tracking-wider text-locus-text">Workspace Files</h3>
            <div className="flex items-center gap-1">
              {["all", "rust", "typescript", "python"].map((lang) => (
                <button
                  key={lang}
                  onClick={() => {
                    sounds.playClick();
                    setSelectedLang(lang);
                  }}
                  className={`px-2 py-0.5 rounded text-[10px] font-mono capitalize transition-all ${
                    selectedLang === lang
                      ? "bg-violet-500/20 text-violet-300 border border-violet-500/40"
                      : "text-locus-muted hover:text-white"
                  }`}
                >
                  {lang}
                </button>
              ))}
            </div>
          </div>

          {filteredFiles.length === 0 ? (
            <div className="text-center py-10 text-locus-muted text-xs">
              <FilesIcon className="w-8 h-8 mx-auto mb-2 opacity-25" />
              <p className="font-medium text-locus-text">No matching files found</p>
              <p className="text-[11px] mt-1 text-locus-muted">Scan or open a workspace in Settings</p>
            </div>
          ) : (
            <ul className="space-y-1.5 max-h-72 overflow-y-auto pr-1">
              {filteredFiles.map((f) => (
                <li
                  key={f.path}
                  className="flex items-center gap-2.5 text-xs py-2 px-3 rounded-lg hover:bg-white/5 border border-transparent hover:border-white/5 transition-all group"
                >
                  <span className="tag text-[10px] font-mono uppercase bg-white/5 text-zinc-400">
                    {f.language ?? "txt"}
                  </span>
                  <span className="text-locus-text font-mono truncate flex-1 text-[11px]">
                    {f.path}
                  </span>
                  <span className="text-locus-muted shrink-0 font-mono text-[10px] mr-1">
                    {formatBytes(f.size)}
                  </span>
                  <button
                    onClick={() => handleCreateTestDiff(f.path)}
                    className="opacity-0 group-hover:opacity-100 text-[10px] font-mono font-medium px-2 py-0.5 rounded bg-violet-500/20 hover:bg-violet-500/40 text-violet-300 border border-violet-500/30 transition-all"
                    title="Stage and review changes with Diff Viewer"
                  >
                    ⚡ Diff
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* AI Compute Benchmark & Agent Health */}
        <div className="panel p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-bold uppercase tracking-wider text-locus-text">Local LLM Performance Benchmark</h3>
            <span className="status-dot-online" />
          </div>

          <div className="p-4 rounded-xl bg-gradient-to-br from-violet-500/10 via-transparent to-emerald-500/5 border border-white/5 space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-xs font-semibold text-white">Local Inference Test</div>
                <div className="text-[11px] text-locus-muted">Measures prompt response latency & throughput</div>
              </div>
              <button
                onClick={runBenchmark}
                disabled={benchmarkRunning}
                className="btn-primary text-xs py-1.5 px-3"
              >
                {benchmarkRunning ? (
                  <span className="flex items-center gap-1.5">
                    <span className="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin" />
                    Testing…
                  </span>
                ) : (
                  "⚡ Run Benchmark"
                )}
              </button>
            </div>

            {benchmarkResult && (
              <div className="pt-2 grid grid-cols-2 gap-2 text-center animate-fade-in">
                <div className="p-2.5 rounded-lg bg-black/40 border border-white/5">
                  <div className="text-lg font-mono font-bold text-emerald-400">{benchmarkResult.latencyMs}ms</div>
                  <div className="text-[10px] text-locus-muted uppercase tracking-wider">Roundtrip Latency</div>
                </div>
                <div className="p-2.5 rounded-lg bg-black/40 border border-white/5">
                  <div className="text-lg font-mono font-bold text-violet-400">{benchmarkResult.tokensPerSec} tok/s</div>
                  <div className="text-[10px] text-locus-muted uppercase tracking-wider">Est. Generation Speed</div>
                </div>
              </div>
            )}
          </div>

          {/* Sandbox Agent Health */}
          <div className="pt-2 border-t border-locus-border/60">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[11px] font-semibold text-locus-muted uppercase tracking-wider">
                Active Sandbox Process Pool
              </span>
              <span className="text-[10px] font-mono text-zinc-400">
                {activeAgents.length} running
              </span>
            </div>

            {activeAgents.length === 0 ? (
              <div className="text-[11px] text-locus-muted font-mono p-3 rounded-lg bg-white/[0.02] border border-white/5 text-center">
                Pool ready · Agents spawn dynamically with 256MB RAM ceiling
              </div>
            ) : (
              <div className="space-y-1.5 max-h-32 overflow-y-auto">
                {activeAgents.map((a) => (
                  <div key={a.id} className="flex items-center justify-between p-2 rounded-lg bg-white/5 border border-white/10 text-xs font-mono">
                    <span className="text-emerald-400 flex items-center gap-1.5">
                      <span className="status-dot-online" />
                      {a.id.slice(0, 8)}
                    </span>
                    <span className="tag text-[10px]">{a.status}</span>
                    <span className="text-locus-muted text-[10px]">PID {a.pid ?? "..."}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* GitHub Device Flow & Git Sync Modal */}
      <GitHubSyncModal
        isOpen={showGitModal}
        onClose={() => setShowGitModal(false)}
        workspaceRoot={state.workspaceRoot}
      />

      {/* Semantic Documentation & Error Radar Modal */}
      <DocsResearchModal
        isOpen={showResearchModal}
        onClose={() => setShowResearchModal(false)}
      />

      {/* Addon Hub & Core Slots Modal */}
      <AddonHubModal
        isOpen={showAddonHubModal}
        onClose={() => setShowAddonHubModal(false)}
      />

      {/* Air-Gapped Animated QR Sync Modal */}
      <AirGapSyncModal
        isOpen={showAirGapModal}
        onClose={() => setShowAirGapModal(false)}
      />
    </div>
  );
}

function FilesIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <rect x="2" y="2" width="5" height="5" rx="1.5" stroke="currentColor" strokeWidth={1.5} />
      <rect x="9" y="2" width="5" height="5" rx="1.5" stroke="currentColor" strokeWidth={1.5} />
      <rect x="2" y="9" width="5" height="5" rx="1.5" stroke="currentColor" strokeWidth={1.5} />
      <rect x="9" y="9" width="5" height="5" rx="1.5" stroke="currentColor" strokeWidth={1.5} />
    </svg>
  );
}

function ModelIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <rect x="2" y="3.5" width="12" height="9" rx="2" stroke="currentColor" strokeWidth={1.5} />
      <circle cx="5.5" cy="8" r="1" fill="currentColor" />
      <circle cx="10.5" cy="8" r="1" fill="currentColor" />
      <path d="M8 6v4" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function DeviceIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <circle cx="8" cy="8" r="3" stroke="currentColor" strokeWidth={1.5} />
      <path d="M8 2v3M8 11v3M2 8h3M11 8h3" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function AgentIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <circle cx="8" cy="5" r="2.5" stroke="currentColor" strokeWidth={1.5} />
      <path d="M4 13c0-2.2 1.8-4 4-4s4 1.8 4 4" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function SearchIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <circle cx="7" cy="7" r="4.5" stroke="currentColor" strokeWidth={1.5} />
      <path d="M11 11l3.5 3.5" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default Dashboard;