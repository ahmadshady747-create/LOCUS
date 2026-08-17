import { useCallback, useEffect, useState } from "react";
import MissionControl from "./components/MissionControl";
import Dashboard from "./components/Dashboard";
import Settings from "./components/Settings";
import DiagnosticsView from "./components/DiagnosticsView";
import StatusBar from "./components/StatusBar";
import OnboardingModal from "./components/OnboardingModal";
import SpotlightHUD from "./components/SpotlightHUD";
import NewtonCompanion from "./components/NewtonCompanion";
import type { AppState, PrivacyMode } from "./types";
import { agents, fs, llm, network } from "./lib/api";
import { sounds } from "./lib/sound";
import { applyTypographySettings, getTypographySettings } from "./lib/theme";
import { useTranslation } from "./i18n";

export type Tab = "mission_control" | "workspace" | "settings" | "diagnostics";

interface TabItem {
  id: Tab;
  label: string;
  badge?: string;
  shortcut: string;
  icon: (props: { active?: boolean }) => React.ReactNode;
}

const TABS: TabItem[] = [
  { id: "mission_control", label: "Mission Control", shortcut: "Ctrl+1", icon: MissionControlIcon },
  { id: "workspace", label: "Workspace & Diffs", shortcut: "Ctrl+2", icon: WorkspaceIcon },
  { id: "settings", label: "Settings & Key Vault", shortcut: "Ctrl+3", icon: SettingsIcon },
  { id: "diagnostics", label: "Diagnostics & Skills", shortcut: "Ctrl+4", icon: DiagnosticsIcon },
];

function MissionControlIcon({ active }: { active?: boolean }) {
  return (
    <svg width={18} height={18} viewBox="0 0 20 20" fill="none" className={active ? "text-violet-400" : "text-locus-muted"}>
      <circle cx="10" cy="10" r="7" stroke="currentColor" strokeWidth={1.6} />
      <circle cx="10" cy="10" r="3" stroke="currentColor" strokeWidth={1.6} />
      <path d="M10 2v2M10 16v2M2 10h2M16 10h2" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" />
    </svg>
  );
}

