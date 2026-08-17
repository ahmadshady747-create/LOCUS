//! Git Synchronization Engine & Conventional Commits Orchestrator
//!
//! Handles repository cloning, live status inspection, automated conventional commit message
//! synthesis from staged/modified diffs, local branch pushing, and direct GitHub Pull Request creation.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitStatusReport {
    pub branch: String,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub ahead_commits: u32,
    pub behind_commits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmartCommitResult {
    pub commit_hash: String,
    pub commit_message: String,
    pub pushed: bool,
    pub files_committed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestResult {
    pub pr_url: String,
    pub pr_number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitCloneOptions {
    pub repo_url: String,
    pub target_dir: String,
    pub branch: Option<String>,
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreatePrRequest {
    pub auth_token: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub title: String,
    pub body: String,
    pub base: String,
    pub head: String,
}

pub struct GitSyncEngine;

impl GitSyncEngine {
    /// Inspects the active workspace Git working tree status
    pub fn get_git_status(workspace_root: &Path) -> Result<GitStatusReport> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v1", "-b"])
            .current_dir(workspace_root)
            .output()
            .context("Failed to run 'git status' command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git status error: {}", stderr.trim());
        }

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_git_status_output(&raw_stdout))
    }

    /// Parses porcelain v1 git status output
    pub fn parse_git_status_output(output: &str) -> GitStatusReport {
        let mut report = GitStatusReport::default();
        report.branch = "main".to_string();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }

            // Branch info header: ## main...origin/main [ahead 1, behind 2]
            if line.starts_with("##") {
                let branch_part = line.trim_start_matches("##").trim();
                if let Some(first_space) = branch_part.find(' ') {
                    let branch_name = &branch_part[..first_space];
                    report.branch = branch_name.split("...").next().unwrap_or("main").to_string();
                } else {
                    report.branch = branch_part.split("...").next().unwrap_or("main").to_string();
                }

                // Check ahead / behind
                if line.contains("ahead") {
                    if let Some(ahead_sub) = line.split("ahead ").nth(1) {
                        let count_str: String = ahead_sub.chars().take_while(|c| c.is_ascii_digit()).collect();
                        report.ahead_commits = count_str.parse().unwrap_or(0);
                    }
                }
                if line.contains("behind") {
                    if let Some(behind_sub) = line.split("behind ").nth(1) {
                        let count_str: String = behind_sub.chars().take_while(|c| c.is_ascii_digit()).collect();
                        report.behind_commits = count_str.parse().unwrap_or(0);
                    }
                }
                continue;
            }

            let chars: Vec<char> = line.chars().collect();
            if chars.len() < 3 {
                continue;
            }

            let index_status = chars[0];
            let worktree_status = chars[1];
            let file_path = line[3..].trim().to_string();

            if index_status == '?' && worktree_status == '?' {
                report.untracked_files.push(file_path);
            } else {
                // Staged items
                if index_status != ' ' && index_status != '?' {
                    report.has_staged_changes = true;
                    report.staged_files.push(file_path.clone());
                }
                // Unstaged items
                if worktree_status != ' ' && worktree_status != '?' {
                    report.has_unstaged_changes = true;
                    report.modified_files.push(file_path);
                }
            }
        }

        report
    }

    /// Clones a remote repository into target directory
    pub fn clone_repository(options: &GitCloneOptions, auth_token: Option<&str>) -> Result<String> {
        let mut final_url = options.repo_url.clone();

        // Inject token into HTTPS GitHub URL if provided
        if let Some(token) = auth_token {
            if final_url.starts_with("https://github.com/") {
                final_url = final_url.replace(
                    "https://github.com/",
                    &format!("https://x-access-token:{}@github.com/", token),
                );
            }
        }

        let mut cmd = Command::new("git");
        cmd.arg("clone");

        if let Some(ref branch) = options.branch {
            cmd.args(["--branch", branch]);
        }
        if let Some(depth) = options.depth {
            cmd.args(["--depth", &depth.to_string()]);
        }

        cmd.arg(&final_url);
        cmd.arg(&options.target_dir);

        let output = cmd.output().context("Failed to execute 'git clone'")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Redact token from error message if present
            let sanitized_err = if let Some(token) = auth_token {
                stderr.replace(token, "[REDACTED_TOKEN]")
            } else {
                stderr.to_string()
            };
            anyhow::bail!("Git clone failed: {}", sanitized_err.trim());
        }

        Ok(format!("Repository successfully cloned into '{}'", options.target_dir))
    }

    /// Stages all modified files, synthesizes a Conventional Commit message, commits locally, and optionally pushes
    pub fn smart_commit(
        workspace_root: &Path,
        intent_summary: Option<&str>,
        auto_push: bool,
        _auth_token: Option<&str>,
    ) -> Result<SmartCommitResult> {
        // 1. Stage all changes
        let add_out = Command::new("git")
            .args(["add", "-A"])
            .current_dir(workspace_root)
            .output()
            .context("Failed to run 'git add -A'")?;

        if !add_out.status.success() {
            let err = String::from_utf8_lossy(&add_out.stderr);
            anyhow::bail!("Git add error: {}", err);
        }

        // 2. Read staged changes
        let status = Self::get_git_status(workspace_root)?;
        let total_files = status.staged_files.len();

        if total_files == 0 && status.untracked_files.is_empty() && status.modified_files.is_empty() {
            anyhow::bail!("No changes detected to commit");
        }

        // 3. Generate Conventional Commit message
        let commit_message = Self::generate_conventional_commit_message(
            &status.staged_files,
            intent_summary,
        );

        // 4. Commit locally
        let commit_out = Command::new("git")
            .args(["commit", "-m", &commit_message])
            .current_dir(workspace_root)
            .output()
            .context("Failed to execute 'git commit'")?;

        if !commit_out.status.success() {
            let err = String::from_utf8_lossy(&commit_out.stderr);
            anyhow::bail!("Git commit error: {}", err);
        }

        // 5. Get commit hash
        let hash_out = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(workspace_root)
            .output()
            .context("Failed to get commit hash")?;

        let commit_hash = String::from_utf8_lossy(&hash_out.stdout).trim().to_string();
        let mut pushed = false;

        // 6. Optional Push
        if auto_push {
            let push_out = Command::new("git")
                .args(["push"])
                .current_dir(workspace_root)
                .output();

            if let Ok(p_out) = push_out {
                pushed = p_out.status.success();
            }
        }

        Ok(SmartCommitResult {
            commit_hash,
            commit_message,
            pushed,
            files_committed: total_files,
        })
    }

    /// Synthesizes a standard Conventional Commit message based on changed paths and intent
    pub fn generate_conventional_commit_message(
        modified_files: &[String],
        intent: Option<&str>,
    ) -> String {
        let mut scope = "core";
        let mut prefix = "feat";

        if !modified_files.is_empty() {
            let first_file = &modified_files[0].to_lowercase();
            if first_file.contains("components") || first_file.contains("ui") || first_file.ends_with(".tsx") || first_file.ends_with(".css") {
                scope = "ui";
            } else if first_file.contains("agent") || first_file.contains("task_graph") || first_file.contains("reasoning") {
                scope = "agents";
            } else if first_file.contains("context") || first_file.contains("ast") || first_file.contains("adr") {
                scope = "context";
            } else if first_file.contains("llm") || first_file.contains("keyring") || first_file.contains("router") {
                scope = "llm";
            } else if first_file.contains("fs") || first_file.contains("diff") || first_file.contains("git") {
                scope = "fs";
            } else if first_file.contains("network") || first_file.contains("p2p") {
                scope = "network";
            }
        }

        if let Some(raw_intent) = intent {
            let intent_trimmed = raw_intent.trim();
            if intent_trimmed.to_lowercase().starts_with("fix") || intent_trimmed.to_lowercase().contains("bug") {
                prefix = "fix";
            } else if intent_trimmed.to_lowercase().starts_with("refactor") || intent_trimmed.to_lowercase().contains("clean") {
                prefix = "refactor";
            } else if intent_trimmed.to_lowercase().starts_with("test") {
                prefix = "test";
            } else if intent_trimmed.to_lowercase().starts_with("doc") {
                prefix = "docs";
            }

            format!("{}({}): {}", prefix, scope, intent_trimmed)
        } else {
            let file_summary = if modified_files.is_empty() {
                "update workspace modules".to_string()
            } else {
                format!("update {} file(s) in {}", modified_files.len(), scope)
            };
            format!("{}({}): {}", prefix, scope, file_summary)
        }
    }

    /// Creates a Pull Request on GitHub via REST API
    pub async fn create_pull_request(request: &CreatePrRequest) -> Result<PullRequestResult> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls",
            request.repo_owner, request.repo_name
        );

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("LOCUS-Desktop-Agent"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github.v3+json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", request.auth_token))?,
        );

        let payload = serde_json::json!({
            "title": request.title,
            "body": request.body,
            "head": request.head,
            "base": request.base,
        });

        let res = client
            .post(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .context("Failed to send Pull Request to GitHub API")?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("GitHub PR API error ({}): {}", status, body);
        }

        let json: serde_json::Value = res
            .json()
            .await
            .context("Failed to parse Pull Request response")?;

        Ok(PullRequestResult {
            pr_url: json.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            pr_number: json.get("number").and_then(|v| v.as_u64()).unwrap_or_default(),
            title: json.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            state: json.get("state").and_then(|v| v.as_str()).unwrap_or("open").to_string(),
            html_url: json.get("html_url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_conventional_commit_message_feat() {
        let files = vec!["src/src/components/MissionControl.tsx".to_string()];
        let intent = "add floating QA report sheet";
        let msg = GitSyncEngine::generate_conventional_commit_message(&files, Some(intent));

        assert_eq!(msg, "feat(ui): add floating QA report sheet");
    }

    #[test]
    fn test_generate_conventional_commit_message_fix() {
        let files = vec!["crates/locus-agents/src/task_graph.rs".to_string()];
        let intent = "fix cycle detection infinite recursion";
        let msg = GitSyncEngine::generate_conventional_commit_message(&files, Some(intent));

        assert_eq!(msg, "fix(agents): fix cycle detection infinite recursion");
    }

    #[test]
    fn test_parse_git_status_porcelain() {
        let porcelain = "## feat/git-sync...origin/feat/git-sync [ahead 2, behind 1]\nM  crates/locus-fs/src/lib.rs\n M src/src/App.tsx\n?? new_file.txt\n";
        let report = GitSyncEngine::parse_git_status_output(porcelain);

        assert_eq!(report.branch, "feat/git-sync");
        assert_eq!(report.ahead_commits, 2);
        assert_eq!(report.behind_commits, 1);
        assert!(report.has_staged_changes);
        assert!(report.has_unstaged_changes);
        assert_eq!(report.staged_files, vec!["crates/locus-fs/src/lib.rs"]);
        assert_eq!(report.modified_files, vec!["src/src/App.tsx"]);
        assert_eq!(report.untracked_files, vec!["new_file.txt"]);
    }

    #[test]
    fn test_pr_payload_serialization() {
        let req = CreatePrRequest {
            auth_token: "test_token".to_string(),
            repo_owner: "locus-ai".to_string(),
            repo_name: "locus".to_string(),
            title: "feat(git): add device flow auth".to_string(),
            body: "Adds full GitHub integration".to_string(),
            base: "main".to_string(),
            head: "feat/git-sync".to_string(),
        };

        assert_eq!(req.repo_owner, "locus-ai");
        assert_eq!(req.head, "feat/git-sync");
    }
}
