import React from "react";
import type { AmbientTelemetry } from "../types";
import { useTranslation } from "../i18n";

interface HUDTelemetryBarProps {
  telemetry: AmbientTelemetry | null;
}

export const HUDTelemetryBar: React.FC<HUDTelemetryBarProps> = ({ telemetry }) => {
  const { t } = useTranslation();

  const ram = telemetry?.ram_usage_mb ?? 38.5;
  const latency = telemetry?.latency_ms ?? 1.8;
  const tokensSaved = telemetry?.tokens_saved_pct ?? 96;
  const cost = telemetry?.estimated_cost_saved_usd ?? 0.0;

  return (
    <div className="flex items-center justify-between px-4 py-2 bg-neutral-950/95 border-t border-neutral-800/80 rounded-b-2xl text-[11px] text-neutral-400 font-mono select-none">
      {/* Live System Metrics */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1 text-emerald-400">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
          <span>{t("spotlight.ram_usage", { mb: ram.toFixed(1) })}</span>
        </div>

        <div className="flex items-center gap-1 text-sky-400">
          <span>⚡</span>
          <span>{t("spotlight.latency", { ms: latency.toFixed(1) })}</span>
        </div>

        <div className="flex items-center gap-1 text-purple-400">
          <span>🛡️</span>
          <span>{t("spotlight.tokens_saved", { pct: tokensSaved.toFixed(0) })}</span>
        </div>

        <div className="flex items-center gap-1 text-amber-400">
          <span>💰</span>
          <span>{t("spotlight.cloud_cost", { cost: cost.toFixed(2) })}</span>
        </div>
      </div>

      {/* Keyboard Shortcuts Cues */}
      <div className="hidden sm:flex items-center gap-2.5 text-neutral-500">
        <span className="bg-neutral-800/80 px-1.5 py-0.5 rounded text-[10px]">
          {t("spotlight.shortcut_nav")}
        </span>
        <span className="bg-neutral-800/80 px-1.5 py-0.5 rounded text-[10px]">
          {t("spotlight.shortcut_select")}
        </span>
        <span className="bg-neutral-800/80 px-1.5 py-0.5 rounded text-[10px]">
          {t("spotlight.shortcut_dismiss")}
        </span>
      </div>
    </div>
  );
};
