import React, { useEffect, useMemo, useState } from "react";
import * as Diff from "diff";
import { security, verifier } from "../lib/api";
import { sounds } from "../lib/sound";
import { useTranslation } from "../i18n";
import type { SecurityScanResult, VerificationVerdict } from "../types";

export interface DiffViewerProps {
  changeId?: string;
  filePath: string;
  originalContent: string;
  proposedContent: string;
  onAccept?: (changeId?: string) => Promise<void> | void;
  onReject?: (changeId?: string) => Promise<void> | void;
  onAcceptHunk?: (changeId: string, hunkId: string) => Promise<void> | void;
  onRejectHunk?: (changeId: string, hunkId: string) => Promise<void> | void;
  readOnly?: boolean;
}

interface HunkModel {
  hunkId: string;
  header: string;
  additions: number;
  deletions: number;
  lines: Array<{
    type: "added" | "removed" | "unchanged";
    content: string;
    origLineNum?: number;
    propLineNum?: number;
  }>;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  changeId,
  filePath,
  originalContent,
  proposedContent,
  onAccept,
  onReject,
  onAcceptHunk,
  onRejectHunk,
  readOnly = false,
}) => {
  const [viewMode, setViewMode] = useState<"inline" | "split">("inline");
  const [copied, setCopied] = useState(false);
  const [busyHunkId, setBusyHunkId] = useState<string | null>(null);
  const [busyAll, setBusyAll] = useState(false);
  const [secResult, setSecResult] = useState<SecurityScanResult | null>(null);
  const [selectedHunkIds, setSelectedHunkIds] = useState<Set<string>>(new Set());
  const [syntaxWarning, setSyntaxWarning] = useState<string | null>(null);
  const [verdict, setVerdict] = useState<VerificationVerdict | null>(null);
  const [showCounterexample, setShowCounterexample] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    const ext = filePath.split(".").pop();
    security
      .scanSnippet(proposedContent, ext)
      .then((res) => setSecResult(res))
      .catch(() => {});

    verifier
      .proveContract(filePath, proposedContent)
      .then((v) => setVerdict(v))
      .catch(() => {});
  }, [proposedContent, filePath]);

  // Compute overall diffs and discrete hunks
  const { hunks, stats } = useMemo(() => {
    const diff = Diff.diffLines(originalContent, proposedContent, {
      newlineIsToken: false,
      ignoreWhitespace: false,
    });

    let additions = 0;
    let deletions = 0;

    const formattedLines: Array<{
      type: "added" | "removed" | "unchanged";
      content: string;
      origLineNum?: number;
      propLineNum?: number;
    }> = [];

    let origCount = 1;
    let propCount = 1;

    for (const part of diff) {
      const lines = part.value.replace(/\n$/, "").split("\n");

      for (const line of lines) {
        if (part.added) {
          additions++;
          formattedLines.push({
            type: "added",
            content: line,
            propLineNum: propCount++,
          });
        } else if (part.removed) {
          deletions++;
          formattedLines.push({
            type: "removed",
            content: line,
            origLineNum: origCount++,
          });
        } else {
          formattedLines.push({
            type: "unchanged",
            content: line,
            origLineNum: origCount++,
            propLineNum: propCount++,
          });
        }
      }
    }

    // Group formatted lines into discrete hunks (cluster changes with 3 lines of context)
    const computedHunks: HunkModel[] = [];
    const changeIndices: number[] = [];

    formattedLines.forEach((l, idx) => {
      if (l.type !== "unchanged") changeIndices.push(idx);
    });

    if (changeIndices.length === 0) {
      return {
        hunks: [],
        stats: { additions, deletions, totalLines: formattedLines.length },
      };
    }

    const contextRadius = 3;
    const clusters: Array<{ start: number; end: number }> = [];
    let currentCluster: { start: number; end: number } | null = null;

    for (const idx of changeIndices) {
      const start = Math.max(0, idx - contextRadius);
      const end = Math.min(formattedLines.length, idx + contextRadius + 1);

      if (!currentCluster) {
        currentCluster = { start, end };
      } else if (start <= currentCluster.end) {
        currentCluster.end = Math.max(currentCluster.end, end);
      } else {
        clusters.push(currentCluster);
        currentCluster = { start, end };
      }
    }
    if (currentCluster) clusters.push(currentCluster);

    clusters.forEach((c, idx) => {
      const slice = formattedLines.slice(c.start, c.end);
      let hunkAdds = 0;
      let hunkDels = 0;
      let oldStart = 0;
      let newStart = 0;

      slice.forEach((l) => {
        if (l.type === "added") hunkAdds++;
        if (l.type === "removed") hunkDels++;
        if (!oldStart && l.origLineNum) oldStart = l.origLineNum;
        if (!newStart && l.propLineNum) newStart = l.propLineNum;
      });

      const header = `@@ -${oldStart || 1},${slice.filter((l) => l.type !== "added").length} +${newStart || 1},${slice.filter((l) => l.type !== "removed").length} @@`;

      computedHunks.push({
        hunkId: `hunk-${idx + 1}`,
        header,
        additions: hunkAdds,
        deletions: hunkDels,
        lines: slice,
      });
    });

    return {
      hunks: computedHunks,
      stats: { additions, deletions, totalLines: formattedLines.length },
    };
  }, [originalContent, proposedContent]);

  // Initialize selected hunks when hunks change
  useEffect(() => {
    if (hunks.length > 0 && selectedHunkIds.size === 0) {
      setSelectedHunkIds(new Set(hunks.map((h) => h.hunkId)));
    }
  }, [hunks]);

  const handleCopyProposed = () => {
    navigator.clipboard.writeText(proposedContent);
    sounds.playClick();
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const toggleHunkSelection = (hunkId: string) => {
    sounds.playClick();
    setSelectedHunkIds((prev) => {
      const next = new Set(prev);
      if (next.has(hunkId)) {
        next.delete(hunkId);
      } else {
        next.add(hunkId);
      }
      return next;
    });
  };

  const handleApplySelectedOnly = async () => {
    if (busyAll || !changeId) return;
    try {
      sounds.playSuccess();
      setBusyAll(true);
      setSyntaxWarning(null);

      // Convert HunkModel to DiffHunk for backend
      const backendHunks = hunks.map((h) => ({
        hunk_id: h.hunkId,
        old_start: 1,
        old_lines: h.lines.filter((l) => l.type !== "added").length,
        new_start: 1,
        new_lines: h.lines.filter((l) => l.type !== "removed").length,
        header: h.header,
        lines: h.lines.map((l) => ({
          line_type: l.type === "added" ? "Addition" : l.type === "removed" ? "Deletion" : "Context",
          content: l.content,
          old_line_no: l.origLineNum ?? null,
          new_line_no: l.propLineNum ?? null,
        })),
      }));

      const res = await (await import("../lib/api")).ergonomics.applySelectedHunks(
        filePath,
        originalContent,
        backendHunks as any,
        Array.from(selectedHunkIds)
      );

      if (res.syntax_warning) {
        setSyntaxWarning(res.syntax_warning);
      }

      if (onAccept) {
        await onAccept(changeId);
      }
    } catch (e: any) {
      setSyntaxWarning(e?.toString() || "Failed to apply selected hunks");
    } finally {
      setBusyAll(false);
    }
  };

  const handleAcceptAll = async () => {
    if (busyAll || !onAccept) return;
    sounds.playSuccess();
    setBusyAll(true);
    try {
      await onAccept(changeId);
    } finally {
      setBusyAll(false);
    }
  };

  const handleRejectAll = async () => {
    if (busyAll || !onReject) return;
    sounds.playClick();
    setBusyAll(true);
    try {
      await onReject(changeId);
    } finally {
      setBusyAll(false);
    }
  };

  const handleAcceptSingleHunk = async (hunkId: string) => {
    if (busyHunkId || !changeId) return;
    sounds.playSuccess();
    setBusyHunkId(hunkId);
    try {
      if (onAcceptHunk) {
        await onAcceptHunk(changeId, hunkId);
      } else if (onAccept) {
        await onAccept(changeId);
      }
    } finally {
      setBusyHunkId(null);
    }
  };

  const handleRejectSingleHunk = async (hunkId: string) => {
    if (busyHunkId || !changeId) return;
    sounds.playClick();
    setBusyHunkId(hunkId);
    try {
      if (onRejectHunk) {
        await onRejectHunk(changeId, hunkId);
      } else if (onReject) {
        await onReject(changeId);
      }
    } finally {
      setBusyHunkId(null);
    }
  };

  return (
    <div className="rounded-xl border border-violet-500/25 bg-[#0a0c14] overflow-hidden shadow-2xl transition-all">
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5 bg-[#0e111d] border-b border-white/5">
        <div className="flex items-center gap-2.5 min-w-0">
          <span className="text-sm font-mono font-bold text-white truncate flex items-center gap-1.5" dir="ltr">
            <span className="text-violet-400">📄</span> {filePath}
          </span>
          <div className="flex items-center gap-1 text-[11px] font-mono font-semibold" dir="ltr">
            <span className="px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
              +{stats.additions}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-300 border border-rose-500/30">
              -{stats.deletions}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-violet-500/15 text-violet-300 border border-violet-500/30">
              {t("diff.hunks_count", { count: hunks.length })}
            </span>
            {/* SAST Security Gate Badge */}
            {secResult && (
              <span
                className={`px-1.5 py-0.5 rounded font-mono text-[11px] font-semibold border flex items-center gap-1 ${
                  secResult.is_safe
                    ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                    : "bg-rose-500/20 text-rose-300 border-rose-500/40 animate-pulse"
                }`}
                title={secResult.summary}
              >
                <span>{secResult.is_safe ? `🛡️ ${t("diff.sast_safe")}` : `🚨 ${t("diff.sast_alerts", { count: secResult.violations.length })}`}</span>
                <span className="opacity-60 text-[9px]">({secResult.scan_duration_micros}µs)</span>
              </span>
            )}

            {/* Directive-Bound Bidirectional Formal Verifier Badge */}
            {verdict && (
              <button
                onClick={() => {
                  if (verdict.counterexample) {
                    sounds.playClick();
                    setShowCounterexample(!showCounterexample);
                  }
                }}
                className={`px-2 py-0.5 rounded font-mono text-[11px] font-semibold border flex items-center gap-1.5 transition-all ${
                  verdict.is_bidirectionally_verified
                    ? "bg-teal-500/15 text-teal-300 border-teal-500/30"
                    : "bg-rose-500/20 text-rose-300 border-rose-500/40 hover:bg-rose-500/30 cursor-pointer animate-pulse"
                }`}
                title="Bidirectional Formal Proof (Forward Symbolic + Backward wp Intent)"
              >
                <span>{verdict.is_bidirectionally_verified ? "🛡️" : "❌"}</span>
                <span>
                  {verdict.is_bidirectionally_verified
                    ? `${t("verifier.badge_verified")} (${verdict.forward_safety_proved ? t("verifier.forward_proved") : t("verifier.forward_failed")} | ${verdict.backward_intent_proved ? t("verifier.backward_proved") : t("verifier.backward_failed")})`
                    : t("verifier.badge_failed", { violation: verdict.violated_contract || "Violation" })}
                </span>
                <span className="opacity-60 text-[9px]" dir="ltr">({verdict.proof_time_ms}ms)</span>
                {verdict.counterexample && (
                  <span className="text-[9px] underline ms-1">
                    {showCounterexample ? "Hide Proof" : "Show Counterexample"}
                  </span>
                )}
              </button>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Mode Switcher */}
          <div className="flex items-center bg-black/40 rounded-lg p-0.5 border border-white/5 text-[11px]">
            <button
              onClick={() => {
                sounds.playClick();
                setViewMode("inline");
              }}
              className={`px-2 py-0.5 rounded transition-all font-medium ${
                viewMode === "inline"
                  ? "bg-violet-600 text-white shadow-sm font-semibold"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              {t("diff.unified_view")}
            </button>
            <button
              onClick={() => {
                sounds.playClick();
                setViewMode("split");
              }}
              className={`px-2 py-0.5 rounded transition-all font-medium ${
                viewMode === "split"
                  ? "bg-violet-600 text-white shadow-sm font-semibold"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              {t("diff.split_view")}
            </button>
          </div>

          <button
            onClick={handleCopyProposed}
            className="text-[11px] font-medium px-2.5 py-1 rounded-md bg-white/5 hover:bg-white/10 text-zinc-300 border border-white/10 transition-all"
            title="Copy modified code"
          >
            {copied ? `✓ ${t("common.copied")}` : t("diff.copy_new")}
          </button>

          {!readOnly && (
            <div className="flex items-center gap-1.5 ms-1">
              {hunks.length > 1 && (
                <button
                  onClick={handleApplySelectedOnly}
                  disabled={busyAll || selectedHunkIds.size === 0}
                  className="text-[11px] font-semibold px-2.5 py-1 rounded-lg bg-teal-600/30 hover:bg-teal-600/50 text-teal-200 border border-teal-500/40 flex items-center gap-1 transition-all disabled:opacity-50"
                  title="Apply only checked hunks"
                >
                  <span>⚡</span> {t("diff.apply_selected", { selected: selectedHunkIds.size, total: hunks.length })}
                </button>
              )}
              <button
                onClick={handleRejectAll}
                disabled={busyAll || Boolean(busyHunkId)}
                className="text-[11px] font-semibold px-3 py-1 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 text-rose-300 border border-rose-500/40 flex items-center gap-1 transition-all disabled:opacity-50"
                title="Reject all changes in this file"
              >
                <span>✕</span> {t("diff.reject_all")}
              </button>
              <button
                onClick={handleAcceptAll}
                disabled={busyAll || Boolean(busyHunkId)}
                className="text-[11px] font-semibold px-3 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white shadow-md flex items-center gap-1 transition-all disabled:opacity-50"
                title="Accept all changes in this file"
              >
                <span>✓</span> {t("diff.accept_all")}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* AST Syntax Safety Warning Banner */}
      {syntaxWarning && (
        <div className="px-4 py-2 bg-amber-500/15 border-b border-amber-500/30 text-amber-300 text-xs font-mono flex items-center justify-between">
          <span className="flex items-center gap-1.5">
            <span>⚠️</span> <strong>{t("diff.ast_warning", { warning: syntaxWarning })}</strong>
          </span>
          <button
            onClick={() => setSyntaxWarning(null)}
            className="text-[10px] text-amber-400 hover:text-white px-1.5 py-0.5 rounded bg-amber-500/20"
          >
            {t("common.dismiss")}
          </button>
        </div>
      )}

      {/* Formal Verifier Counterexample Drawer */}
      {showCounterexample && verdict?.counterexample && (
        <div className="px-4 py-3 bg-[#1A0A0D] border-b border-rose-500/30 text-xs font-mono space-y-2 animate-fade-in">
          <div className="flex items-center justify-between text-rose-300 font-bold">
            <span className="flex items-center gap-1.5">
              <span>🔬</span>
              <span>{t("verifier.counterexample_title")}: {verdict.violated_contract}</span>
            </span>
            <span className="text-[10px] text-zinc-500 font-normal" dir="ltr">
              {t("verifier.steps", { steps: verdict.steps_evaluated })}
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-2 bg-black/50 p-2.5 rounded-lg border border-rose-500/20 text-[11px]" dir="ltr">
            <div>
              <span className="text-zinc-500">{t("verifier.failing_variable")}</span>{" "}
              <code className="text-amber-300 font-bold">{verdict.counterexample.failing_var}</code>
            </div>
            <div>
              <span className="text-zinc-500">{t("verifier.failing_value")}</span>{" "}
              <code className="text-rose-400 font-bold">{verdict.counterexample.failing_val}</code>
            </div>
            <div className="truncate">
              <span className="text-zinc-500">{t("verifier.violation_expr")}</span>{" "}
              <code className="text-zinc-300">{verdict.counterexample.violation_expr}</code>
            </div>
          </div>

          <p className="text-zinc-400 text-[11px] font-sans italic" dir="ltr">
            {verdict.counterexample.trace_summary}
          </p>
        </div>
      )}

      {/* Diff Content View: Grouped by Hunks */}
      <div className="max-h-[540px] overflow-y-auto font-mono text-[11px] leading-relaxed custom-scrollbar p-3 space-y-4">
        {hunks.length === 0 ? (
          <div className="text-center py-6 text-zinc-500 font-sans text-xs">
            {t("diff.no_diffs")}
          </div>
        ) : (
          hunks.map((hunk, hIdx) => {
            const isHunkBusy = busyHunkId === hunk.hunkId;
            const isSelected = selectedHunkIds.has(hunk.hunkId);

            return (
              <div
                key={hunk.hunkId}
                className={`rounded-lg border overflow-hidden shadow-md transition-all ${
                  isSelected
                    ? "border-teal-500/40 bg-[#070910]"
                    : "border-white/5 bg-black/40 opacity-70"
                }`}
              >
                {/* Hunk Sub-Header with per-hunk actions */}
                <div className="flex items-center justify-between px-3 py-1.5 bg-[#121624] border-b border-white/5 text-[11px]">
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleHunkSelection(hunk.hunkId)}
                      className="rounded accent-teal-500 cursor-pointer"
                      title="Select this hunk for partial patch"
                    />
                    <span className="font-bold text-violet-300 font-sans">
                      {t("diff.hunk_title", { index: hIdx + 1 })}
                    </span>
                    <span className="text-zinc-500 text-[10px]" dir="ltr">
                      {hunk.header}
                    </span>
                    <div className="flex items-center gap-1 text-[10px]" dir="ltr">
                      <span className="text-emerald-400 font-semibold">
                        +{hunk.additions}
                      </span>
                      <span className="text-rose-400 font-semibold">
                        -{hunk.deletions}
                      </span>
                    </div>
                  </div>

                  {!readOnly && changeId && (
                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => handleRejectSingleHunk(hunk.hunkId)}
                        disabled={isHunkBusy || busyAll}
                        className="text-[10px] font-semibold px-2 py-0.5 rounded bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 border border-rose-500/30 transition-all disabled:opacity-50"
                        title="Discard this specific block"
                      >
                        ✕ {t("diff.reject_hunk")}
                      </button>
                      <button
                        onClick={() => handleAcceptSingleHunk(hunk.hunkId)}
                        disabled={isHunkBusy || busyAll}
                        className="text-[10px] font-semibold px-2.5 py-0.5 rounded bg-emerald-600/80 hover:bg-emerald-500 text-white transition-all shadow-sm disabled:opacity-50"
                        title="Apply only this block to disk"
                      >
                        {isHunkBusy ? "Applying…" : `✓ ${t("diff.accept_hunk")}`}
                      </button>
                    </div>
                  )}
                </div>

                {/* Hunk Code Lines */}
                {viewMode === "inline" ? (
                  <div className="divide-y divide-white/[0.02] diff-code-container font-mono">
                    {hunk.lines.map((line, idx) => {
                      const isAdded = line.type === "added";
                      const isRemoved = line.type === "removed";

                      const bgClass = isAdded
                        ? "bg-emerald-950/30 text-emerald-200 border-l-2 border-emerald-500"
                        : isRemoved
                        ? "bg-rose-950/30 text-rose-200 border-l-2 border-rose-500"
                        : "text-zinc-300 hover:bg-white/[0.02]";

                      const sign = isAdded ? "+" : isRemoved ? "-" : " ";

                      return (
                        <div
                          key={idx}
                          className={`flex items-stretch select-text transition-colors ${bgClass}`}
                        >
                          <div className="w-10 text-right pr-2 py-0.5 text-zinc-600 select-none border-r border-white/5 shrink-0">
                            {line.origLineNum ?? ""}
                          </div>
                          <div className="w-10 text-right pr-2 py-0.5 text-zinc-600 select-none border-r border-white/5 shrink-0">
                            {line.propLineNum ?? ""}
                          </div>
                          <div className="w-6 text-center py-0.5 font-bold select-none text-zinc-500 shrink-0">
                            {sign}
                          </div>
                          <div className="flex-1 py-0.5 pl-2 pr-4 overflow-x-auto whitespace-pre">
                            {line.content || " "}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  /* Split View for Hunk */
                  <div className="grid grid-cols-2 divide-x divide-white/10 bg-[#07090f] diff-code-container font-mono">
                    <div className="overflow-x-auto">
                      <div className="px-2 py-0.5 bg-black/40 text-[9px] font-bold text-rose-400 border-b border-white/5 uppercase">
                        Original
                      </div>
                      <div className="divide-y divide-white/[0.02]">
                        {hunk.lines
                          .filter((l) => l.type !== "added")
                          .map((line, idx) => (
                            <div
                              key={idx}
                              className={`flex items-stretch ${
                                line.type === "removed"
                                  ? "bg-rose-950/40 text-rose-200"
                                  : "text-zinc-400"
                              }`}
                            >
                              <div className="w-8 text-right pr-1.5 py-0.5 text-zinc-600 select-none border-r border-white/5 shrink-0">
                                {line.origLineNum}
                              </div>
                              <div className="flex-1 py-0.5 pl-2 pr-2 overflow-x-auto whitespace-pre">
                                {line.content || " "}
                              </div>
                            </div>
                          ))}
                      </div>
                    </div>

                    <div className="overflow-x-auto">
                      <div className="px-2 py-0.5 bg-black/40 text-[9px] font-bold text-emerald-400 border-b border-white/5 uppercase">
                        Proposed
                      </div>
                      <div className="divide-y divide-white/[0.02]">
                        {hunk.lines
                          .filter((l) => l.type !== "removed")
                          .map((line, idx) => (
                            <div
                              key={idx}
                              className={`flex items-stretch ${
                                line.type === "added"
                                  ? "bg-emerald-950/40 text-emerald-200"
                                  : "text-zinc-300"
                              }`}
                            >
                              <div className="w-8 text-right pr-1.5 py-0.5 text-zinc-600 select-none border-r border-white/5 shrink-0">
                                {line.propLineNum}
                              </div>
                              <div className="flex-1 py-0.5 pl-2 pr-2 overflow-x-auto whitespace-pre">
                                {line.content || " "}
                              </div>
                            </div>
                          ))}
                      </div>
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};

export default DiffViewer;
