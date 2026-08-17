import { useEffect, useRef, useState } from "react";
import type { LocalDiscoveryReport, ModelPullProgress } from "../types";
import { localDiscovery, modelPuller } from "../lib/api";
import { sounds } from "../lib/sound";

interface LocalModelDiscoveryBannerProps {
  onModelInstalled?: () => void;
}

export default function LocalModelDiscoveryBanner({
  onModelInstalled,
}: LocalModelDiscoveryBannerProps) {
  const [report, setReport] = useState<LocalDiscoveryReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [pullProgress, setPullProgress] = useState<ModelPullProgress | null>(null);
  const pollTimerRef = useRef<number | null>(null);

  const fetchDiscoveryReport = async () => {
    setLoading(true);
    try {
      const rep = await localDiscovery.getReport();
      setReport(rep);
    } catch (e) {
      console.error("Failed to probe local hardware & inference endpoints", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDiscoveryReport();
  }, []);

  // Poll active pull progress
  useEffect(() => {
    if (!activeJobId) return;

    const poll = async () => {
      try {
        const prog = await modelPuller.getProgress(activeJobId);
        if (prog) {
          setPullProgress(prog);

          if (prog.is_done) {
            if (!prog.error) {
              sounds.playSuccess();
              if (onModelInstalled) onModelInstalled();
              fetchDiscoveryReport();
            }
            setActiveJobId(null);
          }
        }
      } catch (err) {
        console.error("Polling error", err);
      }
    };

    pollTimerRef.current = window.setInterval(poll, 600);
    return () => {
      if (pollTimerRef.current) clearInterval(pollTimerRef.current);
    };
  }, [activeJobId, onModelInstalled]);

  const handleStartPull = async (modelId: string) => {
    sounds.playClick();
    try {
      const jobId = await modelPuller.startPull(modelId);
      setActiveJobId(jobId);
    } catch (err: any) {
      alert(`Failed to start download: ${err?.toString() || "Unknown error"}`);
    }
  };

  const handleCancelPull = async () => {
    if (!activeJobId) return;
    sounds.playClick();
    try {
      await modelPuller.cancelPull(activeJobId);
      setActiveJobId(null);
      setPullProgress(null);
    } catch (err) {
      console.error("Cancel error", err);
    }
  };

  if (!report) {
    return (
      <div className="p-3 rounded-xl bg-black/30 border border-white/10 flex items-center justify-between text-xs font-mono animate-pulse">
        <span className="text-zinc-400">🔍 Probing local hardware & active inference ports...</span>
      </div>
    );
  }

  const { hardware, recommendation, endpoints } = report;
  const ollamaEndpoint = endpoints.find((e) => e.name === "Ollama");
  const isOllamaOnline = ollamaEndpoint?.is_reachable ?? false;

  return (
    <div className="p-4 rounded-2xl bg-gradient-to-r from-[#0d1220] via-[#090b14] to-black/60 border border-violet-500/30 space-y-3.5 shadow-xl animate-spring-in gpu-layer">
      {/* Header Info */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-xl bg-violet-600/20 border border-violet-500/30 flex items-center justify-center text-base">
            🖥️
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                Hardware Prober & Local Discovery
              </h4>
              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                {hardware.total_ram_gb} GB RAM · {hardware.cpu_cores} Cores
              </span>
            </div>
            <p className="text-[11px] text-zinc-400 font-mono mt-0.5">
              {hardware.os} ({hardware.arch}) {hardware.has_gpu ? `· 🎮 ${hardware.gpu_name ?? "GPU"} (${hardware.vram_gb ?? "?"} GB VRAM)` : "· CPU Inference"}
            </p>
          </div>
        </div>

        {/* Endpoints Status Badges */}
        <div className="flex items-center gap-1.5 text-[10px] font-mono">
          {endpoints.map((ep) => (
            <div
              key={ep.name}
              className={`px-2.5 py-1 rounded-lg border flex items-center gap-1.5 ${
                ep.is_reachable
                  ? "bg-emerald-950/40 text-emerald-300 border-emerald-500/30"
                  : "bg-black/40 text-zinc-500 border-white/5"
              }`}
              title={ep.url}
            >
              <span className={`w-1.5 h-1.5 rounded-full ${ep.is_reachable ? "bg-emerald-400 animate-pulse" : "bg-zinc-600"}`} />
              <span>{ep.name}</span>
              {ep.is_reachable && (
                <span className="text-[9px] opacity-75">({ep.models_count} models)</span>
              )}
            </div>
          ))}
          <button
            onClick={fetchDiscoveryReport}
            disabled={loading}
            className="p-1 rounded-md bg-white/5 hover:bg-white/10 text-zinc-400 hover:text-white border border-white/10"
            title="Rescan hardware and ports"
          >
            🔄
          </button>
        </div>
      </div>

      {/* Recommended Model Box */}
      <div className="p-3 rounded-xl bg-black/40 border border-white/10 flex flex-col md:flex-row items-start md:items-center justify-between gap-3">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="text-xs">⚡</span>
            <span className="text-xs font-bold text-white font-mono">
              Recommended Model: <span className="text-violet-300">{recommendation.display_name}</span>
            </span>
            <span className="text-[9px] font-mono px-2 py-0.5 rounded bg-violet-500/15 text-violet-300 border border-violet-500/30">
              {recommendation.tier} · {recommendation.download_size_gb} GB
            </span>
          </div>
          <p className="text-[11px] text-zinc-400 max-w-2xl">{recommendation.rationale}</p>
        </div>

        {/* Action Button / Status */}
        <div className="shrink-0">
          {recommendation.is_installed ? (
            <div className="px-3 py-1.5 rounded-lg bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 text-xs font-mono font-semibold flex items-center gap-1.5">
              <span>✓ Model Installed & Ready</span>
            </div>
          ) : activeJobId ? (
            <button
              onClick={handleCancelPull}
              className="btn-spring px-3 py-1.5 bg-red-600/30 hover:bg-red-600/50 text-red-200 border border-red-500/40 rounded-lg text-xs font-mono font-semibold transition-all"
            >
              Cancel Pull
            </button>
          ) : (
            <button
              onClick={() => handleStartPull(recommendation.model_id)}
              disabled={!isOllamaOnline}
              className={`btn-spring px-4 py-2 rounded-xl text-xs font-mono font-bold flex items-center gap-2 transition-all shadow-glow-violet ${
                isOllamaOnline
                  ? "bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white"
                  : "bg-zinc-800 text-zinc-500 cursor-not-allowed border border-white/10"
              }`}
              title={isOllamaOnline ? "Stream download model directly into Ollama" : "Ollama daemon is offline on port 11434"}
            >
              <span>📥 1-Click Install ({recommendation.parameter_size})</span>
            </button>
          )}
        </div>
      </div>

      {/* Streaming Download Progress Bar */}
      {pullProgress && (
        <div className="p-3 rounded-xl bg-violet-950/30 border border-violet-500/40 space-y-2 animate-spring-in">
          <div className="flex items-center justify-between text-xs font-mono">
            <div className="flex items-center gap-2 text-violet-200 font-semibold truncate">
              <span className="animate-spin text-violet-400">↻</span>
              <span>{pullProgress.status}</span>
            </div>
            <div className="flex items-center gap-3 text-zinc-400 shrink-0">
              {pullProgress.speed_mb_per_sec > 0 && (
                <span className="text-emerald-400 font-bold">{pullProgress.speed_mb_per_sec} MB/s</span>
              )}
              {pullProgress.eta_seconds !== null && pullProgress.eta_seconds !== undefined && (
                <span>ETA: ~{pullProgress.eta_seconds}s</span>
              )}
              <span className="text-white font-bold">{pullProgress.percentage}%</span>
            </div>
          </div>

          <div className="w-full bg-black/60 rounded-full h-2 overflow-hidden border border-white/10">
            <div
              className="bg-gradient-to-r from-violet-500 to-emerald-400 h-full transition-all duration-300 rounded-full"
              style={{ width: `${pullProgress.percentage}%` }}
            />
          </div>

          {pullProgress.error && (
            <div className="text-[11px] font-mono text-red-300 bg-red-950/50 border border-red-500/30 p-2 rounded-lg">
              ✗ {pullProgress.error}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
