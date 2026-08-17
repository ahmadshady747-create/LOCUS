import React, { useRef, useEffect } from "react";
import type { OmniIntent } from "../types";
import { useTranslation } from "../i18n";

interface OmniSearchBarProps {
  query: string;
  onChange: (val: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  intent: OmniIntent | null;
  loading: boolean;
}

export const OmniSearchBar: React.FC<OmniSearchBarProps> = ({
  query,
  onChange,
  onKeyDown,
  intent,
  loading,
}) => {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Focus immediately on mount / wake
    inputRef.current?.focus();
  }, []);

  // Determine badge label and color theme based on active intent
  const getIntentBadge = () => {
    if (!intent) return null;

    switch (intent.type) {
      case "WebSearch":
        return {
          label: t("spotlight.intent_web"),
          badgeClass: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
          icon: "🌐",
        };
      case "TerminalCommand":
        return {
          label: t("spotlight.intent_terminal"),
          badgeClass: "bg-sky-500/20 text-sky-300 border-sky-500/30",
          icon: "⚡",
        };
      case "ChatMemory":
        return {
          label: t("spotlight.intent_chat"),
          badgeClass: "bg-purple-500/20 text-purple-300 border-purple-500/30",
          icon: "🧠",
        };
      case "FormalVerify":
        return {
          label: t("spotlight.intent_verify"),
          badgeClass: "bg-amber-500/20 text-amber-300 border-amber-500/30",
          icon: "🛡️",
        };
      case "AgentAction":
        return {
          label: t("spotlight.intent_action"),
          badgeClass: "bg-rose-500/20 text-rose-300 border-rose-500/30",
          icon: "🤖",
        };
      case "LocalSearch":
      default:
        return {
          label: t("spotlight.intent_search"),
          badgeClass: "bg-neutral-800 text-neutral-300 border-neutral-700",
          icon: "🔍",
        };
    }
  };

  const badge = getIntentBadge();

  return (
    <div className="relative flex items-center gap-3 px-4 py-3.5 bg-neutral-900/90 border-b border-neutral-800/80 rounded-t-2xl">
      {/* Intent Badge / Search Icon */}
      {badge ? (
        <div
          className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-semibold rounded-lg border transition-all duration-150 ${badge.badgeClass}`}
        >
          <span>{badge.icon}</span>
          <span>{badge.label}</span>
        </div>
      ) : (
        <span className="text-neutral-400 text-lg">🔍</span>
      )}

      {/* Main OmniBar Input */}
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={t("spotlight.placeholder")}
        className="w-full bg-transparent text-neutral-100 placeholder-neutral-500 text-sm font-medium focus:outline-none tracking-wide"
        autoFocus
      />

      {/* Loading Indicator */}
      {loading && (
        <div className="flex items-center gap-1">
          <div className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-pulse" />
          <div className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-pulse delay-75" />
          <div className="w-1.5 h-1.5 rounded-full bg-violet-400 animate-pulse delay-150" />
        </div>
      )}
    </div>
  );
};
