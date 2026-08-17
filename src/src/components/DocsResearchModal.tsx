import { useState } from "react";
import type { DocSearchResult, ResearchEcosystem, ResolvedErrorSolution } from "../types";
import { adrLedger, research } from "../lib/api";
import { sounds } from "../lib/sound";

interface DocsResearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialQuery?: string;
}

export default function DocsResearchModal({
  isOpen,
  onClose,
  initialQuery = "",
}: DocsResearchModalProps) {
  const [activeTab, setActiveTab] = useState<"docs" | "errors">("docs");
  const [query, setQuery] = useState(initialQuery);
  const [ecosystem, setEcosystem] = useState<ResearchEcosystem>("general");
  const [loading, setLoading] = useState(false);
  const [docResult, setDocResult] = useState<DocSearchResult | null>(null);
  const [errorSnippet, setErrorSnippet] = useState("");
  const [resolvedSolution, setResolvedSolution] = useState<ResolvedErrorSolution | null>(null);
  const [adrSuccess, setAdrSuccess] = useState(false);

  if (!isOpen) return null;

  const handleSearchDocs = async () => {
    if (!query.trim()) return;
    sounds.playClick();
    setLoading(true);
    setDocResult(null);

    try {
      const res = await research.fetchDocs(query.trim(), ecosystem);
      sounds.playSuccess();
      setDocResult(res);
    } catch (err: any) {
      alert(`Documentation lookup error: ${err?.toString() || "Unknown error"}`);
    } finally {
      setLoading(false);
    }
  };

  const handleResolveError = async () => {
    if (!errorSnippet.trim()) return;
    sounds.playClick();
    setAdrSuccess(false);

    try {
      const res = await research.resolveError(errorSnippet.trim());
      sounds.playSuccess();
      setResolvedSolution(res);
    } catch (err: any) {
      alert(`Error resolution failed: ${err?.toString() || "Unknown error"}`);
    }
  };

  const handleInjectToAdr = async () => {
    if (!resolvedSolution) return;
    sounds.playClick();
    try {
      await adrLedger.addNegative(".", {
        id: `neg-${Date.now()}`,
        pattern_name: resolvedSolution.error_title,
        severity: "warning",
        target_module: resolvedSolution.language,
        reason: resolvedSolution.explanation,
        forbidden_snippets: [resolvedSolution.negative_memory_pattern],
        recommended_alternative: resolvedSolution.recommended_fix_markdown,
        created_at: new Date().toISOString(),
      });
      sounds.playSuccess();
      setAdrSuccess(true);
    } catch (e) {
      console.error("Failed to inject to ADR", e);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md p-4 animate-fade-in">
      <div className="w-full max-w-3xl bg-[#090b14] border border-violet-500/30 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh] animate-spring-in">
        {/* Header */}
        <div className="p-4 bg-gradient-to-r from-violet-950/40 via-[#0d1220] to-black/60 border-b border-white/10 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-xl bg-violet-600/20 border border-violet-500/40 flex items-center justify-center text-base">
              🔬
            </div>
            <div>
              <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider">
                Semantic Research & Error Radar
              </h3>
              <p className="text-[11px] text-zinc-400 font-mono">
                Official registry doc extractors (crates.io, npm, PyPI) & compiler diagnostic resolver
              </p>
            </div>
          </div>

          <button
            onClick={() => {
              sounds.playClick();
              onClose();
            }}
            className="p-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-zinc-400 hover:text-white border border-white/10 text-xs font-mono"
          >
            ✕ Close
          </button>
        </div>

        {/* Tab Selector */}
        <div className="flex border-b border-white/10 bg-black/40 px-4 pt-2 gap-2 text-xs font-mono">
          <button
            onClick={() => {
              sounds.playClick();
              setActiveTab("docs");
            }}
            className={`pb-2 px-3 border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "docs"
                ? "border-violet-500 text-white font-bold"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>📚</span>
            <span>Package & Docs Explorer</span>
          </button>

          <button
            onClick={() => {
              sounds.playClick();
              setActiveTab("errors");
            }}
            className={`pb-2 px-3 border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "errors"
                ? "border-red-500 text-white font-bold"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>🚨</span>
            <span>Compiler Error Resolver</span>
          </button>
        </div>

        {/* Body Content */}
        <div className="p-4 overflow-y-auto flex-1 space-y-4">
          {activeTab === "docs" ? (
            <div className="space-y-4">
              {/* Search Bar */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    placeholder="Search package (e.g. tokio, zustand, fastapi, serde)..."
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleSearchDocs()}
                    className="flex-1 bg-black/60 border border-violet-500/30 rounded-xl px-3.5 py-2 text-xs font-mono text-white placeholder-zinc-500 focus:outline-none focus:border-violet-400"
                    autoFocus
                  />
                  <button
                    onClick={handleSearchDocs}
                    disabled={loading || !query.trim()}
                    className="btn-spring px-4 py-2 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 disabled:opacity-40 text-white rounded-xl text-xs font-mono font-bold shrink-0 shadow-glow-violet"
                  >
                    {loading ? "Searching..." : "🔍 Search"}
                  </button>
                </div>

                {/* Ecosystem Selector */}
                <div className="flex items-center gap-2 text-[10px] font-mono">
                  <span className="text-zinc-500">Ecosystem:</span>
                  {(
                    [
                      { id: "general", label: "🌐 Auto-Detect" },
                      { id: "rust", label: "🦀 Rust (crates.io)" },
                      { id: "typescript", label: "🔷 TypeScript (npm)" },
                      { id: "python", label: "🐍 Python (PyPI)" },
                    ] as const
                  ).map((eco) => (
                    <button
                      key={eco.id}
                      onClick={() => {
                        sounds.playClick();
                        setEcosystem(eco.id);
                      }}
                      className={`px-2.5 py-1 rounded-lg border transition-all ${
                        ecosystem === eco.id
                          ? "bg-violet-600/30 text-violet-200 border-violet-500/50 font-bold"
                          : "bg-black/40 text-zinc-400 border-white/5 hover:text-white"
                      }`}
                    >
                      {eco.label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Doc Search Result View */}
              {docResult && (
                <div className="p-4 rounded-xl bg-black/50 border border-white/10 space-y-3 animate-spring-in">
                  <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 pb-2.5">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-bold text-white font-mono">
                          {docResult.package.name}
                        </span>
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-violet-500/20 text-violet-300 border border-violet-500/40">
                          v{docResult.package.version}
                        </span>
                        <span className="text-[10px] font-mono text-zinc-400">
                          ({docResult.package.ecosystem})
                        </span>
                      </div>
                      <p className="text-xs text-zinc-300 mt-1">{docResult.package.description}</p>
                    </div>

                    <div className="flex items-center gap-2 text-xs font-mono">
                      {docResult.cached && (
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                          ⚡ Local Cache HIT
                        </span>
                      )}
                      {docResult.source_url && (
                        <a
                          href={docResult.source_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-violet-400 hover:text-violet-300 underline flex items-center gap-1"
                        >
                          <span>Official Docs ↗</span>
                        </a>
                      )}
                    </div>
                  </div>

                  {/* Signatures */}
                  {docResult.signatures.length > 0 && (
                    <div className="space-y-1.5">
                      <div className="text-[10px] font-mono uppercase text-zinc-400 font-bold">
                        Quick Import Signatures
                      </div>
                      <div className="space-y-1">
                        {docResult.signatures.map((sig, idx) => (
                          <pre
                            key={idx}
                            className="text-xs font-mono bg-black/80 border border-white/5 p-2 rounded-lg text-emerald-300 overflow-x-auto select-all"
                          >
                            <code>{sig}</code>
                          </pre>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Markdown Excerpt */}
                  <div className="space-y-1">
                    <div className="text-[10px] font-mono uppercase text-zinc-400 font-bold">
                      Extracted Summary
                    </div>
                    <div className="text-xs text-zinc-300 font-mono bg-black/40 border border-white/5 p-3 rounded-lg whitespace-pre-wrap max-h-60 overflow-y-auto leading-relaxed">
                      {docResult.summary_markdown}
                    </div>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              {/* Compiler Error Input */}
              <div className="space-y-2">
                <label className="text-xs font-mono text-zinc-300">
                  Paste Compiler Error or Stack Trace (Rust, TypeScript, Python):
                </label>
                <textarea
                  rows={4}
                  placeholder="e.g. error[E0382]: use of moved value: `buffer`&#10;or TS2345: Argument of type 'string' is not assignable to parameter of type 'number'..."
                  value={errorSnippet}
                  onChange={(e) => setErrorSnippet(e.target.value)}
                  className="w-full bg-black/60 border border-red-500/30 rounded-xl p-3 text-xs font-mono text-white placeholder-zinc-600 focus:outline-none focus:border-red-400"
                />
                <button
                  onClick={handleResolveError}
                  disabled={!errorSnippet.trim()}
                  className="btn-spring px-4 py-2 bg-gradient-to-r from-red-600 to-rose-600 hover:from-red-500 hover:to-rose-500 disabled:opacity-40 text-white rounded-xl text-xs font-mono font-bold shadow-md"
                >
                  🔬 Diagnose & Synthesize Fix
                </button>
              </div>

              {/* Resolved Solution Card */}
              {resolvedSolution && (
                <div className="p-4 rounded-xl bg-red-950/20 border border-red-500/40 space-y-3 animate-spring-in">
                  <div className="flex items-center justify-between border-b border-red-500/20 pb-2">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-mono font-bold px-2 py-0.5 rounded bg-red-500/20 text-red-300 border border-red-500/40">
                        {resolvedSolution.error_code}
                      </span>
                      <span className="text-sm font-bold text-white font-mono">
                        {resolvedSolution.error_title}
                      </span>
                      <span className="text-[10px] font-mono text-zinc-400">
                        ({resolvedSolution.language})
                      </span>
                    </div>

                    <button
                      onClick={handleInjectToAdr}
                      disabled={adrSuccess}
                      className="btn-spring px-3 py-1 bg-violet-600/30 hover:bg-violet-600/50 text-violet-200 border border-violet-500/40 rounded-lg text-xs font-mono transition-all"
                      title="Add this anti-pattern to ADR negative memory"
                    >
                      {adrSuccess ? "✓ Injected in ADR" : "💉 Inject into ADR Memory"}
                    </button>
                  </div>

                  <p className="text-xs text-zinc-300 leading-relaxed font-mono">
                    {resolvedSolution.explanation}
                  </p>

                  {/* Recommended Fix */}
                  <div className="space-y-1">
                    <div className="text-[10px] font-mono uppercase text-emerald-400 font-bold">
                      Recommended Fix Pattern
                    </div>
                    <pre className="text-xs font-mono bg-black/80 border border-emerald-500/30 p-3 rounded-lg text-emerald-300 overflow-x-auto whitespace-pre-wrap">
                      <code>{resolvedSolution.recommended_fix_markdown}</code>
                    </pre>
                  </div>

                  {/* Negative Memory Anti-pattern */}
                  <div className="p-2.5 rounded-lg bg-black/60 border border-amber-500/30 text-amber-300 text-xs font-mono flex items-start gap-2">
                    <span className="text-base leading-none">⚠️</span>
                    <div>
                      <div className="font-bold">Negative Memory Anti-Pattern:</div>
                      <div className="text-zinc-400 mt-0.5">
                        {resolvedSolution.negative_memory_pattern}
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
