import { useEffect, useState } from "react";
import type {
  AppState,
  BudgetStrategy,
  QaReport,
  SpecAlignmentReport,
  TaskGraph,
  TaskNode,
} from "../types";
import { adversarialQa, cognitiveRouter, specAligner, taskGraph } from "../lib/api";
import { generatePlaygroundDemo } from "../lib/playground";
import { sounds } from "../lib/sound";
import Chat from "./Chat";

interface MissionControlProps {
  state: AppState;
  onNavigate?: (tab: string) => void;
}

export default function MissionControl({ state }: MissionControlProps) {
  const [goal, setGoal] = useState("");
  const [graph, setGraph] = useState<TaskGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [runningAll, setRunningAll] = useState(false);
  const [activeView, setActiveView] = useState<"dag" | "chat">("dag");
  const [editingNodeId, setEditingNodeId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editDesc, setEditDesc] = useState("");

  // Cognitive Router & Budget Strategy State
  const [budgetStrategy, setBudgetStrategy] = useState<BudgetStrategy>("balanced");

  // Specification Alignment & Tradeoff Gate state
  const [tradeoffReport, setTradeoffReport] = useState<SpecAlignmentReport | null>(null);
  const [analyzingTradeoffs, setAnalyzingTradeoffs] = useState(false);
  const [selectedTradeoffs, setSelectedTradeoffs] = useState<Record<string, string>>({});

  // Adversarial QA evaluation cache by node ID
  const [qaReports, setQaReports] = useState<Record<string, QaReport>>({});
  const [activeQaInspectNode, setActiveQaInspectNode] = useState<TaskNode | null>(null);

  useEffect(() => {
    cognitiveRouter
      .getStrategy()
      .then((st) => setBudgetStrategy(st))
      .catch(() => {});
  }, []);

  const handleStrategyChange = async (st: BudgetStrategy) => {
    sounds.playClick();
    setBudgetStrategy(st);
    try {
      await cognitiveRouter.setStrategy(st);
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to save budget strategy", e);
    }
  };

  const handleDecompose = async (overrideGoal?: string) => {
    const targetGoal = overrideGoal ?? goal;
    if (!targetGoal.trim() || loading) return;
    sounds.playClick();
    setLoading(true);

    try {
      // 1. Analyze goal for architectural ambiguities
      if (!tradeoffReport) {
        setAnalyzingTradeoffs(true);
        const report = await specAligner.analyze(targetGoal);
        setAnalyzingTradeoffs(false);

        if (report.has_ambiguity) {
          setTradeoffReport(report);
          // Initialize selections with recommended options
          const initialSel: Record<string, string> = {};
          for (const amb of report.ambiguities) {
            const rec = amb.options.find((o) => o.recommended) || amb.options[0];
            if (rec) initialSel[amb.id] = rec.id;
          }
          setSelectedTradeoffs(initialSel);
          sounds.playSuccess();
          setLoading(false);
          return;
        }
      }

      // 2. Decompose into DAG
      const workspaceFiles = state.workspace ? Object.keys(state.workspace.files) : [];
      const newGraph = await taskGraph.decompose(targetGoal, workspaceFiles);
      setGraph(newGraph);
      setTradeoffReport(null);
      sounds.playSuccess();
    } catch (err: any) {
      console.error("Failed to decompose goal:", err);
    } finally {
      setLoading(false);
      setAnalyzingTradeoffs(false);
    }
  };

  const handleQuickDecompose = async () => {
    if (!goal.trim() || loading) return;
    sounds.playClick();
    setLoading(true);

    try {
      const workspaceFiles = state.workspace ? Object.keys(state.workspace.files) : [];
      const newGraph = await taskGraph.decompose(goal, workspaceFiles);
      setGraph(newGraph);
      setTradeoffReport(null);
      sounds.playSuccess();
    } catch (err) {
      console.error("Failed quick decompose:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleRunStep = async (nodeId: string) => {
    if (!graph) return;
    sounds.playClick();

    try {
      const updated = await taskGraph.executeNode(graph, nodeId);
      setGraph(updated);

      // Trigger Adversarial QA on executed node payload/diff
      const executedNode = updated.nodes.find((n) => n.id === nodeId);
      if (executedNode?.result?.diff_preview) {
        try {
          const report = await adversarialQa.evaluate(
            executedNode.result.diff_preview,
            "rust"
          );
          setQaReports((prev) => ({ ...prev, [nodeId]: report }));
        } catch {
          // ignore
        }
      }

      sounds.playSuccess();
    } catch (err: any) {
      console.error("Failed to execute node:", err);
    }
  };

  const handleRunAll = async () => {
    if (!graph || runningAll) return;
    sounds.playClick();
    setRunningAll(true);

    try {
      let currentGraph = graph;
      // Get topological execution order
      const order = await taskGraph.validate(currentGraph);

      for (const nodeId of order) {
        const node = currentGraph.nodes.find((n) => n.id === nodeId);
        if (node && node.status !== "completed" && node.status !== "skipped") {
          currentGraph = await taskGraph.executeNode(currentGraph, nodeId);
          setGraph({ ...currentGraph });

          const executedNode = currentGraph.nodes.find((n) => n.id === nodeId);
          if (executedNode?.result?.diff_preview) {
            try {
              const report = await adversarialQa.evaluate(
                executedNode.result.diff_preview,
                "rust"
              );
              setQaReports((prev) => ({ ...prev, [nodeId]: report }));
            } catch {
              // ignore
            }
          }

          if (executedNode?.status === "failed") {
            sounds.playClick();
            break;
          }
        }
      }
      sounds.playSuccess();
    } catch (err: any) {
      console.error("Autonomous execution error:", err);
      sounds.playClick();
    } finally {
      setRunningAll(false);
    }
  };

  const handleSaveEdit = async (nodeId: string) => {
    if (!graph) return;
    sounds.playClick();

    try {
      const updated = await taskGraph.updateNode(
        graph,
        nodeId,
        editTitle,
        editDesc
      );
      setGraph(updated);
      setEditingNodeId(null);
    } catch (err) {
      console.error("Failed to update node:", err);
    }
  };

  const handleSkipStep = async (nodeId: string) => {
    if (!graph) return;
    sounds.playClick();

    try {
      const updated = await taskGraph.updateNode(
        graph,
        nodeId,
        undefined,
        undefined,
        undefined,
        "skipped"
      );
      setGraph(updated);
    } catch (err) {
      console.error("Failed to skip node:", err);
    }
  };

  const handleLoadPlayground = () => {
    sounds.playClick();
    const demo = generatePlaygroundDemo();
    setGoal(demo.goal);
    setGraph(demo.graph);
    setQaReports(demo.qaReports);
    setTradeoffReport(demo.specReport);
    sounds.playSuccess();
  };

  const completedCount = graph?.nodes.filter((n) => n.status === "completed" || n.status === "skipped").length ?? 0;
  const totalCount = graph?.nodes.length ?? 0;
  const progressPercent = totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0;

  return (
    <div className="flex flex-col h-full bg-[#08090D] text-zinc-100 font-sans overflow-hidden">
      {/* Top Mission Control Bar */}
      <div className="border-b border-locus-border/80 bg-[#0A0C13] px-4 py-3 shrink-0 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-xs font-bold uppercase tracking-wider text-white flex items-center gap-2">
              <span className="text-violet-400">🎯</span> Mission Control
            </h1>
            <div className="flex items-center bg-black/40 p-0.5 rounded-lg border border-white/10 text-[11px] font-mono">
              <button
                onClick={() => {
                  sounds.playClick();
                  setActiveView("dag");
                }}
                className={`px-3 py-1 rounded-md transition-colors ${
                  activeView === "dag"
                    ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                    : "text-zinc-400 hover:text-white"
                }`}
              >
                🗺️ Task DAG Plan
              </button>
              <button
                onClick={() => {
                  sounds.playClick();
                  setActiveView("chat");
                }}
                className={`px-3 py-1 rounded-md transition-colors ${
                  activeView === "chat"
                    ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                    : "text-zinc-400 hover:text-white"
                }`}
              >
                💬 Interactive Assistant
              </button>
            </div>
            <button
              onClick={handleLoadPlayground}
              className="btn-spring px-3 py-1 bg-violet-600/20 hover:bg-violet-600/30 text-violet-300 border border-violet-500/40 rounded-lg text-xs font-mono font-semibold flex items-center gap-1.5 transition-all shadow-sm"
              title="Instantly generate an in-memory working demo with DAG, Living Tree, and QA reports"
            >
              <span>⚡ Playground Demo</span>
            </button>

            {/* Cognitive Budget Strategy Selector */}
            <div className="flex items-center bg-black/40 p-0.5 rounded-lg border border-white/10 text-[11px] font-mono">
              <button
                onClick={() => handleStrategyChange("max_speed")}
                className={`px-2.5 py-1 rounded-md transition-all ${
                  budgetStrategy === "max_speed"
                    ? "bg-amber-600/30 text-amber-300 font-semibold border border-amber-500/40"
                    : "text-zinc-400 hover:text-white"
                }`}
                title="Max Speed: Prioritize free local/Groq models with ultra-fast sub-second latency"
              >
                ⚡ Speed
              </button>
              <button
                onClick={() => handleStrategyChange("balanced")}
                className={`px-2.5 py-1 rounded-md transition-all ${
                  budgetStrategy === "balanced"
                    ? "bg-emerald-600/30 text-emerald-300 font-semibold border border-emerald-500/40"
                    : "text-zinc-400 hover:text-white"
                }`}
                title="Balanced: Save 75%+ tokens by routing Micro to free tier and Architectural to Reasoners"
              >
                ⚖️ Balanced
              </button>
              <button
                onClick={() => handleStrategyChange("max_power")}
                className={`px-2.5 py-1 rounded-md transition-all ${
                  budgetStrategy === "max_power"
                    ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                    : "text-zinc-400 hover:text-white"
                }`}
                title="Max Power: Direct all standard & architectural tasks to top frontier reasoning models"
              >
                🔥 Max Power
              </button>
            </div>
          </div>

          {/* Overall Stats */}
          {graph && (
            <div className="flex items-center gap-3 text-[11px] font-mono">
              <div className="flex items-center gap-1.5 bg-black/30 px-2.5 py-1 rounded-md border border-white/10">
                <span className="text-zinc-400">Progress:</span>
                <span className="text-violet-300 font-bold">
                  {completedCount}/{totalCount} ({progressPercent}%)
                </span>
              </div>
              <div className="flex items-center gap-1.5 bg-black/30 px-2.5 py-1 rounded-md border border-white/10">
                <span className="text-zinc-400">Status:</span>
                <span
                  className={`font-semibold uppercase text-[10px] ${
                    graph.status === "completed"
                      ? "text-emerald-400"
                      : graph.status === "failed"
                      ? "text-red-400"
                      : graph.status === "in_progress"
                      ? "text-amber-400"
                      : "text-zinc-400"
                  }`}
                >
                  {graph.status}
                </span>
              </div>
            </div>
          )}
        </div>

        {/* Goal Input Field */}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleDecompose();
          }}
          className="flex items-center gap-2"
        >
          <div className="relative flex-1">
            <span className="absolute left-3.5 top-2.5 text-zinc-500 font-mono text-xs">🎯</span>
            <input
              type="text"
              value={goal}
              onChange={(e) => {
                setGoal(e.target.value);
                if (tradeoffReport) setTradeoffReport(null);
              }}
              placeholder="Define high-level objective (e.g. 'Add Redis cache layer to auth controller and write unit tests')..."
              className="w-full bg-black/40 border border-white/15 focus:border-violet-500/80 rounded-xl pl-9 pr-4 py-2 text-xs text-white placeholder-zinc-500 font-mono focus:outline-none focus:ring-1 focus:ring-violet-500/40 transition-all"
            />
          </div>
          <button
            type="submit"
            disabled={!goal.trim() || loading || analyzingTradeoffs}
            className="px-4 py-2 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 disabled:opacity-40 text-white text-xs font-semibold rounded-xl transition-all shadow-glow-violet flex items-center gap-1.5 shrink-0"
          >
            {analyzingTradeoffs ? "⚖️ Analyzing..." : loading ? "⚡ Planning..." : "⚡ Decompose Goal"}
          </button>
        </form>

        {/* Specification Alignment & Tradeoff Gate Alert Card */}
        {tradeoffReport && tradeoffReport.has_ambiguity && (
          <div className="p-4 rounded-xl bg-[#0e111d] border border-violet-500/40 space-y-3 shadow-2xl animate-fade-in">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-amber-400 font-bold text-sm">⚖️</span>
                <span className="text-xs font-bold text-white font-mono">
                  Specification Alignment Gate: Architectural Tradeoffs Detected
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleQuickDecompose}
                  className="px-3 py-1 bg-white/10 hover:bg-white/20 text-zinc-200 rounded-lg text-xs font-mono transition-colors"
                >
                  ⚡ Quick Decompose (Defaults)
                </button>
                <button
                  onClick={() => handleDecompose(goal)}
                  className="px-3 py-1 bg-violet-600 hover:bg-violet-500 text-white rounded-lg text-xs font-semibold font-mono shadow-sm transition-colors"
                >
                  ✓ Confirm & Decompose DAG
                </button>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
              {tradeoffReport.ambiguities.map((amb) => (
                <div key={amb.id} className="p-3 rounded-lg bg-black/40 border border-white/10 space-y-2">
                  <div className="text-[11px] font-mono text-zinc-300 font-bold flex items-center gap-1.5">
                    <span className="text-violet-400">●</span> {amb.question}
                  </div>
                  <div className="grid grid-cols-1 gap-2">
                    {amb.options.map((opt) => {
                      const isSelected = selectedTradeoffs[amb.id] === opt.id;
                      return (
                        <div
                          key={opt.id}
                          onClick={() => {
                            sounds.playClick();
                            setSelectedTradeoffs((prev) => ({ ...prev, [amb.id]: opt.id }));
                          }}
                          className={`p-2.5 rounded-lg border cursor-pointer transition-all text-xs ${
                            isSelected
                              ? "bg-violet-950/40 border-violet-500/80 shadow-glow-violet"
                              : "bg-black/20 border-white/5 hover:border-white/20"
                          }`}
                        >
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              <span className={`w-3 h-3 rounded-full border flex items-center justify-center ${isSelected ? "border-violet-400 bg-violet-500" : "border-zinc-600"}`}>
                                {isSelected && <span className="w-1.5 h-1.5 rounded-full bg-white" />}
                              </span>
                              <span className="font-semibold text-white font-mono">{opt.title}</span>
                            </div>
                            {opt.recommended && (
                              <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                                Recommended
                              </span>
                            )}
                          </div>
                          <p className="text-[10px] text-zinc-400 mt-1 pl-5">{opt.description}</p>
                          <div className="flex items-center gap-3 text-[9px] font-mono mt-1.5 pl-5">
                            <span className="text-emerald-400">✓ {opt.pros[0]}</span>
                            {opt.cons[0] && <span className="text-red-400">✗ {opt.cons[0]}</span>}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* View Content */}
      <div className="flex-1 overflow-hidden">
        {activeView === "chat" ? (
          <Chat state={state} />
        ) : (
          <div className="h-full flex flex-col p-4 overflow-y-auto space-y-4">
            {!graph ? (
              <div className="flex flex-col items-center justify-center h-72 border border-dashed border-white/10 rounded-2xl bg-black/20 text-center p-6 space-y-4 animate-spring-in">
                <div className="w-12 h-12 rounded-2xl bg-violet-600/20 border border-violet-500/30 flex items-center justify-center text-xl animate-neon-pulse">
                  🗺️
                </div>
                <div>
                  <h2 className="text-sm font-bold text-white">Autonomous Goal Decomposition</h2>
                  <p className="text-xs text-zinc-400 max-w-md mt-1">
                    Enter an objective above. The DAG engine will decompose your goal into
                    verifiable steps, dependency trees, and executable code edits.
                  </p>
                </div>
                <button
                  onClick={handleLoadPlayground}
                  className="btn-spring px-4 py-2 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white rounded-xl text-xs font-semibold font-mono shadow-glow-violet flex items-center gap-2"
                >
                  <span>⚡ Launch 3-Second Playground Demo</span>
                </button>
              </div>
            ) : (
              <div className="space-y-4">
                {/* Control Action Bar */}
                <div className="flex items-center justify-between bg-black/30 p-2.5 rounded-xl border border-white/10 shrink-0">
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleRunAll}
                      disabled={runningAll || graph.status === "completed"}
                      className="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors shadow-sm"
                    >
                      <span>{runningAll ? "⏳ Running..." : "▶️ Autonomous Run All"}</span>
                    </button>
                  </div>

                  <div className="flex items-center gap-2 text-xs font-mono text-zinc-400">
                    <span>{totalCount} Total Steps</span>
                  </div>
                </div>

                {/* Task Node Cards List */}
                <div className="space-y-3">
                  {graph.nodes.map((node, index) => {
                    const isEditing = editingNodeId === node.id;
                    const qa = qaReports[node.id];
                    return (
                      <div
                        key={node.id}
                        className={`rounded-xl border transition-all p-3.5 space-y-3 ${
                          node.status === "completed"
                            ? "bg-[#0c120e]/60 border-emerald-500/30"
                            : node.status === "running"
                            ? "bg-[#14120a]/80 border-amber-500/40 shadow-glow-amber"
                            : node.status === "ready"
                            ? "bg-[#0f111a]/90 border-violet-500/40"
                            : node.status === "failed"
                            ? "bg-[#140c0c]/80 border-red-500/40"
                            : "bg-black/30 border-white/10 opacity-75"
                        }`}
                      >
                        {/* Card Header */}
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex items-center gap-2.5">
                            <span className="w-6 h-6 rounded-lg bg-black/40 border border-white/10 flex items-center justify-center font-mono text-[10px] text-zinc-300 font-bold shrink-0">
                              {index + 1}
                            </span>

                            {isEditing ? (
                              <input
                                type="text"
                                value={editTitle}
                                onChange={(e) => setEditTitle(e.target.value)}
                                className="bg-black/60 border border-violet-500 rounded px-2 py-0.5 text-xs text-white font-mono"
                              />
                            ) : (
                              <div>
                                <h3 className="text-xs font-bold text-white flex flex-wrap items-center gap-2">
                                  <span>{node.title}</span>
                                  <span className="text-[9px] font-mono px-2 py-0.5 bg-white/5 border border-white/10 rounded uppercase text-zinc-400">
                                    {node.node_type}
                                  </span>

                                  {/* Cognitive Load & Model Routing Pill */}
                                  {(() => {
                                    const nt = node.node_type.toLowerCase();
                                    const desc = (node.title + " " + node.description).toLowerCase();
                                    let comp = "Standard";
                                    let model = budgetStrategy === "max_power" ? "Claude-3.5 Sonnet" : "Gemini-2.0-Flash";
                                    let badge = "text-blue-300 bg-blue-950/40 border-blue-500/30";
                                    let icon = "🧠";

                                    if (nt === "analysis" || desc.includes("fuzz") || desc.includes("adversarial") || desc.includes("tradeoff") || desc.includes("architect")) {
                                      comp = "Architect";
                                      model = budgetStrategy === "max_speed" ? "Llama-3.3-70B" : budgetStrategy === "max_power" ? "Claude-3.5 / DeepSeek-R1" : "DeepSeek-Reasoner";
                                      badge = "text-purple-300 bg-purple-950/40 border-purple-500/30";
                                      icon = "🏛️";
                                    } else if (nt === "shell_command" || desc.includes("commit") || desc.includes("format") || desc.includes("lint")) {
                                      comp = "Micro";
                                      model = "Groq LPU / Local (0-cost)";
                                      badge = "text-emerald-300 bg-emerald-950/40 border-emerald-500/30";
                                      icon = "⚡";
                                    }

                                    return (
                                      <span
                                        className={`text-[9px] font-mono px-2 py-0.5 rounded border flex items-center gap-1 ${badge}`}
                                        title={`Cognitive Task Complexity: ${comp} · Routed to: ${model}`}
                                      >
                                        <span>{icon}</span>
                                        <span>{comp}: {model}</span>
                                      </span>
                                    );
                                  })()}
                                </h3>
                              </div>
                            )}
                          </div>

                          <div className="flex items-center gap-2">
                            {/* Adversarial QA Robustness Pill */}
                            {qa && (
                              <button
                                onClick={() => setActiveQaInspectNode(node)}
                                className={`text-[10px] font-mono px-2 py-0.5 rounded flex items-center gap-1 border transition-all ${
                                  qa.is_approved
                                    ? "bg-emerald-500/10 text-emerald-300 border-emerald-500/30 hover:bg-emerald-500/20"
                                    : "bg-red-500/10 text-red-300 border-red-500/30 hover:bg-red-500/20"
                                }`}
                                title="Inspect Adversarial QA Report & Fuzz Cases"
                              >
                                <span>🛡️ QA: {qa.score}/100</span>
                              </button>
                            )}
                            {/* Status Badge */}
                            <span
                              className={`text-[10px] font-mono px-2.5 py-0.5 rounded-md font-semibold uppercase ${
                                node.status === "completed"
                                  ? "bg-emerald-500/15 text-emerald-300 border border-emerald-500/30"
                                  : node.status === "running"
                                  ? "bg-amber-500/15 text-amber-300 border border-amber-500/30 animate-pulse"
                                  : node.status === "ready"
                                  ? "bg-violet-500/15 text-violet-300 border border-violet-500/30"
                                  : node.status === "failed"
                                  ? "bg-red-500/15 text-red-300 border border-red-500/30"
                                  : "bg-zinc-800 text-zinc-400 border border-zinc-700"
                              }`}
                            >
                              {node.status}
                            </span>

                            {/* Actions */}
                            {node.status !== "completed" && node.status !== "skipped" && (
                              <button
                                onClick={() => handleRunStep(node.id)}
                                disabled={node.status === "running"}
                                className="px-2.5 py-1 bg-violet-600 hover:bg-violet-500 disabled:opacity-40 text-white rounded text-[11px] font-semibold flex items-center gap-1 transition-colors"
                              >
                                <span>▶️ Run</span>
                              </button>
                            )}

                            {isEditing ? (
                              <button
                                onClick={() => handleSaveEdit(node.id)}
                                className="px-2 py-1 bg-emerald-600 text-white rounded text-[10px]"
                              >
                                Save
                              </button>
                            ) : (
                              <button
                                onClick={() => {
                                  setEditingNodeId(node.id);
                                  setEditTitle(node.title);
                                  setEditDesc(node.description);
                                }}
                                className="p-1 rounded text-zinc-400 hover:text-white"
                                title="Edit Step"
                              >
                                ✏️
                              </button>
                            )}

                            {node.status !== "completed" && node.status !== "skipped" && (
                              <button
                                onClick={() => handleSkipStep(node.id)}
                                className="text-[10px] text-zinc-500 hover:text-zinc-300"
                                title="Skip Step"
                              >
                                Skip
                              </button>
                            )}
                          </div>
                        </div>

                        {/* Description */}
                        {isEditing ? (
                          <textarea
                            value={editDesc}
                            onChange={(e) => setEditDesc(e.target.value)}
                            className="w-full bg-black/60 border border-violet-500 rounded p-2 text-xs text-zinc-200 font-mono"
                            rows={2}
                          />
                        ) : (
                          <p className="text-xs text-zinc-400 font-mono leading-relaxed pl-8">
                            {node.description}
                          </p>
                        )}

                        {/* Dependency Tag list */}
                        {node.dependencies.length > 0 && (
                          <div className="flex items-center gap-1.5 pl-8 text-[10px] font-mono text-zinc-500">
                            <span>Depends on:</span>
                            {node.dependencies.map((dep) => (
                              <span
                                key={dep}
                                className="bg-white/5 px-2 py-0.5 rounded border border-white/10 text-zinc-300"
                              >
                                {dep}
                              </span>
                            ))}
                          </div>
                        )}

                        {/* Embedded Diff Preview if Code Modification */}
                        {node.payload.target_file && (
                          <div className="pl-8 pt-1">
                            <div className="bg-black/50 border border-white/10 rounded-lg p-2 font-mono text-[11px] space-y-1">
                              <div className="flex items-center justify-between text-zinc-400 text-[10px]">
                                <span>📄 Target File: {node.payload.target_file}</span>
                              </div>
                              {node.payload.search_replace_block && (
                                <pre className="text-zinc-300 max-h-24 overflow-y-auto text-[10px] bg-black/60 p-2 rounded">
                                  {node.payload.search_replace_block}
                                </pre>
                              )}
                            </div>
                          </div>
                        )}

                        {/* Execution Result Terminal Output */}
                        {node.result && (
                          <div className="pl-8 pt-1">
                            <div
                              className={`rounded-lg p-2 font-mono text-[10px] border ${
                                node.result.success
                                  ? "bg-black/40 border-emerald-500/20 text-zinc-300"
                                  : "bg-red-950/30 border-red-500/30 text-red-300"
                              }`}
                            >
                              <div className="flex items-center justify-between pb-1 border-b border-white/5 text-[9px] uppercase font-bold text-zinc-500">
                                <span>Output ({node.result.duration_ms}ms)</span>
                                <span>{node.result.success ? "✓ Passed" : "✕ Failed"}</span>
                              </div>
                              <pre className="whitespace-pre-wrap mt-1 max-h-24 overflow-y-auto">
                                {node.result.output || node.result.error}
                              </pre>
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Adversarial QA Inspect Modal */}
      {activeQaInspectNode && qaReports[activeQaInspectNode.id] && (
        <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-[#0b0e17] border border-violet-500/40 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden animate-fade-in">
            <div className="flex items-center justify-between p-4 border-b border-white/10 bg-[#0e1220]">
              <div className="flex items-center gap-2.5">
                <span className="text-xl">🛡️</span>
                <div>
                  <h2 className="text-sm font-bold text-white flex items-center gap-2">
                    <span>Adversarial QA Report</span>
                    <span
                      className={`text-[10px] font-mono px-2 py-0.5 rounded ${
                        qaReports[activeQaInspectNode.id].is_approved
                          ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                          : "bg-red-500/20 text-red-300 border border-red-500/30"
                      }`}
                    >
                      Score: {qaReports[activeQaInspectNode.id].score}/100
                    </span>
                  </h2>
                  <p className="text-[11px] text-zinc-400 font-mono mt-0.5">
                    Step {activeQaInspectNode.title} · {activeQaInspectNode.payload.target_file}
                  </p>
                </div>
              </div>
              <button
                onClick={() => setActiveQaInspectNode(null)}
                className="text-zinc-400 hover:text-white p-1 rounded-lg hover:bg-white/5"
              >
                ✕
              </button>
            </div>

            <div className="p-5 overflow-y-auto space-y-4 text-xs font-mono">
              {/* Summary */}
              <div className="p-3 rounded-xl bg-black/40 border border-white/10 text-zinc-300">
                {qaReports[activeQaInspectNode.id].summary}
              </div>

              {/* Detected Risks */}
              {qaReports[activeQaInspectNode.id].risks.length > 0 ? (
                <div className="space-y-2">
                  <h3 className="text-xs font-bold text-amber-400 uppercase tracking-wider">
                    Flagged Adversarial Risks ({qaReports[activeQaInspectNode.id].risks.length}):
                  </h3>
                  <div className="space-y-2">
                    {qaReports[activeQaInspectNode.id].risks.map((risk, idx) => (
                      <div key={idx} className="p-3 rounded-xl bg-[#140f0c] border border-red-500/30 space-y-1">
                        <div className="flex items-center justify-between">
                          <span className="font-bold text-red-300">{risk.rule}</span>
                          <span className="text-[9px] uppercase px-1.5 py-0.5 rounded bg-red-500/20 text-red-300">
                            {risk.severity}
                          </span>
                        </div>
                        <p className="text-zinc-400">{risk.description}</p>
                        <p className="text-emerald-300 text-[11px] pt-1">
                          <span className="font-bold">Suggested Fix:</span> {risk.suggested_fix}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              ) : (
                <div className="p-3 rounded-xl bg-emerald-950/20 border border-emerald-500/30 text-emerald-300">
                  ✓ Clean audit. No null dereferences, unhandled panics, or concurrency race conditions.
                </div>
              )}

              {/* Fuzz Boundary Test Cases */}
              <div className="space-y-2 pt-2">
                <h3 className="text-xs font-bold text-violet-400 uppercase tracking-wider">
                  Simulated Fuzz Boundary Cases:
                </h3>
                <div className="grid grid-cols-1 gap-2">
                  {qaReports[activeQaInspectNode.id].fuzz_cases.map((fc, idx) => (
                    <div key={idx} className="p-2.5 rounded-lg bg-black/30 border border-white/5 space-y-1 text-[11px]">
                      <div className="flex items-center justify-between text-zinc-300">
                        <span className="font-bold text-white">{fc.input_name}</span>
                        <span className="text-zinc-500">Payload: {fc.input_value}</span>
                      </div>
                      <p className="text-zinc-400 text-[10px]">Expected: {fc.expected_behavior}</p>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="p-3 border-t border-white/10 bg-[#0e1220] flex justify-end">
              <button
                onClick={() => setActiveQaInspectNode(null)}
                className="px-4 py-1.5 rounded-xl bg-violet-600 hover:bg-violet-500 text-white text-xs font-semibold font-mono"
              >
                Close Audit Report
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
