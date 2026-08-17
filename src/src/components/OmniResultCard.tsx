import React from "react";
import type { OmniSearchResult, QuickVerifyReport, AmbientActionResult } from "../types";
import { useTranslation } from "../i18n";

interface OmniResultCardProps {
  result?: OmniSearchResult;
  formalReport?: QuickVerifyReport | null;
  agentResult?: AmbientActionResult | null;
  selected: boolean;
  onSelect: () => void;
  onInject: (text: string) => void;
}

export const OmniResultCard: React.FC<OmniResultCardProps> = ({
  result,
  formalReport,
  agentResult,
  selected,
  onSelect,
  onInject,
}) => {
  const { t } = useTranslation();

  // 1. Formal Verification Card Render
  if (formalReport) {
    return (
      <div
        onClick={onSelect}
        className={`p-3.5 rounded-xl border transition-all cursor-pointer ${
          selected
            ? "bg-amber-500/10 border-amber-500/40 shadow-lg shadow-amber-500/5"
            : "bg-neutral-900/50 border-neutral-800/80 hover:bg-neutral-800/40"
        }`}
      >
        <div className="flex items-center justify-between gap-2 mb-1.5">
          <div className="flex items-center gap-2">
            <span className="text-base">{formalReport.is_safe ? "🛡️" : "🚨"}</span>
            <span className="text-sm font-semibold text-neutral-200">
              {formalReport.target_function}
            </span>
          </div>

          <span
            className={`text-xs px-2 py-0.5 rounded font-mono font-medium ${
              formalReport.is_safe
                ? "bg-emerald-500/20 text-emerald-300"
                : "bg-rose-500/20 text-rose-300"
            }`}
          >
            {formalReport.is_safe
              ? t("spotlight.verified_safe")
              : t("spotlight.violation_found")}
          </span>
        </div>

        {formalReport.counterexample ? (
          <div className="p-2.5 mt-2 bg-rose-950/40 border border-rose-800/40 rounded-lg text-xs font-mono text-rose-300">
            {formalReport.counterexample}
          </div>
        ) : (
          <div className="text-xs text-neutral-400 font-mono">
            Forward Safety: {formalReport.forward_safety_score}% · Backward Intent:{" "}
            {formalReport.backward_intent_score}% · ({formalReport.execution_time_ms.toFixed(2)}ms)
          </div>
        )}
      </div>
    );
  }

  // 2. Ambient Agent Patch Card Render
  if (agentResult) {
    return (
      <div
        onClick={onSelect}
        className={`p-3.5 rounded-xl border transition-all cursor-pointer ${
          selected
            ? "bg-rose-500/10 border-rose-500/40 shadow-lg shadow-rose-500/5"
            : "bg-neutral-900/50 border-neutral-800/80 hover:bg-neutral-800/40"
        }`}
      >
        <div className="flex items-center justify-between gap-2 mb-2">
          <div className="flex items-center gap-2">
            <span className="text-base">🤖</span>
            <span className="text-sm font-semibold text-neutral-200">
              {agentResult.prompt}
            </span>
          </div>

          {agentResult.verification_passed && (
            <span className="text-xs px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 font-mono">
              🛡️ Safe (0 Panics)
            </span>
          )}
        </div>

        <p className="text-xs text-neutral-400 mb-2.5 leading-relaxed">
          {agentResult.explanation}
        </p>

        {agentResult.generated_patch && (
          <div className="relative group">
            <pre className="p-3 bg-neutral-950/80 border border-neutral-800 rounded-lg text-xs font-mono text-emerald-300 overflow-x-auto max-h-36">
              {agentResult.generated_patch}
            </pre>

            <button
              onClick={(e) => {
                e.stopPropagation();
                if (agentResult.generated_patch) {
                  onInject(agentResult.generated_patch);
                }
              }}
              className="mt-2.5 flex items-center gap-1.5 px-3 py-1.5 bg-violet-600 hover:bg-violet-500 text-white text-xs font-semibold rounded-lg shadow transition-all duration-150 active:scale-95"
            >
              <span>⚡</span>
              <span>{t("spotlight.inject_to_active")}</span>
            </button>
          </div>
        )}
      </div>
    );
  }

  // 3. Default Search Result Card (Files & Code Snippets)
  if (!result) return null;

  return (
    <div
      onClick={onSelect}
      className={`flex items-center justify-between p-3 rounded-xl border transition-all cursor-pointer ${
        selected
          ? "bg-violet-500/15 border-violet-500/40 shadow-md shadow-violet-500/5"
          : "bg-neutral-900/40 border-neutral-800/60 hover:bg-neutral-800/30"
      }`}
    >
      <div className="flex items-center gap-3 overflow-hidden">
        <span className="text-lg">
          {result.category === "File"
            ? "📄"
            : result.category === "Code"
            ? "💻"
            : result.category === "Terminal"
            ? "⚡"
            : "🔍"}
        </span>

        <div className="truncate">
          <div className="text-sm font-semibold text-neutral-200 truncate">
            {result.title}
          </div>
          <div className="text-xs text-neutral-400 font-mono truncate">
            {result.subtitle}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-[10px] uppercase tracking-wider px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 font-mono">
          {result.category}
        </span>
      </div>
    </div>
  );
};
