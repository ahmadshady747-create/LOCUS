import React, { useEffect, useState } from "react";
import type { CreateSkillRequest, SkillDto, SkillExecutionResultDto } from "../types";
import { skills as skillsApi } from "../lib/api";
import { sounds } from "../lib/sound";

export default function SkillsManager() {
  const [skillsList, setSkillsList] = useState<SkillDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [runtimeFilter, setRuntimeFilter] = useState<"all" | "wasm" | "script">("all");
  const [locationFilter, setLocationFilter] = useState<"all" | "workspace" | "global">("all");

  // Test Run Modal state
  const [testingSkill, setTestingSkill] = useState<SkillDto | null>(null);
  const [testInputJson, setTestInputJson] = useState("{}");
  const [runningTest, setRunningTest] = useState(false);
  const [testResult, setTestResult] = useState<SkillExecutionResultDto | null>(null);

  // Create Skill Modal state
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newSkillId, setNewSkillId] = useState("");
  const [newSkillName, setNewSkillName] = useState("");
  const [newSkillRuntime, setNewSkillRuntime] = useState<"script" | "wasm">("script");
  const [newSkillLang, setNewSkillLang] = useState("python");
  const [newSkillDesc, setNewSkillDesc] = useState("");
  const [newSkillInWorkspace, setNewSkillInWorkspace] = useState(true);

  const loadSkills = async () => {
    setLoading(true);
    try {
      const list = await skillsApi.list();
      setSkillsList(list);
    } catch (e) {
      console.error("Failed to load skills", e);
    } finally {
      setLoading(false);
    }
  };

  const handleRescan = async () => {
    sounds.playClick();
    setLoading(true);
    try {
      const list = await skillsApi.rescan();
      setSkillsList(list);
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to rescan skills", e);
    } finally {
      setLoading(false);
    }
  };

  const handleToggle = async (skill: SkillDto) => {
    sounds.playClick();
    const nextState = !skill.enabled;
    try {
      await skillsApi.toggle(skill.id, nextState);
      setSkillsList((prev) =>
        prev.map((s) => (s.id === skill.id ? { ...s, enabled: nextState } : s))
      );
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to toggle skill", e);
    }
  };

  const openTestModal = (skill: SkillDto) => {
    sounds.playClick();
    setTestingSkill(skill);
    setTestResult(null);

    // Generate sample JSON input from schema properties
    const sample: Record<string, any> = {};
    if (skill.parameters?.properties) {
      for (const [key, prop] of Object.entries<any>(skill.parameters.properties)) {
        if (prop.type === "string") sample[key] = "sample_value";
        else if (prop.type === "integer" || prop.type === "number") sample[key] = 10;
        else if (prop.type === "boolean") sample[key] = true;
        else if (prop.type === "array") sample[key] = [];
        else sample[key] = {};
      }
    } else {
      sample["query"] = "test query";
    }

    setTestInputJson(JSON.stringify(sample, null, 2));
  };

  const handleRunTest = async () => {
    if (!testingSkill) return;
    sounds.playClick();
    setRunningTest(true);
    setTestResult(null);

    try {
      let parsedArgs = {};
      try {
        parsedArgs = JSON.parse(testInputJson);
      } catch {
        parsedArgs = {};
      }

      const res = await skillsApi.execute(testingSkill.id, parsedArgs);
      setTestResult(res);
      if (res.success) {
        sounds.playSuccess();
      } else {
        sounds.playReceive();
      }
    } catch (e: any) {
      setTestResult({
        success: false,
        stdout: "",
        stderr: String(e),
        parsed_json: null,
        exit_code: -1,
        duration_ms: 0,
        is_timeout: false,
        error: String(e),
      });
    } finally {
      setRunningTest(false);
    }
  };

  const handleCreateSkill = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newSkillId.trim()) return;

    sounds.playClick();
    setCreating(true);

    try {
      const req: CreateSkillRequest = {
        id: newSkillId.trim(),
        name: newSkillName.trim() || newSkillId.trim(),
        runtime: newSkillRuntime,
        language: newSkillLang,
        description: newSkillDesc.trim() || "Custom LOCUS agent skill capability",
        target_in_workspace: newSkillInWorkspace,
      };

      const created = await skillsApi.create(req);
      sounds.playSuccess();
      setSkillsList((prev) => [created, ...prev]);
      setCreateModalOpen(false);

      // Reset form
      setNewSkillId("");
      setNewSkillName("");
      setNewSkillDesc("");
    } catch (err) {
      console.error("Failed to create skill", err);
    } finally {
      setCreating(false);
    }
  };

  useEffect(() => {
    loadSkills();
  }, []);

  const filteredSkills = skillsList.filter((s) => {
    const matchesSearch =
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.description.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesRuntime =
      runtimeFilter === "all" || s.runtime === runtimeFilter;

    const matchesLocation =
      locationFilter === "all" || s.location_type === locationFilter;

    return matchesSearch && matchesRuntime && matchesLocation;
  });

  const enabledCount = skillsList.filter((s) => s.enabled).length;

  return (
    <div className="space-y-4">
      {/* Top Header Controls */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl bg-black/40 border border-white/10">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-bold text-white flex items-center gap-2">
              <span>⚡</span> Installed Skills & Tool Calling
            </h3>
            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-violet-500/15 text-violet-300 border border-violet-500/30 font-semibold">
              {enabledCount} Active / {skillsList.length} Total
            </span>
          </div>
          <p className="text-xs text-locus-muted mt-0.5">
            Sandboxed WebAssembly and Subprocess script capabilities discovered in <code className="text-violet-300 font-mono text-[11px]">.locus/skills/</code> and <code className="text-violet-300 font-mono text-[11px]">~/.locus/skills/</code>
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleRescan}
            disabled={loading}
            className="btn-secondary py-1.5 px-3 text-xs font-mono"
            title="Scan folders for updated skill manifests"
          >
            {loading ? "Scanning…" : "↻ Rescan"}
          </button>
          <button
            onClick={() => {
              sounds.playClick();
              setCreateModalOpen(true);
            }}
            className="btn-primary py-1.5 px-3 text-xs font-semibold shadow-glow-violet"
          >
            + Create New Skill
          </button>
        </div>
      </div>

      {/* Filter & Search Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2.5">
        <div className="flex-1 min-w-[200px]">
          <input
            type="text"
            placeholder="Search skills by name, ID, or description…"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input-compact font-mono"
          />
        </div>

        <div className="flex items-center gap-2 text-xs font-mono">
          <select
            value={runtimeFilter}
            onChange={(e) => setRuntimeFilter(e.target.value as any)}
            className="input-compact py-1 px-2"
          >
            <option value="all">All Runtimes</option>
            <option value="wasm">WASM Sandbox</option>
            <option value="script">Script Subprocess</option>
          </select>

          <select
            value={locationFilter}
            onChange={(e) => setLocationFilter(e.target.value as any)}
            className="input-compact py-1 px-2"
          >
            <option value="all">All Locations</option>
            <option value="workspace">Workspace (.locus)</option>
            <option value="global">Global (~/.locus)</option>
          </select>
        </div>
      </div>

      {/* Skills Grid */}
      {filteredSkills.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {filteredSkills.map((skill) => {
            const isWasm = skill.runtime === "wasm";
            return (
              <div
                key={skill.id}
                className={`p-4 rounded-xl border transition-all relative flex flex-col justify-between ${
                  skill.enabled
                    ? "bg-[#0b0e17] border-white/10 hover:border-violet-500/40 hover:shadow-glow-violet"
                    : "bg-[#080a10] border-white/5 opacity-60"
                }`}
              >
                <div>
                  {/* Top card header */}
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-white tracking-wide">
                          {skill.name}
                        </span>
                        <span className="text-[10px] font-mono text-zinc-500">
                          v{skill.version}
                        </span>
                      </div>
                      <span className="text-[10px] font-mono text-violet-400 block mt-0.5">
                        id: {skill.id}
                      </span>
                    </div>

                    {/* Enable Toggle */}
                    <button
                      onClick={() => handleToggle(skill)}
                      className={`px-2.5 py-0.5 rounded-full text-[10px] font-mono font-bold transition-all border ${
                        skill.enabled
                          ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30 shadow-sm"
                          : "bg-white/5 text-zinc-500 border-white/10"
                      }`}
                    >
                      {skill.enabled ? "● Enabled" : "○ Disabled"}
                    </button>
                  </div>

                  <p className="text-xs text-zinc-300 leading-relaxed mb-3">
                    {skill.description}
                  </p>

                  {/* Badges */}
                  <div className="flex flex-wrap items-center gap-1.5 mb-3 font-mono text-[10px]">
                    <span
                      className={`px-2 py-0.5 rounded border font-semibold ${
                        isWasm
                          ? "bg-violet-500/15 text-violet-300 border-violet-500/30"
                          : "bg-blue-500/15 text-blue-300 border-blue-500/30"
                      }`}
                    >
                      {isWasm ? "⚡ WASM" : "📜 Script"}
                    </span>

                    <span className="px-2 py-0.5 rounded bg-white/5 text-zinc-400 border border-white/5">
                      📁 {skill.location_type}
                    </span>

                    <span className="px-2 py-0.5 rounded bg-white/5 text-zinc-400 border border-white/5">
                      ⏱️ {skill.timeout_seconds}s timeout
                    </span>

                    {skill.permissions.allow_network && (
                      <span className="px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-300 border border-amber-500/20">
                        🌐 Network
                      </span>
                    )}

                    {skill.permissions.allow_fs_read && (
                      <span className="px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                        🛡️ Read FS
                      </span>
                    )}

                    {skill.permissions.allow_fs_write && (
                      <span className="px-1.5 py-0.5 rounded bg-rose-500/10 text-rose-300 border border-rose-500/20">
                        💾 Write FS
                      </span>
                    )}
                  </div>
                </div>

                {/* Card footer action */}
                <div className="pt-2 border-t border-white/5 flex items-center justify-between text-xs font-mono">
                  <span className="text-[10px] text-zinc-500 truncate max-w-[200px]" title={skill.entrypoint}>
                    📄 {skill.entrypoint}
                  </span>

                  <button
                    onClick={() => openTestModal(skill)}
                    className="btn-secondary py-1 px-2.5 text-[11px] text-violet-300 hover:text-white"
                  >
                    ▶️ Test Run
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="p-8 rounded-xl bg-black/20 border border-dashed border-white/10 text-center space-y-2">
          <span className="text-2xl block">🧩</span>
          <p className="text-xs text-zinc-400">
            {searchQuery
              ? "No skills matching your search filter."
              : "No custom skills discovered yet. Click '+ Create New Skill' to scaffold your first tool."}
          </p>
        </div>
      )}

      {/* ========================================================================= */}
      {/* TEST RUN MODAL */}
      {/* ========================================================================= */}
      {testingSkill && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md select-none animate-fade-in">
          <div className="relative w-full max-w-xl bg-[#0c0e17] border border-violet-500/40 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
            <div className="px-5 py-3.5 bg-gradient-to-r from-violet-900/20 to-transparent border-b border-white/10 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-sm font-bold text-white">▶️ Test Skill:</span>
                <span className="font-mono text-xs text-violet-300 font-bold bg-white/5 px-2 py-0.5 rounded border border-white/10">
                  {testingSkill.name} ({testingSkill.id})
                </span>
              </div>
              <button
                onClick={() => setTestingSkill(null)}
                className="text-xs text-zinc-400 hover:text-white font-mono"
              >
                ✕ Close
              </button>
            </div>

            <div className="p-5 overflow-y-auto space-y-4 text-xs font-mono flex-1">
              <div>
                <label className="text-zinc-300 font-bold block mb-1.5">
                  Input JSON Arguments (Schema Validated):
                </label>
                <textarea
                  rows={5}
                  value={testInputJson}
                  onChange={(e) => setTestInputJson(e.target.value)}
                  className="input-dark font-mono text-xs p-3 leading-relaxed w-full bg-[#06080e]"
                />
              </div>

              {testResult && (
                <div
                  className={`p-3.5 rounded-xl border space-y-2 ${
                    testResult.success
                      ? "bg-emerald-950/20 border-emerald-500/30"
                      : testResult.is_timeout
                      ? "bg-amber-950/20 border-amber-500/30"
                      : "bg-rose-950/20 border-rose-500/30"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span
                      className={`font-bold ${
                        testResult.success
                          ? "text-emerald-400"
                          : testResult.is_timeout
                          ? "text-amber-400"
                          : "text-rose-400"
                      }`}
                    >
                      {testResult.success
                        ? "✓ Execution Succeeded"
                        : testResult.is_timeout
                        ? "⚠️ Execution Timed Out"
                        : "✕ Execution Failed"}
                    </span>
                    <span className="text-zinc-400 text-[11px]">
                      ⏱️ {testResult.duration_ms}ms
                    </span>
                  </div>

                  {testResult.error && (
                    <div className="text-rose-300 bg-rose-500/10 p-2 rounded border border-rose-500/20 whitespace-pre-wrap">
                      {testResult.error}
                    </div>
                  )}

                  {testResult.stdout && (
                    <div>
                      <span className="text-[10px] text-zinc-500 uppercase block mb-1">
                        stdout
                      </span>
                      <pre className="p-2.5 rounded bg-black/60 text-zinc-200 overflow-x-auto text-[11px] leading-relaxed">
                        {testResult.stdout}
                      </pre>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="px-5 py-3.5 bg-[#090b12] border-t border-white/10 flex items-center justify-between">
              <button
                onClick={() => setTestingSkill(null)}
                className="btn-secondary py-1.5 px-3 text-xs"
              >
                Cancel
              </button>
              <button
                onClick={handleRunTest}
                disabled={runningTest}
                className="btn-primary py-1.5 px-4 text-xs font-semibold"
              >
                {runningTest ? "Executing…" : "Execute Test"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ========================================================================= */}
      {/* CREATE NEW SKILL MODAL */}
      {/* ========================================================================= */}
      {createModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md select-none animate-fade-in">
          <div className="relative w-full max-w-lg bg-[#0c0e17] border border-violet-500/40 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
            <div className="px-5 py-3.5 bg-gradient-to-r from-violet-900/20 to-transparent border-b border-white/10 flex items-center justify-between">
              <span className="text-sm font-bold text-white">
                ✨ Create New LOCUS Skill
              </span>
              <button
                onClick={() => setCreateModalOpen(false)}
                className="text-xs text-zinc-400 hover:text-white font-mono"
              >
                ✕
              </button>
            </div>

            <form onSubmit={handleCreateSkill} className="p-5 overflow-y-auto space-y-3.5 text-xs font-mono flex-1">
              <div>
                <label className="text-zinc-300 font-bold block mb-1">Skill Identifier (id):</label>
                <input
                  type="text"
                  placeholder="e.g. git_commit_generator"
                  required
                  value={newSkillId}
                  onChange={(e) => setNewSkillId(e.target.value)}
                  className="input-compact"
                />
              </div>

              <div>
                <label className="text-zinc-300 font-bold block mb-1">Display Name:</label>
                <input
                  type="text"
                  placeholder="e.g. Git Commit Generator"
                  value={newSkillName}
                  onChange={(e) => setNewSkillName(e.target.value)}
                  className="input-compact"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-zinc-300 font-bold block mb-1">Target Runtime:</label>
                  <select
                    value={newSkillRuntime}
                    onChange={(e) => setNewSkillRuntime(e.target.value as any)}
                    className="input-compact w-full"
                  >
                    <option value="script">Script (Subprocess)</option>
                    <option value="wasm">WebAssembly (WASM)</option>
                  </select>
                </div>

                <div>
                  <label className="text-zinc-300 font-bold block mb-1">Language Template:</label>
                  <select
                    value={newSkillLang}
                    onChange={(e) => setNewSkillLang(e.target.value)}
                    className="input-compact w-full"
                  >
                    <option value="python">Python (main.py)</option>
                    <option value="javascript">JavaScript / Node (index.js)</option>
                    <option value="powershell">PowerShell (script.ps1)</option>
                    <option value="shell">Shell (run.sh)</option>
                    <option value="wasm">Rust / WASM (plugin.wasm)</option>
                  </select>
                </div>
              </div>

              <div>
                <label className="text-zinc-300 font-bold block mb-1">Description:</label>
                <textarea
                  rows={2}
                  placeholder="Explain what this tool does and how the agent should use it…"
                  value={newSkillDesc}
                  onChange={(e) => setNewSkillDesc(e.target.value)}
                  className="input-dark text-xs p-2"
                />
              </div>

              <div className="pt-2 border-t border-white/5">
                <label className="flex items-center gap-2 cursor-pointer text-zinc-300 text-xs">
                  <input
                    type="checkbox"
                    checked={newSkillInWorkspace}
                    onChange={(e) => setNewSkillInWorkspace(e.target.checked)}
                    className="rounded bg-black border-white/20 text-locus-violet"
                  />
                  <span>Save inside project workspace (<code className="text-violet-300">.locus/skills/</code>)</span>
                </label>
              </div>

              <div className="px-0 pt-3 border-t border-white/10 flex items-center justify-between">
                <button
                  type="button"
                  onClick={() => setCreateModalOpen(false)}
                  className="btn-secondary py-1.5 px-3"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={creating || !newSkillId.trim()}
                  className="btn-primary py-1.5 px-4 font-semibold"
                >
                  {creating ? "Scaffolding…" : "Scaffold & Register Skill"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
