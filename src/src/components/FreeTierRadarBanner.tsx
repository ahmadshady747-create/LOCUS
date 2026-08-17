import { useEffect, useState } from "react";
import type { FreeProviderSuggestion } from "../types";
import { freeProviderRadar } from "../lib/api";
import { sounds } from "../lib/sound";

interface FreeTierRadarBannerProps {
  onKeyConfigured?: () => void;
}

export default function FreeTierRadarBanner({ onKeyConfigured }: FreeTierRadarBannerProps) {
  const [suggestions, setSuggestions] = useState<FreeProviderSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [inputKeys, setInputKeys] = useState<Record<string, string>>({});
  const [savingId, setSavingId] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const fetchSuggestions = async () => {
    setLoading(true);
    try {
      const list = await freeProviderRadar.getSuggestions();
      setSuggestions(list);
    } catch (e) {
      console.error("Failed to query free provider radar", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSuggestions();
  }, []);

  const handleDismiss = async (providerId: string) => {
    sounds.playClick();
    try {
      await freeProviderRadar.dismiss(providerId);
      setSuggestions((prev) => prev.filter((s) => s.provider.id !== providerId));
    } catch (e) {
      console.error("Failed to dismiss radar item", e);
    }
  };

  const handleSaveKey = async (providerId: string) => {
    const key = inputKeys[providerId];
    if (!key || !key.trim()) return;

    sounds.playClick();
    setSavingId(providerId);
    try {
      await freeProviderRadar.saveAndActivate(providerId, key.trim());
      sounds.playSuccess();
      setSuggestions((prev) => prev.filter((s) => s.provider.id !== providerId));
      if (onKeyConfigured) onKeyConfigured();
    } catch (err: any) {
      alert(`Failed to save key: ${err?.toString() || "Unknown error"}`);
    } finally {
      setSavingId(null);
    }
  };

  if (suggestions.length === 0) {
    return null;
  }

  return (
    <div className="p-4 rounded-2xl bg-gradient-to-r from-emerald-950/30 via-[#070e12] to-black/60 border border-emerald-500/30 space-y-3 shadow-xl animate-spring-in gpu-layer">
      {/* Banner Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-xl bg-emerald-600/20 border border-emerald-500/40 flex items-center justify-center text-base animate-neon-pulse">
            📡
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                Free Cloud Inference Radar
              </h4>
              <span className="text-[9px] font-mono px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 font-bold">
                {suggestions.length} Free Opportunities
              </span>
            </div>
            <p className="text-[11px] text-zinc-400 font-mono mt-0.5">
              Generous permanent free quotas available without credit card requirements.
            </p>
          </div>
        </div>

        <button
          onClick={fetchSuggestions}
          disabled={loading}
          className="p-1 rounded-md bg-white/5 hover:bg-white/10 text-zinc-400 hover:text-white border border-white/10 text-xs font-mono"
          title="Refresh free radar scan"
        >
          🔄 Refresh
        </button>
      </div>

      {/* Suggestion Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
        {suggestions.map(({ provider, potential_token_savings }) => {
          const isExpanded = expandedId === provider.id;
          const isSaving = savingId === provider.id;

          return (
            <div
              key={provider.id}
              className="p-3 rounded-xl bg-black/40 border border-white/10 hover:border-emerald-500/30 transition-all space-y-2.5 relative group"
            >
              {/* Dismiss Button */}
              <button
                onClick={() => handleDismiss(provider.id)}
                className="absolute top-2 right-2 text-zinc-500 hover:text-zinc-300 p-1 text-xs opacity-60 hover:opacity-100 transition-opacity"
                title="Dismiss suggestion"
              >
                ✕
              </button>

              {/* Card Header */}
              <div className="pr-6 space-y-1">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-bold text-white font-mono">{provider.name}</span>
                  <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                    {provider.badge}
                  </span>
                </div>
                <div className="flex items-center gap-2 text-[10px] font-mono text-zinc-400">
                  <span className="text-emerald-400">⚡ {provider.speed_tier}</span>
                  <span>·</span>
                  <span>{provider.free_tier_limits}</span>
                </div>
              </div>

              <p className="text-[11px] text-zinc-400 line-clamp-2">{provider.description}</p>

              {/* Token Savings Callout */}
              <div className="text-[10px] font-mono text-violet-300 bg-violet-950/30 px-2 py-1 rounded border border-violet-500/20 flex items-center gap-1.5">
                <span>💡</span>
                <span>{potential_token_savings}</span>
              </div>

              {/* Action Bar */}
              <div className="pt-1 space-y-2">
                {!isExpanded ? (
                  <div className="flex items-center justify-between gap-2">
                    <a
                      href={provider.key_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-xs font-mono text-emerald-400 hover:text-emerald-300 underline flex items-center gap-1"
                    >
                      <span>🔑 Get Free API Key</span>
                      <span className="text-[10px]">↗</span>
                    </a>

                    <button
                      onClick={() => setExpandedId(provider.id)}
                      className="btn-spring px-3 py-1 bg-emerald-600/20 hover:bg-emerald-600/30 text-emerald-300 border border-emerald-500/40 rounded-lg text-xs font-mono font-semibold transition-all"
                    >
                      + Paste Key
                    </button>
                  </div>
                ) : (
                  <div className="space-y-1.5 animate-spring-in">
                    <div className="flex items-center gap-1.5">
                      <input
                        type="password"
                        placeholder={`Paste ${provider.name} Key...`}
                        value={inputKeys[provider.id] ?? ""}
                        onChange={(e) =>
                          setInputKeys((prev) => ({ ...prev, [provider.id]: e.target.value }))
                        }
                        className="flex-1 bg-black/60 border border-emerald-500/40 rounded-lg px-2.5 py-1 text-xs font-mono text-white placeholder-zinc-600 focus:outline-none focus:border-emerald-400"
                        autoFocus
                      />
                      <button
                        onClick={() => handleSaveKey(provider.id)}
                        disabled={isSaving || !inputKeys[provider.id]?.trim()}
                        className="btn-spring px-3 py-1 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white rounded-lg text-xs font-mono font-bold shrink-0 transition-colors shadow-sm"
                      >
                        {isSaving ? "Saving..." : "Save"}
                      </button>
                      <button
                        onClick={() => setExpandedId(null)}
                        className="text-zinc-500 hover:text-zinc-300 text-xs px-1.5 py-1"
                      >
                        Cancel
                      </button>
                    </div>
                    <div className="text-[9px] font-mono text-zinc-500">
                      Keys stored locally in OS Keyring Vault with hardware encryption.
                    </div>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
