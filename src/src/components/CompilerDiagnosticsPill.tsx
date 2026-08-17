import React, { useEffect, useState } from "react";
import { compilerDiagnostics } from "../lib/api";
import { sounds } from "../lib/sound";
import { useTranslation } from "../i18n";
import type { DiagnosticItem } from "../types";

interface CompilerDiagnosticsPillProps {
  workspaceRoot?: string;
  onQuickFix?: (directive: string) => void;
}

export const CompilerDiagnosticsPill: React.FC<CompilerDiagnosticsPillProps> = ({
  workspaceRoot,
  onQuickFix,
}) => {
  const [items, setItems] = useState<DiagnosticItem[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [isScanning, setIsScanning] = useState(false);
  const { t } = useTranslation();

  const fetchDiagnostics = async () => {
    try {
      if (workspaceRoot) {
        const res = await compilerDiagnostics.runProbe(workspaceRoot);
        setItems(res);
      } else {
        const res = await compilerDiagnostics.getActiveFeed();
        setItems(res);
      }
    } catch {
      // Background probe failure is non-fatal
    }
  };

  const handleManualScan = async (e: React.MouseEvent) => {
    e.stopPropagation();
    sounds.playClick();
    setIsScanning(true);
    try {
      if (workspaceRoot) {
        const res = await compilerDiagnostics.runProbe(workspaceRoot);
        setItems(res);
      }
    } finally {
      setIsScanning(false);
    }
  };

  useEffect(() => {
    fetchDiagnostics();
    const timer = setInterval(fetchDiagnostics, 15000);
    return () => clearInterval(timer);
  }, [workspaceRoot]);

  const errorCount = items.filter((i) => i.severity === "Error").length;
  const warningCount = items.filter((i) => i.severity === "Warning").length;

  const handleQuickFixClick = (item: DiagnosticItem) => {
    sounds.playClick();
    const directive = `/fix @file:${item.file_path}:${item.line}`;
    if (onQuickFix) {
      onQuickFix(directive);
    }
    setIsOpen(false);
  };

  return (
    <div className="relative">
      {/* Pill Badge */}
      <button
        onClick={() => {
          sounds.playClick();
          setIsOpen(!isOpen);
        }}
        className={`px-2.5 py-1 rounded-full text-xs font-mono font-medium flex items-center gap-1.5 transition-all border shadow-sm ${
          errorCount > 0
            ? "bg-rose-500/20 text-rose-300 border-rose-500/40 hover:bg-rose-500/30"
            : warningCount > 0
            ? "bg-amber-500/20 text-amber-300 border-amber-500/40 hover:bg-amber-500/30"
            : "bg-emerald-500/15 text-emerald-300 border-emerald-500/30 hover:bg-emerald-500/20"
        }`}
        title={t("diagnostics_pill.title")}
      >
        <span
          className={`w-1.5 h-1.5 rounded-full ${
            errorCount > 0
              ? "bg-rose-500 animate-pulse"
              : warningCount > 0
              ? "bg-amber-400"
              : "bg-emerald-400"
          }`}
        />
        {errorCount > 0 ? (
          <span>
            {warningCount > 0
              ? t("diagnostics_pill.errors_and_warnings", { errors: errorCount, warnings: warningCount })
              : t("diagnostics_pill.errors_only", { errors: errorCount })}
          </span>
        ) : warningCount > 0 ? (
          <span>{t("diagnostics_pill.warnings_only", { warnings: warningCount })}</span>
        ) : (
          <span>{t("diagnostics_pill.clean_build")}</span>
        )}

        {isScanning && <span className="animate-spin text-[10px]">⏳</span>}
      </button>

      {/* Diagnostics Drawer Flyout */}
      {isOpen && (
        <div className="absolute bottom-full start-0 mb-2 w-96 max-h-80 overflow-y-auto bg-[#0C101A] border border-white/10 rounded-xl shadow-2xl p-3 z-50 text-xs font-sans space-y-2 animate-scale-up">
          <div className="flex items-center justify-between border-b border-white/5 pb-2 text-[11px]">
            <span className="font-bold text-zinc-200 flex items-center gap-1.5">
              <span>🩺 {t("diagnostics_pill.title")}</span>
              <span className="text-zinc-500 font-mono text-[10px]" dir="ltr">({items.length})</span>
            </span>
            <div className="flex items-center gap-1.5">
              <button
                onClick={handleManualScan}
                disabled={isScanning}
                className="text-[10px] px-2 py-0.5 rounded bg-white/5 hover:bg-white/10 text-teal-300 font-mono transition-colors"
              >
                {isScanning ? t("diagnostics_pill.scanning") : t("diagnostics_pill.re_scan")}
              </button>
              <button
                onClick={() => setIsOpen(false)}
                className="text-zinc-500 hover:text-white p-0.5"
              >
                ✕
              </button>
            </div>
          </div>

          {items.length === 0 ? (
            <div className="py-6 text-center text-zinc-500 text-xs font-mono">
              ✓ {t("diagnostics_pill.clean_message")}
            </div>
          ) : (
            <div className="space-y-2 custom-scrollbar">
              {items.map((item, idx) => (
                <div
                  key={`${item.file_path}-${item.line}-${idx}`}
                  className="p-2.5 rounded-lg bg-black/40 border border-white/5 space-y-1 hover:border-white/10 transition-colors"
                >
                  <div className="flex items-center justify-between text-[10px] font-mono" dir="ltr">
                    <span
                      className={`px-1.5 py-0.2 rounded font-bold ${
                        item.severity === "Error"
                          ? "bg-rose-500/20 text-rose-300 border border-rose-500/30"
                          : "bg-amber-500/20 text-amber-300 border border-amber-500/30"
                      }`}
                    >
                      {item.source.toUpperCase()}{item.code ? `:${item.code}` : ""}
                    </span>

                    <span className="text-zinc-400 truncate max-w-[180px]">
                      {item.file_path}:{item.line}:{item.col}
                    </span>
                  </div>

                  <p className="text-xs text-zinc-300 font-mono leading-relaxed line-clamp-2" dir="ltr">
                    {item.message}
                  </p>

                  <div className="pt-1 flex justify-end">
                    <button
                      onClick={() => handleQuickFixClick(item)}
                      className="px-2 py-0.5 rounded bg-rose-600/30 hover:bg-rose-600/50 text-rose-200 border border-rose-500/40 text-[10px] font-mono font-semibold transition-all flex items-center gap-1"
                    >
                      <span>⚡ {t("diagnostics_pill.quick_fix")}</span>
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default CompilerDiagnosticsPill;
