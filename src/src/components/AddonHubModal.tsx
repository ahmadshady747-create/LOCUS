import React, { useEffect, useState } from "react";
import { pluginsRegistry, pluginsTools, slots } from "../lib/api";
import { sounds } from "../lib/sound";
import type { CircuitState, InstalledAddon, LocalToolManifest, SlotDescriptor, SlotsConfig } from "../types";

interface AddonHubModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AddonHubModal: React.FC<AddonHubModalProps> = ({ isOpen, onClose }) => {
  const [activeTab, setActiveTab] = useState<"slots" | "tools" | "addons">("slots");

  // Core Slots state
  const [slotsConfig, setSlotsConfig] = useState<SlotsConfig | null>(null);
  const [slotDescriptors, setSlotDescriptors] = useState<SlotDescriptor[]>([]);
  const [switchingSlot, setSwitchingSlot] = useState<string | null>(null);

  // Local Tools state
  const [localTools, setLocalTools] = useState<LocalToolManifest[]>([]);
  const [circuitStatus, setCircuitStatus] = useState<Record<string, CircuitState>>({});
  const [resettingCircuit, setResettingCircuit] = useState<string | null>(null);

  // Community Addons state
  const [installedAddons, setInstalledAddons] = useState<InstalledAddon[]>([]);
  const [gitUrlInput, setGitUrlInput] = useState("");
  const [installingGit, setInstallingGit] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadAllData();
    }
  }, [isOpen]);

  const loadAllData = async () => {
    try {
      const [cfg, descs, toolsList, cStatus, addonsList] = await Promise.all([
        slots.getConfig().catch(() => null),
        slots.listAvailable().catch(() => []),
        pluginsTools.listLocalTools().catch(() => []),
        pluginsTools.getCircuitStatus().catch(() => ({})),
        pluginsRegistry.list().catch(() => []),
      ]);

      if (cfg) setSlotsConfig(cfg);
      setSlotDescriptors(descs);
      setLocalTools(toolsList);
      setCircuitStatus(cStatus);
      setInstalledAddons(addonsList);
    } catch {
      // Ignore
    }
  };

  const handleSwitchDriver = async (slotType: "context" | "sandbox", driverId: string) => {
    try {
      sounds.playClick();
      setSwitchingSlot(driverId);
      const updated = await slots.setDriver(slotType, driverId);
      setSlotsConfig(updated);
      sounds.playSuccess();
    } catch (e) {
      console.error(e);
    } finally {
      setSwitchingSlot(null);
    }
  };

  const handleResetCircuit = async (toolId: string) => {
    try {
      sounds.playClick();
      setResettingCircuit(toolId);
      await pluginsTools.resetCircuit(toolId);
      const updatedStatus = await pluginsTools.getCircuitStatus();
      setCircuitStatus(updatedStatus);
      sounds.playSuccess();
    } catch (e) {
      console.error(e);
    } finally {
      setResettingCircuit(null);
    }
  };

  const handleInstallGit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!gitUrlInput.trim()) return;

    try {
      sounds.playClick();
      setInstallingGit(true);
      setInstallError(null);
      await pluginsRegistry.installGit(gitUrlInput.trim());
      sounds.playSuccess();
      setGitUrlInput("");
      const updatedAddons = await pluginsRegistry.list();
      setInstalledAddons(updatedAddons);
    } catch (err: any) {
      setInstallError(err?.toString() || "Failed to clone and install addon");
    } finally {
      setInstallingGit(false);
    }
  };

  const handleToggleAddon = async (addonId: string, enabled: boolean) => {
    try {
      sounds.playClick();
      await pluginsRegistry.toggle(addonId, enabled);
      setInstalledAddons((prev) =>
        prev.map((a) => (a.manifest.id === addonId ? { ...a, enabled } : a))
      );
    } catch (e) {
      console.error(e);
    }
  };

  const handleUninstallAddon = async (addonId: string) => {
    try {
      sounds.playClick();
      await pluginsRegistry.uninstall(addonId);
      setInstalledAddons((prev) => prev.filter((a) => a.manifest.id !== addonId));
      sounds.playSuccess();
    } catch (e) {
      console.error(e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
      <div className="w-full max-w-4xl max-h-[85vh] bg-[#0A0D14] border border-teal-500/30 rounded-2xl shadow-2xl flex flex-col overflow-hidden text-zinc-100">
        {/* Top Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-white/10 bg-[#0E121B]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-xl bg-teal-500/20 border border-teal-500/40 flex items-center justify-center text-teal-300 text-base shadow-sm">
              🧩
            </div>
            <div>
              <h2 className="text-base font-bold text-white flex items-center gap-2">
                LOCUS Addon Hub & Core Slots
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-teal-500/15 text-teal-300 border border-teal-500/30">
                  MODULAR OS
                </span>
              </h2>
              <p className="text-xs text-zinc-400">
                Hot-swappable subsystem slots, zero-panic local tools, and decentralized Git plugins
              </p>
            </div>
          </div>

          <button
            onClick={() => {
              sounds.playClick();
              onClose();
            }}
            className="p-1.5 rounded-lg text-zinc-400 hover:text-white hover:bg-white/10 transition-colors"
            title="Close (Esc)"
          >
            <svg width={16} height={16} viewBox="0 0 16 16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Tab Navigation */}
        <div className="flex items-center gap-1 px-6 pt-3 border-b border-white/5 bg-[#0A0D14] text-xs">
          <button
            onClick={() => {
              sounds.playClick();
              setActiveTab("slots");
            }}
            className={`px-4 py-2 font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
              activeTab === "slots"
                ? "border-teal-400 text-teal-300 bg-teal-500/10"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>⚙️</span> Core Swappable Slots
          </button>
          <button
            onClick={() => {
              sounds.playClick();
              setActiveTab("tools");
            }}
            className={`px-4 py-2 font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
              activeTab === "tools"
                ? "border-teal-400 text-teal-300 bg-teal-500/10"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>⚡</span> Local Tools & Circuit Breaker
            {localTools.length > 0 && (
              <span className="px-1.5 py-0.2 rounded-full bg-white/10 text-zinc-300 text-[10px] font-mono">
                {localTools.length}
              </span>
            )}
          </button>
          <button
            onClick={() => {
              sounds.playClick();
              setActiveTab("addons");
            }}
            className={`px-4 py-2 font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
              activeTab === "addons"
                ? "border-teal-400 text-teal-300 bg-teal-500/10"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            <span>🌐</span> Community Addons (Git)
            {installedAddons.length > 0 && (
              <span className="px-1.5 py-0.2 rounded-full bg-teal-500/20 text-teal-300 text-[10px] font-mono">
                {installedAddons.length}
              </span>
            )}
          </button>
        </div>

        {/* Tab Contents */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* TAB 1: Core Swappable Slots */}
          {activeTab === "slots" && (
            <div className="space-y-6">
              {/* Context Slot */}
              <div className="p-4 rounded-xl border border-white/10 bg-white/5 space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-sm font-bold text-white flex items-center gap-2">
                      <span>🔍</span> Context Retrieval Slot
                    </h3>
                    <p className="text-xs text-zinc-400">
                      Engine used by LOCUS to search and rank repository context
                    </p>
                  </div>
                  <span className="text-[11px] font-mono px-2.5 py-1 rounded bg-teal-500/20 text-teal-300 border border-teal-500/40">
                    Active: {slotsConfig?.active_context_driver.toUpperCase() ?? "BM25"}
                  </span>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                  {slotDescriptors
                    .filter((d) => d.slot_type === "context")
                    .map((desc) => {
                      const isActive = slotsConfig?.active_context_driver === desc.id;
                      return (
                        <div
                          key={desc.id}
                          className={`p-3 rounded-lg border transition-all flex flex-col justify-between ${
                            isActive
                              ? "bg-teal-950/30 border-teal-500/50 shadow-md shadow-teal-500/5"
                              : "bg-black/30 border-white/5 hover:border-white/20"
                          }`}
                        >
                          <div>
                            <div className="flex items-center justify-between">
                              <span className="text-xs font-bold text-white">{desc.name}</span>
                              {isActive && (
                                <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300">
                                  ACTIVE
                                </span>
                              )}
                            </div>
                            <p className="text-[11px] text-zinc-400 mt-1">{desc.description}</p>
                          </div>

                          <button
                            onClick={() => handleSwitchDriver("context", desc.id)}
                            disabled={isActive || switchingSlot === desc.id}
                            className={`mt-3 w-full py-1.5 rounded text-xs font-medium transition-all ${
                              isActive
                                ? "bg-teal-600/30 text-teal-300 cursor-default border border-teal-500/30"
                                : "bg-white/10 hover:bg-white/20 text-white"
                            }`}
                          >
                            {switchingSlot === desc.id
                              ? "Switching..."
                              : isActive
                              ? "Currently Active"
                              : "Switch to Driver"}
                          </button>
                        </div>
                      );
                    })}
                </div>
              </div>

              {/* Sandbox Slot */}
              <div className="p-4 rounded-xl border border-white/10 bg-white/5 space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-sm font-bold text-white flex items-center gap-2">
                      <span>🛡️</span> Sandbox Execution Slot
                    </h3>
                    <p className="text-xs text-zinc-400">
                      Subsystem responsible for command execution and security isolation
                    </p>
                  </div>
                  <span className="text-[11px] font-mono px-2.5 py-1 rounded bg-teal-500/20 text-teal-300 border border-teal-500/40">
                    Active: {slotsConfig?.active_sandbox_driver.toUpperCase() ?? "NATIVE"}
                  </span>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                  {slotDescriptors
                    .filter((d) => d.slot_type === "sandbox")
                    .map((desc) => {
                      const isActive = slotsConfig?.active_sandbox_driver === desc.id;
                      return (
                        <div
                          key={desc.id}
                          className={`p-3 rounded-lg border transition-all flex flex-col justify-between ${
                            isActive
                              ? "bg-teal-950/30 border-teal-500/50 shadow-md shadow-teal-500/5"
                              : "bg-black/30 border-white/5 hover:border-white/20"
                          }`}
                        >
                          <div>
                            <div className="flex items-center justify-between">
                              <span className="text-xs font-bold text-white">{desc.name}</span>
                              {isActive && (
                                <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300">
                                  ACTIVE
                                </span>
                              )}
                            </div>
                            <p className="text-[11px] text-zinc-400 mt-1">{desc.description}</p>
                          </div>

                          <button
                            onClick={() => handleSwitchDriver("sandbox", desc.id)}
                            disabled={isActive || switchingSlot === desc.id}
                            className={`mt-3 w-full py-1.5 rounded text-xs font-medium transition-all ${
                              isActive
                                ? "bg-teal-600/30 text-teal-300 cursor-default border border-teal-500/30"
                                : "bg-white/10 hover:bg-white/20 text-white"
                            }`}
                          >
                            {switchingSlot === desc.id
                              ? "Switching..."
                              : isActive
                              ? "Currently Active"
                              : "Switch to Driver"}
                          </button>
                        </div>
                      );
                    })}
                </div>
              </div>
            </div>
          )}

          {/* TAB 2: Local Tools & Circuit Breaker */}
          {activeTab === "tools" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between pb-2">
                <div>
                  <h3 className="text-sm font-bold text-white flex items-center gap-2">
                    <span>🛠️</span> Discovered Local Tools
                  </h3>
                  <p className="text-xs text-zinc-400">
                    Scripts placed in <code className="text-teal-300">.locus/tools/</code> and <code className="text-teal-300">~/.locus/tools/</code>
                  </p>
                </div>
                <button
                  onClick={loadAllData}
                  className="px-3 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs font-mono text-zinc-300 border border-white/10"
                >
                  ↻ Rescan Tools
                </button>
              </div>

              {localTools.length === 0 ? (
                <div className="p-8 text-center rounded-xl border border-white/5 bg-white/[0.02] space-y-2">
                  <span className="text-2xl">📂</span>
                  <p className="text-xs text-zinc-400">No local tools found in workspace.</p>
                  <p className="text-[11px] text-zinc-500">
                    Add a script (e.g. <code className="text-zinc-400">.locus/tools/format.py</code>) with headers <code className="text-zinc-400"># @name: My Tool</code> to register automatically.
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {localTools.map((tool) => {
                    const cState = circuitStatus[tool.id] || { state: "closed" };
                    const isOpen = cState.state === "open";
                    return (
                      <div
                        key={tool.id}
                        className={`p-4 rounded-xl border transition-all flex flex-col md:flex-row md:items-center justify-between gap-3 ${
                          isOpen
                            ? "bg-rose-950/20 border-rose-500/40"
                            : "bg-white/5 border-white/10"
                        }`}
                      >
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-bold text-white">{tool.name}</span>
                            <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-white/5 text-zinc-300 border border-white/10">
                              {tool.is_global ? "Global" : "Workspace"}
                            </span>
                            {/* Circuit Status Badge */}
                            {isOpen ? (
                              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-rose-500/20 text-rose-300 border border-rose-500/40 animate-pulse">
                                🚨 CIRCUIT OPEN ({cState.failure_count} failures)
                              </span>
                            ) : cState.state === "half_open" ? (
                              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-500/20 text-amber-300 border border-amber-500/40">
                                ⚠️ HALF OPEN
                              </span>
                            ) : (
                              <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                                🛡️ HEALTHY
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-zinc-400">{tool.description}</p>
                          <div className="flex flex-wrap items-center gap-2 pt-1 text-[11px] font-mono text-zinc-500">
                            <span>⏱️ Timeout: {tool.timeout_secs}s</span>
                            {tool.shebang && <span>• Shebang: {tool.shebang}</span>}
                            <span>• Path: {tool.script_path}</span>
                          </div>
                        </div>

                        {isOpen && (
                          <button
                            onClick={() => handleResetCircuit(tool.id)}
                            disabled={resettingCircuit === tool.id}
                            className="px-3 py-1.5 rounded-lg bg-rose-600/30 hover:bg-rose-600/40 text-rose-200 border border-rose-500/50 text-xs font-medium shrink-0"
                          >
                            {resettingCircuit === tool.id ? "Resetting..." : "Reset Circuit Breaker"}
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* TAB 3: Community Addons (Git) */}
          {activeTab === "addons" && (
            <div className="space-y-6">
              {/* Git Installer Input */}
              <div className="p-4 rounded-xl border border-teal-500/30 bg-teal-950/10 space-y-3">
                <div>
                  <h3 className="text-sm font-bold text-white flex items-center gap-2">
                    <span>🌐</span> Install Community Addon from Git
                  </h3>
                  <p className="text-xs text-zinc-400">
                    Enter a repository shorthand (e.g. <code className="text-teal-300">github:user/locus-addon</code>) or full HTTPS Git URL
                  </p>
                </div>

                <form onSubmit={handleInstallGit} className="flex gap-2">
                  <input
                    type="text"
                    value={gitUrlInput}
                    onChange={(e) => setGitUrlInput(e.target.value)}
                    placeholder="e.g. github:locus-community/rust-ast-tools"
                    disabled={installingGit}
                    className="flex-1 bg-black/50 border border-white/10 rounded-lg px-3.5 py-2 text-xs text-white placeholder:text-zinc-600 focus:outline-none focus:border-teal-500"
                  />
                  <button
                    type="submit"
                    disabled={installingGit || !gitUrlInput.trim()}
                    className="px-4 py-2 rounded-lg bg-teal-600 hover:bg-teal-500 text-white font-medium text-xs transition-all disabled:opacity-50 flex items-center gap-1.5 shrink-0"
                  >
                    {installingGit ? (
                      <>
                        <span className="animate-spin">🌀</span> Cloning...
                      </>
                    ) : (
                      <>
                        <span>📥</span> Install Addon
                      </>
                    )}
                  </button>
                </form>

                {installError && (
                  <div className="p-2.5 rounded bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs font-mono">
                    ⚠️ {installError}
                  </div>
                )}
              </div>

              {/* Installed Addons List */}
              <div className="space-y-3">
                <h4 className="text-xs font-bold text-zinc-400 uppercase tracking-wider">
                  Installed Addons ({installedAddons.length})
                </h4>

                {installedAddons.length === 0 ? (
                  <div className="p-8 text-center rounded-xl border border-white/5 bg-white/[0.02] text-xs text-zinc-500">
                    No community addons installed yet. Clone your first addon above!
                  </div>
                ) : (
                  <div className="space-y-3">
                    {installedAddons.map((addon) => (
                      <div
                        key={addon.manifest.id}
                        className="p-4 rounded-xl border border-white/10 bg-white/5 flex flex-col md:flex-row md:items-center justify-between gap-3"
                      >
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-bold text-white">{addon.manifest.name}</span>
                            <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-teal-500/20 text-teal-300 border border-teal-500/30">
                              v{addon.manifest.version}
                            </span>
                            <span className="text-xs text-zinc-500">by {addon.manifest.author}</span>
                          </div>
                          <p className="text-xs text-zinc-400">{addon.manifest.description}</p>
                          <div className="text-[11px] font-mono text-zinc-500">
                            Path: {addon.install_path}
                          </div>
                        </div>

                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => handleToggleAddon(addon.manifest.id, !addon.enabled)}
                            className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-all ${
                              addon.enabled
                                ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                                : "bg-white/5 text-zinc-400 border-white/10"
                            }`}
                          >
                            {addon.enabled ? "Enabled" : "Disabled"}
                          </button>
                          <button
                            onClick={() => handleUninstallAddon(addon.manifest.id)}
                            className="p-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 border border-rose-500/30 text-xs"
                            title="Uninstall Addon"
                          >
                            🗑️
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default AddonHubModal;
