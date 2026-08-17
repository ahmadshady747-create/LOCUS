import { useEffect, useState } from "react";
import type { AppState, DetectedKeyReport, LocalModel } from "../types";
import { llm } from "../lib/api";
import { sounds } from "../lib/sound";
import {
  AVAILABLE_FONTS,
  getTypographySettings,
  saveTypographySettings,
  type TypographySettings,
} from "../lib/theme";

interface OnboardingModalProps {
  isOpen: boolean;
  onClose: () => void;
  appState: AppState;
  onRefreshModels?: () => Promise<void>;
}

export default function OnboardingModal({
  isOpen,
  onClose,
  appState,
  onRefreshModels,
}: OnboardingModalProps) {
  const [step, setStep] = useState<1 | 2 | 3>(1);

  // Step 1: Key auto-detection state
  const [detectedReports, setDetectedReports] = useState<DetectedKeyReport[] | null>(null);
  const [scanningKeys, setScanningKeys] = useState(false);
  const [importSuccess, setImportSuccess] = useState(false);

  // Step 2: Local models state
  const [scanningModels, setScanningModels] = useState(false);
  const [detectedModels, setDetectedModels] = useState<LocalModel[]>(appState.models || []);

  // Step 3: Typography settings
  const [typography, setTypography] = useState<TypographySettings>(getTypographySettings());

  // Auto-scan keys on first mount if open
  useEffect(() => {
    if (isOpen && step === 1 && !detectedReports && !scanningKeys) {
      void handleScanKeys();
    }
  }, [isOpen, step]);

  // Sync models
  useEffect(() => {
    if (appState.models?.length) {
      setDetectedModels(appState.models);
    }
  }, [appState.models]);

  if (!isOpen) return null;

  const handleScanKeys = async () => {
    setScanningKeys(true);
    try {
      const reports = await llm.autoDetectKeys();
      setDetectedReports(reports);
      sounds.playReceive();
      if (reports.some((r) => r.imported)) {
        setImportSuccess(true);
      }
    } catch (e) {
      console.error("Failed to auto-detect API keys", e);
      setDetectedReports([]);
    } finally {
      setScanningKeys(false);
    }
  };

  const handleScanModels = async () => {
    setScanningModels(true);
    sounds.playClick();
    try {
      const models = await llm.detectModels();
      setDetectedModels(models);
      if (onRefreshModels) {
        await onRefreshModels();
      }
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to scan local models", e);
    } finally {
      setScanningModels(false);
    }
  };

  const handleUpdateTypography = (updates: Partial<TypographySettings>) => {
    const next = { ...typography, ...updates };
    setTypography(next);
    saveTypographySettings(next);
  };

  const handleComplete = () => {
    sounds.playSuccess();
    saveTypographySettings(typography);
    localStorage.setItem("locus_onboarding_completed", "true");
    onClose();
  };

  const handleSkip = () => {
    sounds.playClick();
    localStorage.setItem("locus_onboarding_completed", "true");
    onClose();
  };

  const detectedCount = detectedReports?.length || 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in select-none">
      <div className="relative w-full max-w-2xl bg-[#0d0f18] border border-locus-violet/40 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Modal Top Header */}
        <div className="px-6 py-4 bg-gradient-to-r from-locus-violet/20 via-indigo-900/20 to-transparent border-b border-locus-border/80 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-violet-600 to-indigo-700 flex items-center justify-center shadow-glow-violet text-white text-lg font-bold">
              ⚡
            </div>
            <div>
              <h2 className="text-sm font-bold text-white tracking-wide">
                Welcome to LOCUS
              </h2>
              <p className="text-xs text-zinc-400">
                Quick 3-Step Local-First Setup & Preferences
              </p>
            </div>
          </div>

          <button
            onClick={handleSkip}
            className="text-xs text-zinc-400 hover:text-white px-2.5 py-1 rounded-lg hover:bg-white/5 transition-colors font-mono"
            title="Skip Onboarding"
          >
            Skip ✕
          </button>
        </div>

        {/* Step Progress Pills */}
        <div className="px-6 py-2.5 bg-black/30 border-b border-white/5 flex items-center justify-between text-xs font-mono shrink-0">
          <div className="flex items-center gap-2">
            {[
              { num: 1, label: "1. Cloud Keys" },
              { num: 2, label: "2. Local Models" },
              { num: 3, label: "3. Typography" },
            ].map((st) => {
              const isCurrent = step === st.num;
              const isDone = step > st.num;
              return (
                <button
                  key={st.num}
                  onClick={() => {
                    sounds.playClick();
                    setStep(st.num as 1 | 2 | 3);
                  }}
                  className={`px-3 py-1 rounded-lg transition-all flex items-center gap-1.5 ${
                    isCurrent
                      ? "bg-locus-violet/25 text-white border border-locus-violet/50 shadow-glow-violet font-semibold"
                      : isDone
                      ? "bg-white/5 text-emerald-300 border border-emerald-500/30"
                      : "text-zinc-500 hover:text-zinc-300"
                  }`}
                >
                  <span>{isDone ? "✓" : st.num}</span>
                  <span>{st.label}</span>
                </button>
              );
            })}
          </div>

          <span className="text-[11px] text-zinc-500">Step {step} of 3</span>
        </div>

        {/* Step Content Area */}
        <div className="p-6 overflow-y-auto flex-1 space-y-4">
          {/* STEP 1: Cloud API Keys */}
          {step === 1 && (
            <div className="space-y-4 animate-fade-in">
              <div>
                <h3 className="text-sm font-bold text-white flex items-center gap-2">
                  <span>🔑</span> Step 1: Auto-Detect & Import Cloud API Keys
                </h3>
                <p className="text-xs text-zinc-400 mt-1 leading-relaxed">
                  LOCUS searches your system environment variables and <code className="text-violet-300 bg-white/5 px-1 py-0.5 rounded">.env</code> files for Google Gemini, Groq, OpenRouter, and DeepSeek credentials.
                </p>
              </div>

              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="text-zinc-300 font-semibold">Detected Keys in Environment:</span>
                    {scanningKeys ? (
                      <span className="text-violet-400 animate-pulse">Scanning…</span>
                    ) : (
                      <span className="px-2 py-0.5 rounded-full bg-violet-500/15 text-violet-300 border border-violet-500/30 font-bold">
                        {detectedCount} Provider(s) Found
                      </span>
                    )}
                  </div>

                  <button
                    onClick={handleScanKeys}
                    disabled={scanningKeys}
                    className="btn-secondary py-1 px-2.5 text-xs"
                  >
                    ↻ Re-Scan
                  </button>
                </div>

                {/* Detected List */}
                {detectedReports && detectedReports.length > 0 ? (
                  <div className="space-y-2 pt-1">
                    {detectedReports.map((report) => (
                      <div
                        key={report.provider_id}
                        className="flex items-center justify-between p-2.5 rounded-lg bg-white/[0.03] border border-white/5 text-xs font-mono"
                      >
                        <div className="flex items-center gap-2.5">
                          <span className="font-semibold text-white">{report.provider_name}</span>
                          <span className="text-[10px] text-zinc-500">{report.source}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-emerald-400 font-bold text-[11px] bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                            {report.key_masked || "●●●●●●●●"}
                          </span>
                          {report.imported && (
                            <span className="text-[10px] text-emerald-400 font-semibold">✓ In Keyring</span>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="p-4 rounded-lg bg-white/[0.02] border border-dashed border-white/10 text-center text-xs text-zinc-400">
                    {scanningKeys
                      ? "Analyzing environment variables and project files…"
                      : "No API keys found in environment. You can enter them manually later in Settings."}
                  </div>
                )}

                {detectedCount > 0 && (
                  <div className="pt-2 flex items-center justify-between">
                    <span className="text-xs text-emerald-400 font-mono font-semibold flex items-center gap-1.5">
                      <span>✓</span> {importSuccess ? "Keys encrypted and imported to OS Keyring" : "Detected keys loaded"}
                    </span>

                    <span className="text-[10px] text-zinc-500 font-mono">
                      Zero plaintext storage · OS Keyring encrypted
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* STEP 2: Local Models & Ollama */}
          {step === 2 && (
            <div className="space-y-4 animate-fade-in">
              <div>
                <h3 className="text-sm font-bold text-white flex items-center gap-2">
                  <span>🧠</span> Step 2: Local Hardware & Ollama Detection
                </h3>
                <p className="text-xs text-zinc-400 mt-1 leading-relaxed">
                  LOCUS connects to your local Ollama daemon for 100% offline, air-gapped code intelligence without sending data to the cloud.
                </p>
              </div>

              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="text-zinc-300 font-semibold">Local AI Engine Status:</span>
                    {detectedModels.length > 0 ? (
                      <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 font-bold flex items-center gap-1">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                        {detectedModels.length} Model(s) Ready
                      </span>
                    ) : (
                      <span className="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 border border-amber-500/30 font-bold">
                        Standby / No Local Models
                      </span>
                    )}
                  </div>

                  <button
                    onClick={handleScanModels}
                    disabled={scanningModels}
                    className="btn-secondary py-1 px-2.5 text-xs"
                  >
                    {scanningModels ? "Checking…" : "↻ Check Models"}
                  </button>
                </div>

                {detectedModels.length > 0 ? (
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 pt-1">
                    {detectedModels.map((m) => (
                      <div
                        key={m.name}
                        className="p-3 rounded-lg bg-white/[0.03] border border-white/5 space-y-1 font-mono text-xs"
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-bold text-violet-300">{m.name}</span>
                          <span className="text-[10px] text-zinc-500">{m.size || "Local"}</span>
                        </div>
                        <div className="text-[10px] text-zinc-400 flex items-center gap-2">
                          <span className="text-emerald-400">● Ready for inference</span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="p-4 rounded-lg bg-white/[0.02] border border-dashed border-white/10 space-y-2 text-xs text-zinc-400">
                    <p>Ollama was not detected on default port (11434).</p>
                    <div className="p-2.5 rounded bg-black/60 font-mono text-[11px] text-violet-300 flex items-center justify-between">
                      <span>ollama run qwen2.5-coder:7b</span>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* STEP 3: Typography & Coding Fonts */}
          {step === 3 && (
            <div className="space-y-4 animate-fade-in">
              <div>
                <h3 className="text-sm font-bold text-white flex items-center gap-2">
                  <span>🎨</span> Step 3: Developer Typography & Editor Preferences
                </h3>
                <p className="text-xs text-zinc-400 mt-1 leading-relaxed">
                  Select your preferred monospace font and size for code blocks, chat responses, and diff reviews.
                </p>
              </div>

              {/* Font Selector Cards */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                {AVAILABLE_FONTS.map((f) => {
                  const isSelected = typography.monoFont === f.id;
                  return (
                    <button
                      key={f.id}
                      type="button"
                      onClick={() => {
                        sounds.playClick();
                        handleUpdateTypography({ monoFont: f.id });
                      }}
                      className={`p-3 rounded-xl border text-left transition-all ${
                        isSelected
                          ? "bg-locus-violet/20 border-locus-violet/60 shadow-glow-violet ring-1 ring-locus-violet/50"
                          : "bg-black/40 border-white/10 hover:border-white/20"
                      }`}
                    >
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-xs font-bold text-white">{f.name}</span>
                        {isSelected && <span className="text-xs text-locus-violet font-bold">✓ Selected</span>}
                      </div>
                      <p className="text-[11px] text-zinc-400 leading-snug">{f.description}</p>
                    </button>
                  );
                })}
              </div>

              {/* Font Size & Ligatures Controls */}
              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-zinc-300">
                    Code Font Size: <span className="font-mono text-violet-400">{typography.codeFontSize}px</span>
                  </span>
                  <input
                    type="range"
                    min={12}
                    max={18}
                    step={1}
                    value={typography.codeFontSize}
                    onChange={(e) =>
                      handleUpdateTypography({
                        codeFontSize: parseInt(e.target.value, 10),
                        chatFontSize: parseInt(e.target.value, 10),
                      })
                    }
                    className="w-48 accent-locus-violet cursor-pointer"
                  />
                </div>

                <div className="flex items-center justify-between pt-1 border-t border-white/5">
                  <label className="flex items-center gap-2 text-xs text-zinc-300 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={typography.ligatures}
                      onChange={(e) => handleUpdateTypography({ ligatures: e.target.checked })}
                      className="rounded bg-black border-white/20 text-locus-violet focus:ring-locus-violet"
                    />
                    <span>Enable Programming Ligatures (<code className="font-mono text-violet-300">=&gt;</code>, <code className="font-mono text-violet-300">!=</code>, <code className="font-mono text-violet-300">===</code>)</span>
                  </label>
                </div>
              </div>

              {/* Live Interactive Code Preview */}
              <div className="space-y-1.5">
                <span className="text-[10px] uppercase font-bold text-zinc-500 tracking-wider">
                  Live Typography Preview
                </span>
                <div
                  className="p-3.5 rounded-xl bg-[#06080e] border border-white/10 text-zinc-200 overflow-x-auto code-editor-font transition-all"
                  style={{
                    fontFamily: AVAILABLE_FONTS.find((f) => f.id === typography.monoFont)?.family,
                    fontSize: `${typography.codeFontSize}px`,
                  }}
                >
                  <pre className="m-0 leading-relaxed font-mono">
                    {`pub async fn execute_task(ctx: &Context) -> Result<()> {\n    let is_valid = hash != 0x00 && status === "Ready";\n    tracing::info!("LOCUS Neural Engine OK => {}", is_valid);\n    Ok(())\n}`}
                  </pre>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Modal Bottom Footer */}
        <div className="px-6 py-4 bg-[#090a12] border-t border-locus-border/80 flex items-center justify-between shrink-0">
          <div>
            {step > 1 ? (
              <button
                type="button"
                onClick={() => {
                  sounds.playClick();
                  setStep((s) => (s - 1) as 1 | 2);
                }}
                className="btn-secondary py-2 px-3 text-xs"
              >
                ← Back
              </button>
            ) : (
              <button
                type="button"
                onClick={handleSkip}
                className="btn-ghost text-xs"
              >
                Skip for now
              </button>
            )}
          </div>

          <div className="flex items-center gap-2">
            {step < 3 ? (
              <button
                type="button"
                onClick={() => {
                  sounds.playClick();
                  setStep((s) => (s + 1) as 2 | 3);
                }}
                className="btn-primary py-2 px-4 text-xs font-semibold"
              >
                Next Step →
              </button>
            ) : (
              <button
                type="button"
                onClick={handleComplete}
                className="btn-primary py-2 px-5 text-xs font-bold bg-gradient-to-r from-emerald-600 to-teal-700 shadow-glow-emerald"
              >
                Launch LOCUS 🚀
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
