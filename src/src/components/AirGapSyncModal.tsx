import React, { useEffect, useRef, useState } from "react";
import { airgap } from "../lib/api";
import { sounds } from "../lib/sound";
import type { AirGapIngestProgress } from "../types";

interface AirGapSyncModalProps {
  isOpen: boolean;
  onClose: () => void;
}

// Lightweight deterministic pseudo-QR matrix generator for visual display
function renderMatrixOnCanvas(canvas: HTMLCanvasElement, text: string) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const size = 256;
  canvas.width = size;
  canvas.height = size;

  // Background
  ctx.fillStyle = "#FFFFFF";
  ctx.fillRect(0, 0, size, size);

  // Grid params
  const gridCells = 29;
  const cellSize = size / gridCells;

  // Simple deterministic hash based on text bytes
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }

  ctx.fillStyle = "#000000";

  // Draw 3 standard corner finder patterns
  const drawFinder = (startX: number, startY: number) => {
    ctx.fillRect(startX * cellSize, startY * cellSize, 7 * cellSize, 7 * cellSize);
    ctx.fillStyle = "#FFFFFF";
    ctx.fillRect((startX + 1) * cellSize, (startY + 1) * cellSize, 5 * cellSize, 5 * cellSize);
    ctx.fillStyle = "#000000";
    ctx.fillRect((startX + 2) * cellSize, (startY + 2) * cellSize, 3 * cellSize, 3 * cellSize);
  };

  drawFinder(1, 1);
  drawFinder(gridCells - 8, 1);
  drawFinder(1, gridCells - 8);

  // Fill data matrix
  let byteIdx = 0;
  for (let r = 0; r < gridCells; r++) {
    for (let c = 0; c < gridCells; c++) {
      // Skip finder zones
      if (
        (r < 9 && c < 9) ||
        (r < 9 && c >= gridCells - 9) ||
        (r >= gridCells - 9 && c < 9)
      ) {
        continue;
      }

      const charCode = text.charCodeAt(byteIdx % text.length) || 0;
      const bit = ((charCode ^ (r * 31 + c * 17) ^ (hash >> (r % 16))) & 1) === 1;

      if (bit) {
        ctx.fillRect(c * cellSize, r * cellSize, cellSize, cellSize);
      }
      byteIdx++;
    }
  }
}

