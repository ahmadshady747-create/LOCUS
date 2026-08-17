import { useEffect, useRef, useState } from "react";
import type {
  DeviceCodeResponse,
  GitHubAuthStatus,
  GitHubRepo,
  GitStatusReport,
  PullRequestResult,
  SmartCommitResult,
} from "../types";
import { gitSync, githubAuth } from "../lib/api";
import { sounds } from "../lib/sound";

interface GitHubSyncModalProps {
  isOpen: boolean;
  onClose: () => void;
  workspaceRoot?: string | null;
}

export default function GitHubSyncModal({
  isOpen,
  onClose,
  workspaceRoot = ".",
}: GitHubSyncModalProps) {
  const [activeTab, setActiveTab] = useState<"auth" | "clone" | "sync">("auth");

  // Auth State
  const [authStatus, setAuthStatus] = useState<GitHubAuthStatus | null>(null);
  const [loadingAuth, setLoadingAuth] = useState(false);
  const [deviceCode, setDeviceCode] = useState<DeviceCodeResponse | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [copiedCode, setCopiedCode] = useState(false);
  const pollTimerRef = useRef<number | null>(null);

  // Clone State
  const [repos, setRepos] = useState<GitHubRepo[]>([]);
  const [loadingRepos, setLoadingRepos] = useState(false);
  const [repoFilter, setRepoFilter] = useState("");
  const [customRepoUrl, setCustomRepoUrl] = useState("");
  const [targetCloneDir, setTargetCloneDir] = useState("");
  const [cloning, setCloning] = useState(false);
  const [cloneMsg, setCloneMsg] = useState<{ text: string; isError: boolean } | null>(null);

  // Git Sync State
  const [gitStatus, setGitStatus] = useState<GitStatusReport | null>(null);
  const [loadingGitStatus, setLoadingGitStatus] = useState(false);
  const [intentSummary, setIntentSummary] = useState("");
  const [autoPush, setAutoPush] = useState(true);
  const [committing, setCommitting] = useState(false);
  const [commitResult, setCommitResult] = useState<SmartCommitResult | null>(null);
  const [prTitle, setPrTitle] = useState("");
  const [prBody, setPrBody] = useState("");
  const [prBase, setPrBase] = useState("main");
  const [prHead, setPrHead] = useState("");
  const [creatingPr, setCreatingPr] = useState(false);
  const [prResult, setPrResult] = useState<PullRequestResult | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);

  const fetchAuthStatus = async () => {
    setLoadingAuth(true);
    try {
      const status = await githubAuth.getStatus();
      setAuthStatus(status);
      if (status.is_authenticated) {
        fetchRepos();
      }
    } catch (e) {
      console.error("Failed to check GitHub auth status", e);
    } finally {
      setLoadingAuth(false);
    }
  };

  const fetchRepos = async () => {
    setLoadingRepos(true);
    try {
      const list = await githubAuth.listRepos(1, 50);
      setRepos(list);
    } catch (e) {
      console.error("Failed to list repos", e);
    } finally {
      setLoadingRepos(false);
    }
  };

  const fetchGitStatus = async () => {
    setLoadingGitStatus(true);
    setSyncError(null);
    try {
      const st = await gitSync.getStatus(workspaceRoot || ".");
      setGitStatus(st);
      setPrHead(st.branch);
    } catch (e: any) {
      setSyncError(e?.toString() || "Failed to inspect git status");
    } finally {
      setLoadingGitStatus(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      fetchAuthStatus();
      fetchGitStatus();
    } else {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
      setIsPolling(false);
      setDeviceCode(null);
    }
  }, [isOpen, workspaceRoot]);

  // Start Device Flow
  const handleStartDeviceAuth = async () => {
    sounds.playClick();
    setDeviceCode(null);
    setIsPolling(true);
    try {
      const res = await githubAuth.requestDeviceCode("repo,user");
      setDeviceCode(res);

      const intervalSec = Math.max(res.interval || 5, 5);
      if (pollTimerRef.current) clearInterval(pollTimerRef.current);

      pollTimerRef.current = window.setInterval(async () => {
        try {
          const pollRes = await githubAuth.pollToken(res.device_code);
          if ("success" in pollRes) {
            clearInterval(pollTimerRef.current!);
            pollTimerRef.current = null;
            setIsPolling(false);
            setDeviceCode(null);
            sounds.playSuccess();
            fetchAuthStatus();
          } else if ("expired" in pollRes || "denied" in pollRes || "error" in pollRes) {
            clearInterval(pollTimerRef.current!);
            pollTimerRef.current = null;
            setIsPolling(false);
          }
        } catch (pollErr) {
          console.error("Poll error", pollErr);
        }
      }, intervalSec * 1000);
    } catch (err: any) {
      setIsPolling(false);
      console.error("Device auth request failed", err);
    }
  };

  const handleCopyCode = () => {
    if (!deviceCode) return;
    sounds.playClick();
    navigator.clipboard.writeText(deviceCode.user_code);
    setCopiedCode(true);
    setTimeout(() => setCopiedCode(false), 2500);
  };

  const handleLogout = async () => {
    sounds.playClick();
    try {
      await githubAuth.logout();
      setAuthStatus({
        is_authenticated: false,
        user: null,
        token_preview: null,
        error: null,
      });
      setRepos([]);
      sounds.playSuccess();
    } catch (e) {
      console.error("Failed to logout", e);
    }
  };

  // Clone Repo Handler
  const handleCloneRepo = async (url: string, defaultName?: string) => {
    const dir = targetCloneDir.trim() || `./${defaultName || "cloned-repo"}`;
    setCloning(true);
    setCloneMsg(null);
    sounds.playClick();
    try {
      const res = await gitSync.cloneRepo({
        repo_url: url,
        target_dir: dir,
      });
      setCloneMsg({ text: res, isError: false });
      sounds.playSuccess();
    } catch (e: any) {
      setCloneMsg({ text: e?.toString() || "Clone failed", isError: true });
    } finally {
      setCloning(false);
    }
  };

  // Smart Commit Handler
  const handleSmartCommit = async () => {
    setCommitting(true);
    setSyncError(null);
    sounds.playClick();
    try {
      const res = await gitSync.smartCommit(
        workspaceRoot || ".",
        intentSummary.trim() || undefined,
        autoPush
      );
      setCommitResult(res);
      sounds.playSuccess();
      fetchGitStatus();
    } catch (e: any) {
      setSyncError(e?.toString() || "Smart commit failed");
    } finally {
      setCommitting(false);
    }
  };

  // Create PR Handler
  const handleCreatePr = async () => {
    if (!prTitle.trim()) return;
    setCreatingPr(true);
    setSyncError(null);
    sounds.playClick();
    try {
      // Find repo owner and repo name from remote if possible
      const repoUrl = repos[0]?.full_name || "owner/repo";
      const [owner, name] = repoUrl.split("/");

      const res = await gitSync.createPullRequest({
        auth_token: "", // backend retrieves stored token
        repo_owner: owner || "owner",
        repo_name: name || "repo",
        title: prTitle,
        body: prBody || "Generated automatically via LOCUS Autonomous Agent.",
        base: prBase || "main",
        head: prHead || "main",
      });
      setPrResult(res);
      sounds.playSuccess();
    } catch (e: any) {
      setSyncError(e?.toString() || "Failed to create Pull Request");
    } finally {
      setCreatingPr(false);
    }
  };

  if (!isOpen) return null;

  const filteredRepos = repos.filter((r) =>
    r.full_name.toLowerCase().includes(repoFilter.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-[#0b0e17] border border-violet-500/40 rounded-2xl w-full max-w-3xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden animate-fade-in font-sans text-xs">
        {/* Modal Header */}
        <div className="flex items-center justify-between p-4 border-b border-white/10 bg-[#0e1220] shrink-0">
          <div className="flex items-center gap-2.5">
            <span className="text-xl">🐙</span>
            <div>
              <h2 className="text-sm font-bold text-white font-mono flex items-center gap-2">
                <span>Git & GitHub Device Flow</span>
                <span className="text-[9px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                  Zero-Server Direct OAuth
                </span>
              </h2>
              <p className="text-[11px] text-zinc-400 font-mono mt-0.5">
                Local-first repository synchronization, smart conventional commits, and direct PR orchestration.
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-zinc-400 hover:text-white p-1 rounded-lg hover:bg-white/5 transition-colors"
          >
            ✕
          </button>
        </div>

        {/* Navigation Sub-Tabs */}
        <div className="px-4 py-2 border-b border-white/5 bg-black/40 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-1.5 font-mono text-xs">
            <button
              onClick={() => {
                sounds.playClick();
                setActiveTab("auth");
              }}
              className={`px-3 py-1 rounded-lg transition-colors flex items-center gap-1.5 ${
                activeTab === "auth"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>🔐</span>
              <span>Account & Device Auth</span>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setActiveTab("clone");
              }}
              className={`px-3 py-1 rounded-lg transition-colors flex items-center gap-1.5 ${
                activeTab === "clone"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>📥</span>
              <span>Clone Repository</span>
            </button>

            <button
              onClick={() => {
                sounds.playClick();
                setActiveTab("sync");
                fetchGitStatus();
              }}
              className={`px-3 py-1 rounded-lg transition-colors flex items-center gap-1.5 ${
                activeTab === "sync"
                  ? "bg-violet-600/30 text-violet-300 font-semibold border border-violet-500/40"
                  : "text-zinc-400 hover:text-white"
              }`}
            >
              <span>🚀</span>
              <span>Smart Commit & PR</span>
            </button>
          </div>

          {authStatus?.is_authenticated && authStatus.user && (
            <div className="flex items-center gap-2 font-mono text-[11px] text-zinc-300">
              <img
                src={authStatus.user.avatar_url}
                alt={authStatus.user.login}
                className="w-5 h-5 rounded-full border border-violet-400/40"
              />
              <span className="font-bold text-white">@{authStatus.user.login}</span>
            </div>
          )}
        </div>

        {/* Tab Content Panes */}
        <div className="p-5 flex-1 overflow-y-auto space-y-4">
          {/* 1. Account & Device Auth Tab */}
          {activeTab === "auth" && (
            <div className="space-y-4 font-mono text-xs">
              {loadingAuth ? (
                <div className="text-center py-10 text-zinc-400 animate-pulse">
                  Checking GitHub authentication status…
                </div>
              ) : authStatus?.is_authenticated && authStatus.user ? (
                <div className="p-5 rounded-xl bg-[#0c120e] border border-emerald-500/30 space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <img
                        src={authStatus.user.avatar_url}
                        alt={authStatus.user.login}
                        className="w-12 h-12 rounded-xl border-2 border-emerald-500/40 shadow-glow-emerald"
                      />
                      <div>
                        <h3 className="text-sm font-bold text-white">
                          {authStatus.user.name || authStatus.user.login}
                        </h3>
                        <p className="text-[11px] text-zinc-400">@{authStatus.user.login}</p>
                        <p className="text-[10px] text-zinc-500">
                          {authStatus.user.public_repos} public repos · {authStatus.token_preview}
                        </p>
                      </div>
                    </div>

                    <button
                      onClick={handleLogout}
                      className="px-3 py-1.5 rounded-lg bg-red-600/20 hover:bg-red-600/30 text-red-300 border border-red-500/30 transition-colors"
                    >
                      🚪 Sign Out
                    </button>
                  </div>
                  <div className="p-3 rounded-lg bg-black/40 border border-white/5 text-[11px] text-zinc-300 space-y-1">
                    <div className="text-emerald-400 font-bold">✓ Ready for Push and Pull Requests</div>
                    <div className="text-zinc-400">
                      Your Personal Access Token is securely stored in local encrypted vault.
                    </div>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-2">
                    <h3 className="text-xs font-bold text-white uppercase tracking-wider flex items-center gap-1.5">
                      <span>🔑</span> GitHub Device Authorization Grant
                    </h3>
                    <p className="text-zinc-400 text-[11px]">
                      Authenticate directly with GitHub without exposing your password or requiring an external relay server.
                    </p>
                  </div>

                  {!deviceCode ? (
                    <div className="text-center py-6">
                      <button
                        onClick={handleStartDeviceAuth}
                        disabled={isPolling}
                        className="px-6 py-2.5 rounded-xl bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 text-white font-bold text-xs shadow-glow-violet transition-all"
                      >
                        {isPolling ? "Connecting…" : "🔐 Connect GitHub with Device Flow"}
                      </button>
                    </div>
                  ) : (
                    <div className="p-5 rounded-xl bg-[#0e1120] border border-violet-500/40 space-y-4 animate-fade-in">
                      <div className="text-center space-y-2">
                        <div className="text-zinc-400 text-[11px]">
                          Enter this code at GitHub activation page:
                        </div>
                        <div className="text-2xl font-black text-violet-300 tracking-widest bg-black/60 py-3 rounded-xl border border-violet-500/40 select-all">
                          {deviceCode.user_code}
                        </div>
                      </div>

                      <div className="flex items-center justify-center gap-3 pt-1">
                        <button
                          onClick={handleCopyCode}
                          className="px-4 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white text-xs transition-colors"
                        >
                          {copiedCode ? "✓ Copied Code" : "📋 Copy Code"}
                        </button>
                        <a
                          href={deviceCode.verification_uri}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="px-4 py-1.5 rounded-lg bg-violet-600 hover:bg-violet-500 text-white font-semibold text-xs transition-colors"
                        >
                          🚀 Open GitHub Activation Page →
                        </a>
                      </div>

                      <div className="text-center text-[10px] text-zinc-500 animate-pulse pt-2">
                        ⏳ Waiting for user authorization on GitHub…
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* 2. Clone Repository Tab */}
          {activeTab === "clone" && (
            <div className="space-y-4 font-mono text-xs">
              {/* Custom URL Clone Input */}
              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <h3 className="text-xs font-bold text-white">Clone from Any Repository URL</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                  <input
                    type="text"
                    value={customRepoUrl}
                    onChange={(e) => setCustomRepoUrl(e.target.value)}
                    placeholder="https://github.com/username/repository.git"
                    className="bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                  />
                  <input
                    type="text"
                    value={targetCloneDir}
                    onChange={(e) => setTargetCloneDir(e.target.value)}
                    placeholder="./my-cloned-folder"
                    className="bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                  />
                </div>
                <button
                  onClick={() => handleCloneRepo(customRepoUrl)}
                  disabled={!customRepoUrl.trim() || cloning}
                  className="px-4 py-1.5 rounded-lg bg-violet-600 hover:bg-violet-500 disabled:opacity-40 text-white font-semibold text-xs"
                >
                  {cloning ? "⏳ Cloning..." : "📥 Clone to Directory"}
                </button>
              </div>

              {/* Status Message */}
              {cloneMsg && (
                <div
                  className={`p-3 rounded-lg border text-xs ${
                    cloneMsg.isError
                      ? "bg-red-950/30 border-red-500/30 text-red-300"
                      : "bg-emerald-950/30 border-emerald-500/30 text-emerald-300"
                  }`}
                >
                  {cloneMsg.text}
                </div>
              )}

              {/* User Repositories List */}
              {authStatus?.is_authenticated && (
                <div className="space-y-2 pt-2">
                  <div className="flex items-center justify-between">
                    <h3 className="text-xs font-bold text-zinc-300 uppercase tracking-wider">
                      Your GitHub Repositories ({repos.length}):
                    </h3>
                    <input
                      type="text"
                      value={repoFilter}
                      onChange={(e) => setRepoFilter(e.target.value)}
                      placeholder="Filter repositories..."
                      className="bg-black/40 border border-white/10 rounded-lg px-2.5 py-1 text-xs text-white"
                    />
                  </div>

                  {loadingRepos ? (
                    <div className="text-center py-6 text-zinc-400">Loading repositories…</div>
                  ) : (
                    <div className="grid grid-cols-1 gap-2 max-h-60 overflow-y-auto">
                      {filteredRepos.map((repo) => (
                        <div
                          key={repo.id}
                          className="p-3 rounded-lg bg-[#0e111d] border border-white/5 hover:border-violet-500/40 flex items-center justify-between transition-colors"
                        >
                          <div>
                            <div className="font-bold text-white flex items-center gap-2">
                              <span>{repo.name}</span>
                              {repo.private && (
                                <span className="text-[9px] px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300">
                                  Private
                                </span>
                              )}
                              <span className="text-zinc-500 text-[10px]">⭐ {repo.stargazers_count}</span>
                            </div>
                            <p className="text-zinc-400 text-[10px] truncate max-w-md">
                              {repo.description || "No description provided."}
                            </p>
                          </div>

                          <button
                            onClick={() => handleCloneRepo(repo.clone_url, repo.name)}
                            disabled={cloning}
                            className="px-3 py-1 rounded bg-violet-600/30 hover:bg-violet-600 text-violet-200 hover:text-white border border-violet-500/40 transition-colors text-xs"
                          >
                            Clone
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* 3. Smart Commit & PR Tab */}
          {activeTab === "sync" && (
            <div className="space-y-4 font-mono text-xs">
              {/* Live Working Tree Status */}
              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-sm">🌿</span>
                    <span className="font-bold text-white">Branch: {gitStatus?.branch || "main"}</span>
                  </div>
                  <div className="flex items-center gap-3 text-[11px] text-zinc-400">
                    <span>↑ {gitStatus?.ahead_commits || 0} Ahead</span>
                    <span>↓ {gitStatus?.behind_commits || 0} Behind</span>
                    <button
                      onClick={fetchGitStatus}
                      disabled={loadingGitStatus}
                      className="px-2 py-0.5 rounded bg-white/5 hover:bg-white/10 text-zinc-300"
                    >
                      🔄 Refresh
                    </button>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 text-center text-[10px]">
                  <div className="p-2 rounded bg-emerald-950/20 border border-emerald-500/20 text-emerald-300">
                    <div className="font-bold text-sm">{gitStatus?.staged_files.length || 0}</div>
                    <div>Staged Files</div>
                  </div>
                  <div className="p-2 rounded bg-amber-950/20 border border-amber-500/20 text-amber-300">
                    <div className="font-bold text-sm">{gitStatus?.modified_files.length || 0}</div>
                    <div>Modified Files</div>
                  </div>
                  <div className="p-2 rounded bg-zinc-900 border border-white/10 text-zinc-400">
                    <div className="font-bold text-sm">{gitStatus?.untracked_files.length || 0}</div>
                    <div>Untracked Files</div>
                  </div>
                </div>
              </div>

              {/* Smart Commit Box */}
              <div className="p-4 rounded-xl bg-[#0e111d] border border-violet-500/30 space-y-3">
                <h3 className="text-xs font-bold text-violet-300 uppercase tracking-wider">
                  ⚡ Autonomous Smart Commit
                </h3>
                <input
                  type="text"
                  value={intentSummary}
                  onChange={(e) => setIntentSummary(e.target.value)}
                  placeholder="Optional Intent (e.g. 'implement device flow auth and conventional commit synthesis')..."
                  className="w-full bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                />

                <div className="flex items-center justify-between">
                  <label className="flex items-center gap-2 text-zinc-400 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={autoPush}
                      onChange={(e) => setAutoPush(e.target.checked)}
                      className="rounded border-zinc-700 bg-black/40 text-violet-600 focus:ring-0"
                    />
                    <span>Automatically Push to Remote after commit</span>
                  </label>

                  <button
                    onClick={handleSmartCommit}
                    disabled={committing}
                    className="px-4 py-1.5 rounded-lg bg-violet-600 hover:bg-violet-500 disabled:opacity-40 text-white font-semibold shadow-sm transition-colors"
                  >
                    {committing ? "⏳ Committing..." : "✓ Stage & Smart Commit"}
                  </button>
                </div>

                {commitResult && (
                  <div className="p-3 rounded-lg bg-emerald-950/30 border border-emerald-500/30 text-emerald-300 space-y-1">
                    <div className="font-bold">✓ Committed: {commitResult.commit_hash}</div>
                    <div className="text-[11px] text-zinc-300">{commitResult.commit_message}</div>
                    {commitResult.pushed && <div className="text-[10px] text-emerald-400">🚀 Successfully pushed to remote</div>}
                  </div>
                )}
              </div>

              {/* Pull Request Box */}
              <div className="p-4 rounded-xl bg-black/40 border border-white/10 space-y-3">
                <h3 className="text-xs font-bold text-white uppercase tracking-wider">
                  🔀 Create Direct Pull Request
                </h3>
                <div className="grid grid-cols-2 gap-2">
                  <input
                    type="text"
                    value={prBase}
                    onChange={(e) => setPrBase(e.target.value)}
                    placeholder="Base Branch (e.g. main)"
                    className="bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                  />
                  <input
                    type="text"
                    value={prHead}
                    onChange={(e) => setPrHead(e.target.value)}
                    placeholder="Head Branch (e.g. feat/git-sync)"
                    className="bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                  />
                </div>
                <input
                  type="text"
                  value={prTitle}
                  onChange={(e) => setPrTitle(e.target.value)}
                  placeholder="PR Title (e.g. feat(sync): add device flow oauth)"
                  className="w-full bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                />
                <textarea
                  value={prBody}
                  onChange={(e) => setPrBody(e.target.value)}
                  placeholder="PR Description..."
                  rows={2}
                  className="w-full bg-black/60 border border-white/10 rounded-lg p-2 text-white text-xs"
                />

                <div className="flex justify-end">
                  <button
                    onClick={handleCreatePr}
                    disabled={!prTitle.trim() || creatingPr}
                    className="px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white font-semibold"
                  >
                    {creatingPr ? "⏳ Opening PR..." : "🔀 Create Pull Request"}
                  </button>
                </div>

                {prResult && (
                  <div className="p-3 rounded-lg bg-emerald-950/30 border border-emerald-500/30 text-emerald-300">
                    <div className="font-bold">✓ Pull Request #{prResult.pr_number} Created!</div>
                    <a
                      href={prResult.html_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-violet-400 hover:underline"
                    >
                      {prResult.html_url}
                    </a>
                  </div>
                )}
              </div>

              {/* Sync Error */}
              {syncError && (
                <div className="p-3 rounded-lg bg-red-950/30 border border-red-500/30 text-red-300">
                  {syncError}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-white/10 bg-[#0e1220] flex justify-end shrink-0">
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-xl bg-white/10 hover:bg-white/20 text-white text-xs font-semibold font-mono"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