function WorkspaceIcon({ active }: { active?: boolean }) {
  return (
    <svg width={18} height={18} viewBox="0 0 20 20" fill="none" className={active ? "text-violet-400" : "text-locus-muted"}>
      <path
        d="M3 4.5A1.5 1.5 0 0 1 4.5 3h3.879a1.5 1.5 0 0 1 1.06.44l1.122 1.12a1.5 1.5 0 0 0 1.06.44H15.5A1.5 1.5 0 0 1 17 6.5v9a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 3 15.5v-11z"
        stroke="currentColor"
        strokeWidth={1.6}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M7 11.5l2 2 4-4" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SettingsIcon({ active }: { active?: boolean }) {
  return (
    <svg width={18} height={18} viewBox="0 0 20 20" fill="none" className={active ? "text-violet-400" : "text-locus-muted"}>
      <circle cx="10" cy="10" r="3.2" stroke="currentColor" strokeWidth={1.6} />
      <path
        d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M4.7 15.3l1.4-1.4M13.9 6.1l1.4-1.4"
        stroke="currentColor"
        strokeWidth={1.6}
        strokeLinecap="round"
      />
    </svg>
  );
}

function DiagnosticsIcon({ active }: { active?: boolean }) {
  return (
    <svg width={18} height={18} viewBox="0 0 20 20" fill="none" className={active ? "text-violet-400" : "text-locus-muted"}>
      <path
        d="M3 10.5h3.5l2-5 3 9 2-4H17"
        stroke="currentColor"
        strokeWidth={1.6}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth={1.5} opacity="0.4" />
    </svg>
  );
}

function ChevronLeftIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="none" className={className}>
      <path d="M10 4L6 8l4 4" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function ChevronRightIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="none" className={className}>
      <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function App() {
  // Check if rendered in floating Spotlight HUD window
  const isSpotlight =
    typeof window !== "undefined" &&
    (window.location.search.includes("window=spotlight") ||
      window.location.hash.includes("spotlight"));

  if (isSpotlight) {
    return <SpotlightHUD />;
  }

  const [tab, setTab] = useState<Tab>("mission_control");
  const [appState, setAppState] = useState<AppState>({
    workspaceRoot: null,
    workspace: null,
    models: [],
    devices: [],
    activeAgents: [],
    selectedModel: null,
    privacyMode: "local",
    hybridPercent: 100,
  });
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [onboardingOpen, setOnboardingOpen] = useState(() => {
    try {
      return localStorage.getItem("locus_onboarding_completed") !== "true";
    } catch {
      return false;
    }
  });

  const switchTab = useCallback((nextTab: Tab) => {
    sounds.playClick();
    setTab(nextTab);
  }, []);

  const toggleSidebar = useCallback(() => {
    sounds.playClick();
    setSidebarCollapsed((prev) => !prev);
  }, []);

  useEffect(() => {
    const currentTypography = getTypographySettings();
    applyTypographySettings(currentTypography);
  }, []);

  const refreshWorkspace = useCallback(async () => {
    try {
      const result = await fs.scan();
      setAppState((s) => ({ ...s, workspace: result.index }));
    } catch (e) {
      console.error("Failed to scan workspace", e);
    }
  }, []);

  const refreshModels = useCallback(async () => {
    try {
      const models = await llm.detectModels();
      setAppState((s) => ({
        ...s,
        models,
        selectedModel: s.selectedModel ?? models[0]?.name ?? null,
      }));
    } catch (e) {
      console.error("Failed to detect models", e);
    }
  }, []);

  const refreshDevices = useCallback(async () => {
    try {
      const devices = await network.discover();
      setAppState((s) => ({ ...s, devices }));
    } catch (e) {
      console.error("Failed to discover devices", e);
    }
  }, []);

  const refreshAgents = useCallback(async () => {
    try {
      const activeAgents = await agents.listActive();
      setAppState((s) => ({ ...s, activeAgents }));
    } catch (e) {
      console.error("Failed to list agents", e);
    }
  }, []);

  useEffect(() => {
    refreshWorkspace();
    refreshModels();
    refreshDevices();
    refreshAgents();

    const modelTimer = setInterval(refreshModels, 15000);
    const deviceTimer = setInterval(refreshDevices, 30000);
    const agentTimer = setInterval(refreshAgents, 5000);

    // Global keyboard shortcuts (Ctrl+B and Ctrl+1..4)
    const handleKeyDown = (e: KeyboardEvent) => {
      const isCmdOrCtrl = e.ctrlKey || e.metaKey;

      if (isCmdOrCtrl && e.key.toLowerCase() === "b") {
        e.preventDefault();
        toggleSidebar();
      } else if (isCmdOrCtrl && e.key === "1") {
        e.preventDefault();
        switchTab("mission_control");
      } else if (isCmdOrCtrl && e.key === "2") {
        e.preventDefault();
        switchTab("workspace");
      } else if (isCmdOrCtrl && e.key === "3") {
        e.preventDefault();
        switchTab("settings");
      } else if (isCmdOrCtrl && e.key === "4") {
        e.preventDefault();
        switchTab("diagnostics");
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      clearInterval(modelTimer);
      clearInterval(deviceTimer);
      clearInterval(agentTimer);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [refreshWorkspace, refreshModels, refreshDevices, refreshAgents, switchTab, toggleSidebar]);

  const setPrivacyMode = useCallback((mode: PrivacyMode) => {
    setAppState((s) => ({
      ...s,
      privacyMode: mode,
      hybridPercent: mode === "local" ? 100 : 99,
    }));
  }, []);

  return (
    <div className="flex h-full bg-[#08090D] font-sans text-locus-text select-none antialiased">
      {/* Navigation Sidebar */}
      <Sidebar
        tabs={TABS}
        activeTab={tab}
        onTabChange={switchTab}
        collapsed={sidebarCollapsed}
        onToggleCollapse={toggleSidebar}
        appState={appState}
      />

      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Clean, High-Density Header */}
        <header className="h-10 px-4 border-b border-locus-border/80 bg-[#090A10] flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
            <button
              onClick={toggleSidebar}
              className="p-1 rounded text-locus-muted hover:text-white hover:bg-white/5 transition-colors"
              title={sidebarCollapsed ? "Expand Sidebar (Ctrl+B)" : "Collapse Sidebar (Ctrl+B)"}
              aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {sidebarCollapsed ? <ChevronRightIcon /> : <ChevronLeftIcon />}
            </button>
            <span className="text-xs font-mono font-bold tracking-wider text-white">LOCUS</span>
          </div>

          <div className="flex items-center gap-2.5 text-[11px] font-mono">
            <button
              onClick={() => {
                sounds.playClick();
                setOnboardingOpen(true);
              }}
              className="text-zinc-400 hover:text-white px-2 py-1 rounded bg-white/5 hover:bg-white/10 border border-white/10 transition-colors flex items-center gap-1.5 text-[10px]"
              title="Open Setup & Onboarding Wizard"
            >
              <span>⚡ Setup Wizard</span>
            </button>

            {appState.workspaceRoot ? (
              <div className="flex items-center gap-1.5 text-zinc-300 max-w-[320px] truncate bg-black/40 px-2.5 py-1 rounded border border-white/10">
                <span className="text-emerald-400 text-[8px]">●</span>
                <span className="truncate">{appState.workspaceRoot}</span>
              </div>
            ) : (
              <button
                onClick={() => switchTab("settings")}
                className="text-violet-400 hover:text-violet-300 transition-colors text-[10px] font-semibold flex items-center gap-1"
              >
                <span>+ Open Workspace Folder</span>
              </button>
            )}
          </div>
        </header>

        {/* 4 Core Workspaces */}
        <div className="flex-1 overflow-hidden">
          {tab === "mission_control" && <MissionControl state={appState} onNavigate={(t) => switchTab(t as Tab)} />}
          {tab === "workspace" && <Dashboard state={appState} onNavigate={(t) => switchTab(t as Tab)} />}
          {tab === "settings" && (
            <Settings
              state={appState}
              setState={setAppState}
              setPrivacyMode={setPrivacyMode}
              onOpenOnboarding={() => setOnboardingOpen(true)}
            />
          )}
          {tab === "diagnostics" && <DiagnosticsView state={appState} onNavigate={(t) => switchTab(t as Tab)} />}
        </div>
      </main>

      <NewtonCompanion state={appState} />
      <StatusBar state={appState} onNavigate={(t) => switchTab(t as Tab)} />

      <OnboardingModal
        isOpen={onboardingOpen}
        onClose={() => setOnboardingOpen(false)}
        appState={appState}
        onRefreshModels={refreshModels}
      />
    </div>
  );
}

function Sidebar({
  tabs,
  activeTab,
  onTabChange,
  collapsed,
  onToggleCollapse,
  appState,
}: {
  tabs: TabItem[];
  activeTab: Tab;
  onTabChange: (tab: Tab) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  appState: AppState;
}) {
  const { t } = useTranslation();

  return (
    <aside
      className={`flex flex-col border-r border-locus-border/80 bg-[#090b10] transition-all duration-200 ease-in-out ${
        collapsed ? "w-14" : "w-56"
      } shrink-0 overflow-hidden select-none z-10`}
    >
      {/* Navigation Workspaces */}
      <nav className="flex flex-col gap-1 px-2 py-3 flex-1 overflow-y-auto">
        {tabs.map(({ id, icon, label, shortcut }) => {
          const isActive = activeTab === id;
          const localizedLabel = t(`nav.${id}`) || label;
          return (
            <button
              key={id}
              onClick={() => onTabChange(id)}
              className={`group relative flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-left transition-all ${
                isActive
                  ? "bg-violet-600/20 text-white border border-violet-500/40 font-semibold"
                  : "text-zinc-400 hover:text-white hover:bg-white/5 border border-transparent"
              } ${collapsed ? "justify-center px-0" : ""}`}
              title={`${localizedLabel} (${shortcut})`}
            >
              <div className="shrink-0 flex items-center justify-center">
                {icon({ active: isActive })}
              </div>

              {!collapsed && (
                <div className="flex items-center justify-between flex-1 min-w-0">
                  <span className="text-xs truncate">{localizedLabel}</span>
                  <span className="text-[9px] font-mono text-zinc-500 opacity-60 group-hover:opacity-100 transition-opacity">
                    {shortcut}
                  </span>
                </div>
              )}
            </button>
          );
        })}
      </nav>

      {/* Collapse Footer Button */}
      <div className="px-2 py-2 border-t border-white/5">
        <button
          onClick={onToggleCollapse}
          className="w-full flex items-center justify-center gap-2 p-1.5 rounded text-zinc-500 hover:text-zinc-300 hover:bg-white/5 text-[10px] font-mono transition-colors"
          title="Toggle Sidebar (Ctrl+B)"
        >
          {collapsed ? <ChevronRightIcon /> : <><ChevronLeftIcon /> <span className="text-[9px]">Collapse (Ctrl+B)</span></>}
        </button>
      </div>

      {/* Compact System State */}
      {!collapsed && (
        <div className="p-2.5 border-t border-white/5 bg-black/40 shrink-0 space-y-1.5 font-mono text-[10px]">
          <div className="flex items-center justify-between text-zinc-400">
            <span>Model:</span>
            <span className="text-zinc-200 truncate max-w-[110px]">
              {appState.selectedModel ?? "Auto"}
            </span>
          </div>
          <div className="flex items-center justify-between text-zinc-500">
            <span>{appState.models.length} Local</span>
            <span>{appState.devices.length} Peer(s)</span>
          </div>
        </div>
      )}
    </aside>
  );
}

export default App;