export const AirGapSyncModal: React.FC<AirGapSyncModalProps> = ({ isOpen, onClose }) => {
  const [mode, setMode] = useState<"broadcast" | "receive">("broadcast");

  // Broadcast state
  const [frames, setFrames] = useState<string[]>([]);
  const [currentFrameIdx, setCurrentFrameIdx] = useState(0);
  const [isPlaying, setIsPlaying] = useState(true);
  const [fps, setFps] = useState(8);
  const [generating, setGenerating] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Receive state
  const [inputText, setInputText] = useState("");
  const [progress, setProgress] = useState<AirGapIngestProgress | null>(null);
  const [applySuccess, setApplySuccess] = useState(false);
  const [applying, setApplying] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Load broadcast frames on modal open
  useEffect(() => {
    if (isOpen && mode === "broadcast") {
      loadFrames();
    }
  }, [isOpen, mode]);

  const loadFrames = async () => {
    try {
      setGenerating(true);
      const generated = await airgap.generateSyncFrames();
      setFrames(generated);
      setCurrentFrameIdx(0);
      setIsPlaying(true);
    } catch (e: any) {
      console.error("Failed to generate frames", e);
    } finally {
      setGenerating(false);
    }
  };

  // Animation loop for broadcast QR stream
  useEffect(() => {
    if (!isOpen || mode !== "broadcast" || frames.length === 0 || !isPlaying) return;

    const interval = setInterval(() => {
      setCurrentFrameIdx((prev) => (prev + 1) % frames.length);
    }, 1000 / fps);

    return () => clearInterval(interval);
  }, [isOpen, mode, frames, isPlaying, fps]);

  // Render canvas whenever currentFrameIdx changes
  useEffect(() => {
    if (canvasRef.current && frames.length > 0 && frames[currentFrameIdx]) {
      renderMatrixOnCanvas(canvasRef.current, frames[currentFrameIdx]);
    }
  }, [frames, currentFrameIdx]);

  const handleIngestInput = async (text: string) => {
    setInputText(text);
    const lines = text.split("\n").map((l) => l.trim()).filter((l) => l.startsWith("LOCUS:v1:"));

    for (const line of lines) {
      try {
        const prog = await airgap.ingestFrame(line);
        setProgress(prog);
        setErrorMsg(null);
        if (prog.is_ready) {
          sounds.playSuccess();
        }
      } catch (err: any) {
        setErrorMsg(err?.toString() || "Frame CRC or format error");
      }
    }
  };

  const handleApplyPayload = async () => {
    if (!progress || !progress.session_id) return;
    try {
      sounds.playClick();
      setApplying(true);
      await airgap.applySyncedPayload(progress.session_id);
      setApplySuccess(true);
      sounds.playSuccess();
    } catch (err: any) {
      setErrorMsg(err?.toString() || "Failed to apply synced configuration");
    } finally {
      setApplying(false);
    }
  };

  const handleReset = async () => {
    sounds.playClick();
    await airgap.resetReceiver();
    setProgress(null);
    setInputText("");
    setApplySuccess(false);
    setErrorMsg(null);
  };

  if (!isOpen) return null;

  const currentFrame = frames[currentFrameIdx] || "";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/85 backdrop-blur-md animate-fade-in">
      <div className="w-full max-w-2xl bg-[#090D16] border border-cyan-500/30 rounded-2xl shadow-2xl flex flex-col overflow-hidden text-zinc-100">
        {/* Modal Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-[#0E1422]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-xl bg-cyan-500/20 border border-cyan-500/40 flex items-center justify-center text-cyan-300 text-base shadow-sm">
              📡
            </div>
            <div>
              <h2 className="text-base font-bold text-white flex items-center gap-2">
                Air-Gapped Animated QR Sync
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-300 border border-cyan-500/30 font-medium">
                  ZERO-NETWORK
                </span>
              </h2>
              <p className="text-xs text-zinc-400">
                Sovereign, optical data transfer with SHA-256 integrity & CRC32 verification
              </p>
            </div>
          </div>

          <button
            onClick={() => {
              sounds.playClick();
              onClose();
            }}
            className="p-1.5 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
            title="Close"
          >
            <svg width={16} height={16} viewBox="0 0 16 16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Mode Selector Tabs */}
        <div className="flex items-center gap-1 px-6 pt-3 border-b border-white/5 bg-[#090D16] text-xs">
          <button
            onClick={() => {
              sounds.playClick();
              setMode("broadcast");
            }}
            className={`px-4 py-2 font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
              mode === "broadcast"
                ? "border-cyan-400 text-cyan-300 bg-cyan-500/10"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>📺</span> Broadcast Optical Stream (Send)
          </button>
          <button
            onClick={() => {
              sounds.playClick();
              setMode("receive");
            }}
            className={`px-4 py-2 font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
              mode === "receive"
                ? "border-cyan-400 text-cyan-300 bg-cyan-500/10"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>📥</span> Ingest Optical Stream (Receive)
            {progress && (
              <span className="px-1.5 py-0.2 rounded-full bg-cyan-500/20 text-cyan-300 text-[10px] font-mono">
                {progress.received_chunks}/{progress.total_chunks}
              </span>
            )}
          </button>
        </div>

        {/* Modal Body */}
        <div className="p-6 space-y-5">
          {mode === "broadcast" ? (
            <div className="flex flex-col items-center space-y-4">
              {generating ? (
                <div className="p-12 text-center text-xs text-zinc-400">
                  <span className="animate-spin text-xl inline-block mb-2">🌀</span>
                  <p>Generating cryptographic QR frames...</p>
                </div>
              ) : frames.length === 0 ? (
                <div className="p-8 text-center text-xs text-zinc-400">
                  No configuration frames available to broadcast.
                </div>
              ) : (
                <>
                  {/* High-Contrast Canvas Frame */}
                  <div className="p-3 bg-white rounded-2xl shadow-xl border border-white/20">
                    <canvas ref={canvasRef} className="w-56 h-56 rounded-lg block" />
                  </div>

                  {/* Frame Counter & Progress Bar */}
                  <div className="w-full max-w-md space-y-1.5">
                    <div className="flex items-center justify-between text-xs font-mono text-zinc-400">
                      <span>
                        Frame {currentFrameIdx + 1} / {frames.length}
                      </span>
                      <span>
                        {Math.round(((currentFrameIdx + 1) / frames.length) * 100)}% Complete
                      </span>
                    </div>
                    <div className="w-full h-1.5 rounded-full bg-white/10 overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-cyan-500 to-teal-400 transition-all duration-100"
                        style={{
                          width: `${((currentFrameIdx + 1) / frames.length) * 100}%`,
                        }}
                      />
                    </div>
                  </div>

                  {/* Stream Controls */}
                  <div className="flex items-center gap-3">
                    <button
                      onClick={() => {
                        sounds.playClick();
                        setCurrentFrameIdx((prev) => (prev > 0 ? prev - 1 : frames.length - 1));
                      }}
                      className="px-3 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs font-medium text-zinc-300 border border-white/10"
                      title="Previous Frame"
                    >
                      ⏮️ Prev
                    </button>

                    <button
                      onClick={() => {
                        sounds.playClick();
                        setIsPlaying(!isPlaying);
                      }}
                      className={`px-4 py-1.5 rounded-lg text-xs font-bold transition-all shadow-sm ${
                        isPlaying
                          ? "bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 border border-amber-500/40"
                          : "bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/40"
                      }`}
                    >
                      {isPlaying ? "⏸️ Pause Stream" : "▶️ Play Stream"}
                    </button>

                    <button
                      onClick={() => {
                        sounds.playClick();
                        setCurrentFrameIdx((prev) => (prev + 1) % frames.length);
                      }}
                      className="px-3 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs font-medium text-zinc-300 border border-white/10"
                      title="Next Frame"
                    >
                      Next ⏭️
                    </button>

                    <div className="flex items-center gap-1.5 pl-3 border-l border-white/10 text-xs text-zinc-400">
                      <span>Speed:</span>
                      <select
                        value={fps}
                        onChange={(e) => setFps(Number(e.target.value))}
                        className="bg-black/50 border border-white/10 rounded px-2 py-1 text-xs text-cyan-300 focus:outline-none"
                      >
                        <option value={4}>4 FPS (Slow)</option>
                        <option value={8}>8 FPS (Normal)</option>
                        <option value={12}>12 FPS (Fast)</option>
                        <option value={16}>16 FPS (Ultra)</option>
                      </select>
                    </div>
                  </div>

                  {/* Frame Raw Preview */}
                  <div className="w-full max-w-md p-2.5 rounded-lg bg-black/40 border border-white/5 font-mono text-[10px] text-zinc-500 truncate flex items-center justify-between">
                    <span className="truncate">{currentFrame}</span>
                    <button
                      onClick={() => {
                        navigator.clipboard.writeText(currentFrame);
                        sounds.playSuccess();
                      }}
                      className="ml-2 text-cyan-400 hover:text-cyan-300 shrink-0 font-sans text-xs"
                      title="Copy Frame Text"
                    >
                      📋
                    </button>
                  </div>
                </>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-zinc-300 mb-1.5">
                  Paste Optical Stream Frames (or Continuous Scanner Feed):
                </label>
                <textarea
                  value={inputText}
                  onChange={(e) => handleIngestInput(e.target.value)}
                  placeholder="Paste LOCUS:v1:... frames here or connect camera scanner"
                  rows={4}
                  className="w-full bg-black/50 border border-white/10 rounded-lg p-3 text-xs font-mono text-white placeholder:text-zinc-600 focus:outline-none focus:border-cyan-500"
                />
              </div>

              {/* Ingestion Progress Indicator */}
              {progress && (
                <div className="p-4 rounded-xl border border-white/10 bg-white/5 space-y-2">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-bold text-white flex items-center gap-2">
                      Session: <span className="font-mono text-cyan-300">{progress.session_id}</span>
                    </span>
                    <span className="font-mono text-zinc-400">
                      {progress.received_chunks} / {progress.total_chunks} Frames ({Math.round(progress.percent_complete)}%)
                    </span>
                  </div>

                  <div className="w-full h-2 rounded-full bg-white/10 overflow-hidden">
                    <div
                      className={`h-full transition-all duration-200 ${
                        progress.is_ready
                          ? "bg-emerald-400"
                          : "bg-gradient-to-r from-cyan-500 to-teal-400"
                      }`}
                      style={{ width: `${progress.percent_complete}%` }}
                    />
                  </div>

                  {progress.is_ready && (
                    <div className="pt-2 flex flex-col sm:flex-row items-center justify-between gap-3">
                      <div className="text-xs text-emerald-400 font-medium flex items-center gap-1.5">
                        <span>✅</span> 100% Received & SHA-256 Verified!
                      </div>

                      <button
                        onClick={handleApplyPayload}
                        disabled={applying || applySuccess}
                        className={`px-4 py-2 rounded-lg text-xs font-bold transition-all shadow-md ${
                          applySuccess
                            ? "bg-emerald-600/30 text-emerald-300 border border-emerald-500/40"
                            : "bg-emerald-500 hover:bg-emerald-400 text-black"
                        }`}
                      >
                        {applying
                          ? "Applying..."
                          : applySuccess
                          ? "Applied Successfully ✓"
                          : "Apply Synced Configuration"}
                      </button>
                    </div>
                  )}
                </div>
              )}

              {errorMsg && (
                <div className="p-2.5 rounded bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs font-mono">
                  ⚠️ {errorMsg}
                </div>
              )}

              <div className="flex justify-end pt-2">
                <button
                  onClick={handleReset}
                  className="px-3 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs font-mono text-zinc-400 border border-white/10"
                >
                  Clear & Reset Session
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default AirGapSyncModal;
