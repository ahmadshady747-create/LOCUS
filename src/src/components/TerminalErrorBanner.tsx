import React from "react";
import { sounds } from "../lib/sound";
import { useTranslation } from "../i18n";
import type { TerminalFailureReport } from "../types";

interface TerminalErrorBannerProps {
  report: TerminalFailureReport | null;
  onDismiss: () => void;
  onAutoFix: (directive: string) => void;
}

export const TerminalErrorBanner: React.FC<TerminalErrorBannerProps> = ({
  report,
  onDismiss,
  onAutoFix,
}) => {
  const { t } = useTranslation();
  if (!report) return null;

  const primary = report.primary_error;

  const handleFixClick = () => {
    sounds.playClick();
    let directive = "/fix";
    if (primary && primary.file_path) {
      directive = `/fix @file:${primary.file_path}${primary.line ? `:${primary.line}` : ""}`;
    }
    onAutoFix(directive);
  };

  return (
    <div className="w-full bg-[#180A0D] border border-rose-500/40 rounded-xl p-3.5 shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-3 text-zinc-100 animate-slide-down">
      <div className="space-y-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="w-2 h-2 rounded-full bg-rose-500 animate-ping" />
          <span className="text-xs font-bold text-rose-300 flex items-center gap-1.5">
            <span>🚨 {t("terminal_banner.command_failed")}</span>
            <code className="text-[11px] font-mono px-1.5 py-0.5 rounded bg-rose-500/20 text-rose-200 border border-rose-500/30" dir="ltr">
              {report.command} (exit {report.exit_code})
            </code>
          </span>

          {primary && (
            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-white/10 text-amber-300 border border-amber-500/30 flex items-center gap-1" dir="ltr">
              <span>📍</span> {primary.file_path}
              {primary.line && `:${primary.line}`}
            </span>
          )}
        </div>

        <p className="text-xs text-zinc-400 font-mono line-clamp-2" dir="ltr">
          {primary ? primary.message : report.clean_stderr_snippet.slice(0, 140)}
        </p>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        <button
          onClick={handleFixClick}
          className="px-3.5 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition-all shadow-md flex items-center gap-1.5"
        >
          <span>⚡</span> {t("terminal_banner.auto_fix_btn")}
        </button>
        <button
          onClick={() => {
            sounds.playClick();
            onDismiss();
          }}
          className="p-1.5 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 text-xs transition-colors"
          title={t("terminal_banner.dismiss")}
        >
          ✕
        </button>
      </div>
    </div>
  );
};

export default TerminalErrorBanner;

