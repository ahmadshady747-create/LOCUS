import { useEffect, useState } from "react";
import type { AppState, TaskNode } from "../types";
import { extractArchitectureTree, type ArchitectureTreeData } from "../lib/archTree";
import { sounds } from "../lib/sound";

interface NewtonCompanionProps {
  state: AppState;
  activeTaskNode?: TaskNode | null;
  isRunning?: boolean;
}

export type NewtonState = "idle" | "eureka_drop_apple" | "hop_dissolve";

export default function NewtonCompanion({
  state,
  activeTaskNode,
  isRunning,
}: NewtonCompanionProps) {
  const [visible, setVisible] = useState(() => {
    try {
      return localStorage.getItem("locus_show_newton_companion") !== "false";
    } catch {
      return true;
    }
  });

  const [minimized, setMinimized] = useState(false);
  const [newtonState, setNewtonState] = useState<NewtonState>("idle");
  const [appleY, setAppleY] = useState(38);
  const [lightbulbVisible, setLightbulbVisible] = useState(false);
  const [hoveredBranch, setHoveredBranch] = useState<string | null>(null);

  const activeTarget = activeTaskNode?.payload.target_file ?? (isRunning ? "agents" : null);
  const treeData: ArchitectureTreeData = extractArchitectureTree(
    state.workspace,
    activeTarget
  );

  // Trigger Apple Drop & Eureka when task execution starts
  useEffect(() => {
    if (isRunning || activeTaskNode?.status === "running") {
      setNewtonState("eureka_drop_apple");
      setAppleY(38);
      setLightbulbVisible(false);

      // Animate apple falling
      const dropTimer = setTimeout(() => {
        setAppleY(135);
        setLightbulbVisible(true);
        sounds.playClick();
      }, 300);

      // Transition to hop & dissolve
      const hopTimer = setTimeout(() => {
        setNewtonState("hop_dissolve");
      }, 1800);

      return () => {
        clearTimeout(dropTimer);
        clearTimeout(hopTimer);
      };
    } else {
      setNewtonState("idle");
      setAppleY(38);
      setLightbulbVisible(false);
    }
  }, [isRunning, activeTaskNode?.id, activeTaskNode?.status]);

  const handleDismiss = () => {
    sounds.playClick();
    setVisible(false);
    try {
      localStorage.setItem("locus_show_newton_companion", "false");
    } catch {
      // ignore
    }
  };

  const toggleMinimize = () => {
    sounds.playClick();
    setMinimized((m) => !m);
  };

  if (!visible) return null;

  return (
    <div
      className={`fixed bottom-8 left-4 z-40 transition-all duration-300 select-none ${
        minimized ? "w-10 h-10" : "w-64 h-56"
      }`}
    >
      {/* Minimized Icon Widget */}
      {minimized ? (
        <button
          onClick={toggleMinimize}
          className="w-10 h-10 rounded-xl bg-[#0b0e17]/90 border border-violet-500/40 shadow-glow-violet flex items-center justify-center text-lg hover:scale-105 active:scale-95 transition-all text-white backdrop-blur-md"
          title="Expand Newton & Architecture Tree"
        >
          🍎
        </button>
      ) : (
        /* Full Companion Container */
        <div className="relative w-full h-full bg-[#0a0d16]/90 border border-violet-500/30 rounded-2xl shadow-2xl backdrop-blur-xl p-2.5 flex flex-col justify-between overflow-hidden animate-spring-in gpu-layer">
          {/* Header Controls */}
          <div className="flex items-center justify-between text-[10px] font-mono pb-1 border-b border-white/10 shrink-0">
            <div className="flex items-center gap-1.5 text-zinc-300 font-semibold">
              <span className="text-emerald-400">🌳</span>
              <span>Living Architecture</span>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={toggleMinimize}
                className="text-zinc-400 hover:text-white p-0.5"
                title="Minimize"
              >
                _
              </button>
              <button
                onClick={handleDismiss}
                className="text-zinc-500 hover:text-zinc-300 p-0.5"
                title="Hide (can re-enable in Settings)"
              >
                ✕
              </button>
            </div>
          </div>

          {/* Floating Thought Bubble when Agent DAG Step is active */}
          {activeTaskNode && (
            <div className="absolute top-8 left-3 right-3 bg-violet-950/80 border border-violet-500/50 rounded-lg p-1.5 z-20 shadow-lg text-[9px] font-mono text-violet-200 animate-pulse backdrop-blur-sm truncate">
              <span className="font-bold text-emerald-300">💭 Step {activeTaskNode.title}: </span>
              <span>{activeTaskNode.description}</span>
            </div>
          )}

          {/* Pure SVG Living Tree & Newton Scene */}
          <div className="flex-1 relative w-full h-full">
            <svg
              viewBox="0 0 200 170"
              className="w-full h-full overflow-visible"
              style={{ filter: "drop-shadow(0 2px 8px rgba(0,0,0,0.5))" }}
            >
              <defs>
                {/* Neon Glow Filters */}
                <filter id="neon-glow" x="-20%" y="-20%" width="140%" height="140%">
                  <feGaussianBlur stdDeviation="2.5" result="blur" />
                  <feMerge>
                    <feMergeNode in="blur" />
                    <feMergeNode in="SourceGraphic" />
                  </feMerge>
                </filter>
                <linearGradient id="trunk-grad" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" stopColor="#3b2b1a" />
                  <stop offset="50%" stopColor="#634526" />
                  <stop offset="100%" stopColor="#2c1f13" />
                </linearGradient>
              </defs>

              {/* Ground line */}
              <path
                d="M 10 155 Q 100 150 190 155"
                stroke="#1f2438"
                strokeWidth="2.5"
                strokeLinecap="round"
              />

              {/* Tree Trunk */}
              <path
                d="M 94 153 C 96 125, 93 100, 100 75 C 107 100, 104 125, 106 153 Z"
                fill="url(#trunk-grad)"
              />

              {/* Architecture Branches */}
              {treeData.branches.map((branch) => {
                const isHovered = hoveredBranch === branch.id;
                const strokeColor = branch.isActive ? "#34d399" : branch.color;
                const strokeW = branch.isActive ? 3 : isHovered ? 2.5 : 1.8;

                return (
                  <g
                    key={branch.id}
                    onMouseEnter={() => setHoveredBranch(branch.id)}
                    onMouseLeave={() => setHoveredBranch(null)}
                    className="cursor-pointer transition-all"
                  >
                    {/* Glowing Branch Path */}
                    <path
                      d={`M 100 75 Q ${branch.pathCoordinate.controlX} ${branch.pathCoordinate.controlY} ${branch.pathCoordinate.endX} ${branch.pathCoordinate.endY}`}
                      stroke={strokeColor}
                      strokeWidth={strokeW}
                      strokeLinecap="round"
                      fill="none"
                      filter={branch.isActive ? "url(#neon-glow)" : undefined}
                      className={branch.isActive ? "animate-neon-flow animate-neon-pulse" : ""}
                    />

                    {/* Branch Module Node Dot */}
                    <circle
                      cx={branch.pathCoordinate.endX}
                      cy={branch.pathCoordinate.endY}
                      r={branch.isActive ? 4.5 : isHovered ? 4 : 3}
                      fill={strokeColor}
                      filter={branch.isActive ? "url(#neon-glow)" : undefined}
                    />

                    {/* Branch Label */}
                    <text
                      x={branch.pathCoordinate.endX}
                      y={branch.pathCoordinate.endY - 5}
                      textAnchor="middle"
                      fill={branch.isActive ? "#6ee7b7" : "#94a3b8"}
                      fontSize="7.5"
                      fontFamily="monospace"
                      fontWeight={branch.isActive ? "bold" : "normal"}
                    >
                      {branch.name.split("/")[0].trim()}
                    </text>
                  </g>
                );
              })}

              {/* Falling Apple */}
              <g
                transform={`translate(100, ${appleY})`}
                className="transition-all duration-500 ease-in"
              >
                {/* Apple */}
                <circle cx="0" cy="0" r="4.5" fill="#ef4444" filter="url(#neon-glow)" />
                {/* Stem */}
                <path d="M 0 -4 Q 1.5 -7 3 -6" stroke="#15803d" strokeWidth="1" fill="none" />
                {/* Leaf */}
                <ellipse cx="2" cy="-6" rx="2" ry="1" fill="#22c55e" transform="rotate(-20 2 -6)" />
              </g>

              {/* Newton Character */}
              <g
                className={`transition-all duration-700 ${
                  newtonState === "hop_dissolve"
                    ? "opacity-20 translate-y-3 scale-90"
                    : "opacity-100 translate-y-0"
                }`}
              >
                {/* Newton Body */}
                <g transform="translate(132, 126)">
                  {/* Coat / Torso */}
                  <path
                    d="M 10 14 C 7 17, 4 24, 6 28 L 19 28 C 21 24, 18 17, 15 14 Z"
                    fill="#312e81"
                  />
                  {/* Head & 17th-century Powdered Wig */}
                  <circle cx="12" cy="7" r="5" fill="#fde047" opacity="0.9" />
                  {/* White curls */}
                  <circle cx="8" cy="6" r="3" fill="#e2e8f0" />
                  <circle cx="16" cy="6" r="3" fill="#e2e8f0" />
                  <circle cx="12" cy="3" r="3.5" fill="#f1f5f9" />
                  {/* Legs */}
                  <line x1="9" y1="28" x2="7" y2="35" stroke="#1e1b4b" strokeWidth="2" />
                  <line x1="16" y1="28" x2="18" y2="35" stroke="#1e1b4b" strokeWidth="2" />

                  {/* Eureka Lightbulb */}
                  {lightbulbVisible && (
                    <g transform="translate(12, -7)" className="animate-bounce">
                      <circle cx="0" cy="0" r="4" fill="#fbbf24" filter="url(#neon-glow)" />
                      <line x1="0" y1="4" x2="0" y2="6" stroke="#78350f" strokeWidth="1" />
                      <text x="0" y="2" textAnchor="middle" fontSize="6" fill="#78350f">
                        💡
                      </text>
                    </g>
                  )}
                </g>
              </g>
            </svg>
          </div>

          {/* Footer Info Pill */}
          <div className="flex items-center justify-between text-[9px] font-mono text-zinc-500 pt-0.5 border-t border-white/5">
            <span className="text-zinc-400">
              {hoveredBranch
                ? `Module: ${treeData.branches.find((b) => b.id === hoveredBranch)?.name}`
                : treeData.activeBranchId
                ? `Active: ${treeData.activeBranchId}`
                : "Idle Tree"}
            </span>
            <span>{treeData.totalFiles} files</span>
          </div>
        </div>
      )}
    </div>
  );
}
