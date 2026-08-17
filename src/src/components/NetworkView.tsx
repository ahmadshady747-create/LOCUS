import { useEffect, useState } from "react";
import type { AppState, LocalDeviceSimple } from "../types";
import { network } from "../lib/api";
import { sounds } from "../lib/sound";

interface NetworkViewProps {
  state: AppState;
  onNavigate?: (tab: string) => void;
}

export default function NetworkView({ state }: NetworkViewProps) {
  const [isRunning, setIsRunning] = useState(false);
  const [devices, setDevices] = useState<LocalDeviceSimple[]>(state.devices);
  const [refreshing, setRefreshing] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState<LocalDeviceSimple | null>(null);

  const loadNetworkState = async () => {
    setRefreshing(true);
    try {
      const devList = await network.discover();
      setDevices(devList);
      setIsRunning(true);
    } catch (e) {
      console.warn("Network discovery notice:", e);
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    loadNetworkState();
    const timer = setInterval(loadNetworkState, 15000);
    return () => clearInterval(timer);
  }, []);

  const toggleNetwork = async () => {
    sounds.playClick();
    if (isRunning) {
      try {
        await network.stop();
        setIsRunning(false);
        setDevices([]);
      } catch (e) {
        console.error("Failed to stop network", e);
      }
    } else {
      try {
        await network.start();
        setIsRunning(true);
        sounds.playSuccess();
        await loadNetworkState();
      } catch (e) {
        console.error("Failed to start network", e);
      }
    }
  };

  return (
    <div className="h-full flex flex-col overflow-hidden bg-[#07090e] p-6 space-y-6">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-4 pb-4 border-b border-white/5 shrink-0">
        <div>
          <div className="flex items-center gap-2.5">
            <span className="text-xl">🌐</span>
            <h2 className="text-lg font-bold text-white font-mono tracking-tight">
              P2P Mesh Network & Distributed Compute
            </h2>
            <span
              className={`text-[10px] font-mono px-2 py-0.5 rounded-full border ${
                isRunning
                  ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                  : "bg-zinc-500/15 text-zinc-400 border-zinc-500/30"
              }`}
            >
              {isRunning ? "● MESH ACTIVE" : "○ STANDBY"}
            </span>
          </div>
          <p className="text-xs text-zinc-400 mt-1">
            Zero-config local network discovery (UDP Broadcast & TCP) to offload heavy inference and indexing across local nodes.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => {
              sounds.playClick();
              loadNetworkState();
            }}
            disabled={refreshing}
            className="btn-secondary text-xs py-2 px-3.5 flex items-center gap-2 font-mono"
          >
            {refreshing ? (
              <>
                <span className="animate-spin text-violet-400">↻</span> Discovering…
              </>
            ) : (
              <>🔄 Scan Local LAN</>
            )}
          </button>

          <button
            onClick={toggleNetwork}
            className={`text-xs py-2 px-4 rounded-xl font-mono font-bold transition-all shadow-sm ${
              isRunning
                ? "bg-rose-500/15 text-rose-300 border border-rose-500/30 hover:bg-rose-500/25"
                : "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 hover:bg-emerald-500/30 shadow-glow-emerald"
            }`}
          >
            {isRunning ? "⏹ Stop Daemon" : "▶ Start P2P Mesh"}
          </button>
        </div>
      </div>

      {/* Grid Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 shrink-0">
        <div className="p-4 rounded-xl bg-black/40 border border-white/5 shadow-sm space-y-1">
          <div className="text-[10px] text-zinc-400 uppercase font-mono tracking-wider">Discovered Peers</div>
          <div className="text-2xl font-bold font-mono text-cyan-400">{devices.length}</div>
          <div className="text-[11px] text-zinc-500 font-mono">LAN Nodes connected</div>
        </div>

        <div className="p-4 rounded-xl bg-black/40 border border-white/5 shadow-sm space-y-1">
          <div className="text-[10px] text-zinc-400 uppercase font-mono tracking-wider">Load Balancer State</div>
          <div className="text-2xl font-bold font-mono text-emerald-400">
            {isRunning ? "Round-Robin" : "Offline"}
          </div>
          <div className="text-[11px] text-zinc-500 font-mono">Specialization matching active</div>
        </div>

        <div className="p-4 rounded-xl bg-black/40 border border-white/5 shadow-sm space-y-1">
          <div className="text-[10px] text-zinc-400 uppercase font-mono tracking-wider">Protocol Security</div>
          <div className="text-2xl font-bold font-mono text-violet-400">P2P v2.1</div>
          <div className="text-[11px] text-zinc-500 font-mono">HMAC Payload Verification</div>
        </div>
      </div>

      {/* Peer Devices List */}
      <div className="flex-1 min-h-0 flex flex-col space-y-3">
        <h3 className="text-xs font-bold uppercase tracking-wider text-zinc-400 font-mono">
          Connected Mesh Topology
        </h3>

        {devices.length === 0 ? (
          <div className="flex-1 rounded-xl border border-white/5 bg-black/20 p-8 flex flex-col items-center justify-center text-center space-y-3">
            <div className="w-12 h-12 rounded-2xl bg-violet-500/10 border border-violet-500/20 flex items-center justify-center text-xl text-violet-400">
              📡
            </div>
            <div className="space-y-1">
              <div className="text-sm font-bold text-white font-mono">No other LOCUS peers detected on LAN</div>
              <p className="text-xs text-zinc-500 max-w-md">
                Launch LOCUS on other machines in the same Wi-Fi or Ethernet network to automatically distribute computation.
              </p>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 overflow-y-auto pr-1 flex-1">
            {devices.map((d, i) => (
              <div
                key={d.id ?? i}
                onClick={() => {
                  sounds.playClick();
                  setSelectedDevice(d);
                }}
                className={`p-4 rounded-xl border transition-all cursor-pointer ${
                  selectedDevice?.id === d.id
                    ? "bg-violet-950/20 border-violet-500/40 shadow-glow-violet"
                    : "bg-black/30 border-white/5 hover:border-white/15"
                }`}
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2.5">
                    <span className="text-lg">💻</span>
                    <div>
                      <div className="text-sm font-bold text-white font-mono">{d.name}</div>
                      <div className="text-[10px] text-zinc-500 font-mono">{d.device_type} Node</div>
                    </div>
                  </div>
                  <span className="text-[10px] font-mono font-semibold px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 flex items-center gap-1">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                    Online
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-2 text-[11px] font-mono text-zinc-400 bg-black/30 p-2.5 rounded-lg border border-white/5">
                  <div>
                    <span className="text-zinc-500">Address:</span> {d.ip_address}
                  </div>
                  <div>
                    <span className="text-zinc-500">Port:</span> {d.port}
                  </div>
                  <div>
                    <span className="text-zinc-500">VRAM:</span> {d.vram_gb ? `${d.vram_gb} GB` : "Shared CPU"}
                  </div>
                  <div>
                    <span className="text-zinc-500">Status:</span> {d.status}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
