import { useState, useEffect } from "react";
import type { AppState, EditorBridgeStatus, FallbackChainConfig } from "../types";
import { editorBridge, llm } from "../lib/api";
import { sounds } from "../lib/sound";
import { useTranslation } from "../i18n";
import CompilerDiagnosticsPill from "./CompilerDiagnosticsPill";

interface StatusBarProps {
  state: AppState;
  onNavigate?: (tab: "chat" | "dashboard" | "settings") => void;
}

function StatusBar({ state, onNavigate }: StatusBarProps) {
  const { privacyMode, models, devices, activeAgents, workspace } = state;
  const [soundOn, setSoundOn] = useState(sounds.isEnabled());
  const [fallbackConfig, setFallbackConfig] = useState<FallbackChainConfig | null>(null);
  const [bridgeStatus, setBridgeStatus] = useState<EditorBridgeStatus | null>(null);
  const { t, locale, setLocale } = useTranslation();

  useEffect(() => {
    setSoundOn(sounds.isEnabled());
    llm.getFallbackChain()
      .then((cfg) => setFallbackConfig(cfg))
      .catch(() => {});

    editorBridge.getStatus()
      .then((st) => setBridgeStatus(st))
      .catch(() => {});
  }, []);

  const toggleAudio = () => {
    const next = sounds.toggle();
    setSoundOn(next);
  };

  const toggleLanguage = () => {
    sounds.playClick();
    const nextLocale = locale === "en" ? "ar" : "en";
    setLocale(nextLocale);
  };

  const getStrategyLabel = (strategy?: string) => {
    switch (strategy) {
      case "LocalFirst":
        return "🔒 Local-First";
      case "SpeedFirst":
        return "⚡ Speed-First";
      case "CloudFirst":
        return "🌐 Cloud-First";
      default:
        return "🔄 Auto-Fallback";
    }
  };

  return (
    <footer className="h-8 px-4 border-t border-locus-border/80 glass-panel text-[11px] text-locus-muted shrink-0 select-none flex items-center justify-between font-mono">
      <div className="flex items-center gap-3">
        {/* Privacy Pill */}
        <button
          onClick={() => {
            sounds.playClick();
            onNavigate?.("settings");
          }}
          className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md font-medium text-[10px] transition-all ${
            privacyMode === "local"
              ? "bg-violet-500/10 text-violet-300 border border-violet-500/20 hover:border-violet-500/40"
              : "bg-emerald-500/10 text-emerald-300 border border-emerald-500/20 hover:border-emerald-500/40"
          }`}
          title="Click to configure compute mode"
        >
          <span className={privacyMode === "local" ? "status-dot bg-violet-400" : "status-dot-online"} />
          {privacyMode === "local"
            ? `🔒 ${t("status.local_privacy")}`
            : `⚡ ${t("status.mesh_peers", { count: devices.length })}`}
        </button>

        {/* Fallback Strategy Pill */}
        {fallbackConfig && (
          <button
            onClick={() => {
              sounds.playClick();
              onNavigate?.("settings");
            }}
            className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md font-medium text-[10px] bg-white/5 border border-white/10 text-zinc-300 hover:border-violet-500/40 hover:text-white transition-all"
            title="Click to change routing strategy in Settings"
          >
            <span>{getStrategyLabel(fallbackConfig.strategy)}</span>
          </button>
        )}

        {/* Model Indicator */}
        <div className="flex items-center gap-1.5 text-zinc-400">
          <span className="opacity-50">{t("status.model")}</span>
          {state.selectedModel ? (
            <span className="text-zinc-200 font-semibold px-1.5 py-0.2 bg-white/5 rounded border border-white/5">
              {state.selectedModel}
            </span>
          ) : (
            <span className="text-amber-400/80">
              {t("status.auto_model", { count: models.length })}
            </span>
          )}
        </div>

        {/* Token Economy Pill */}
        <div
          className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md font-medium text-[10px] bg-emerald-500/10 text-emerald-300 border border-emerald-500/20"
          title="Tokens saved via In-Memory AST Cache & Code Skeletonizer"
        >
          <span>🪙</span>
          <span>{t("status.economy_saved")}</span>
        </div>

        {/* Editor Bridge Pill */}
        {bridgeStatus && (
          <button
            onClick={async () => {
              sounds.playClick();
              if (bridgeStatus.connected_editor) {
                await editorBridge.openInEditor(state.workspaceRoot || ".");
              }
            }}
            className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md font-medium text-[10px] bg-cyan-500/10 text-cyan-300 border border-cyan-500/20 hover:border-cyan-500/40 transition-all"
            title={
              bridgeStatus.connected_editor
                ? `Connected to ${bridgeStatus.connected_editor.name}. Click to open workspace in editor.`
                : "No external IDE detected. Bridge active."
            }
          >
            <span>🔌</span>
            <span>
              {bridgeStatus.connected_editor
                ? t("status.editor_connected", { name: bridgeStatus.connected_editor.name })
                : t("status.editor_bridge_ready")}
            </span>
          </button>
        )}

        {/* Compiler Diagnostics Probe Pill */}
        <CompilerDiagnosticsPill
          workspaceRoot={state.workspaceRoot || undefined}
          onQuickFix={(dir) => {
            navigator.clipboard.writeText(dir);
            sounds.playSuccess();
          }}
        />

        {/* Active Agents */}
        {activeAgents.length > 0 && (
          <div className="flex items-center gap-1.5 text-amber-300 bg-amber-500/10 px-2 py-0.5 rounded-md border border-amber-500/20">
            <span className="status-dot-busy" />
            <span>{t("status.agents_executing", { count: activeAgents.length })}</span>
          </div>
        )}
      </div>

      <div className="flex items-center gap-4">
        {/* Language Switcher Pill */}
        <button
          onClick={toggleLanguage}
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-sans font-medium bg-white/5 hover:bg-white/10 text-zinc-300 hover:text-white border border-white/10 transition-colors"
          title={t("status.language_toggle")}
        >
          <span>🌐</span>
          <span>{locale === "en" ? "عربي" : "EN"}</span>
        </button>

        {/* Workspace Summary */}
        {workspace ? (
          <div className="flex items-center gap-1.5 text-zinc-400">
            <span className="text-white font-semibold">{workspace.total_files}</span>
            <span className="opacity-60">files</span>
            <span className="opacity-40">·</span>
            <span className="text-white font-semibold">{formatBytes(workspace.total_size)}</span>
          </div>
        ) : (
          <span className="opacity-50">{t("status.no_workspace")}</span>
        )}

        {/* Audio Toggle */}
        <button
          onClick={toggleAudio}
          className="opacity-70 hover:opacity-100 hover:text-white transition-opacity text-xs"
          title={soundOn ? t("status.audio_mute") : t("status.audio_enable")}
        >
          {soundOn ? "🔊" : "🔇"}
        </button>
      </div>
    </footer>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default StatusBar;