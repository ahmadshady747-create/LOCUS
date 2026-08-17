//! GitHub OAuth Device Flow Authentication Client
//!
//! Provides zero-server direct Device Flow authentication (`/login/device/code` & `/login/oauth/access_token`),
//! user profile fetching, repository inspection, and secure local token storage.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_GITHUB_CLIENT_ID: &str = "Ov23lia8VLOCUSApp"; // Configurable / default OAuth client
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const GITHUB_USER_API: &str = "https://api.github.com/user";
pub const GITHUB_REPOS_API: &str = "https://api.github.com/user/repos";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub public_repos: u32,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub private: bool,
    #[serde(default = "default_branch_name")]
    pub default_branch: String,
    #[serde(default)]
    pub stargazers_count: u32,
}

fn default_branch_name() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubAuthStatus {
    pub is_authenticated: bool,
    pub user: Option<GitHubUser>,
    pub token_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceFlowPollStatus {
    Pending,
    SlowDown(u64),
    Expired,
    Denied,
    Success(String),
    Error(String),
}

#[derive(Debug, Deserialize)]
struct RawTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    interval: Option<u64>,
}

pub struct GitHubAuthClient;

impl GitHubAuthClient {
    fn build_headers(token: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("LOCUS-Desktop-Agent"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        if let Some(t) = token {
            if let Ok(auth_val) = HeaderValue::from_str(&format!("Bearer {}", t)) {
                headers.insert(AUTHORIZATION, auth_val);
            }
        }
        headers
    }

    /// Step 1: Initiates GitHub OAuth Device Flow, returning a verification URI and user code
    pub async fn request_device_code(client_id: &str, scope: &str) -> Result<DeviceCodeResponse> {
        let client = reqwest::Client::new();
        let params = [
            ("client_id", client_id),
            ("scope", if scope.is_empty() { "repo,user" } else { scope }),
        ];

        let res = client
            .post(GITHUB_DEVICE_CODE_URL)
            .headers(Self::build_headers(None))
            .form(&params)
            .send()
            .await
            .context("Failed to connect to GitHub Device Flow endpoint")?;

        let code_res: DeviceCodeResponse = res
            .json()
            .await
            .context("Failed to parse GitHub Device Code response")?;

        Ok(code_res)
    }

    /// Step 2: Polls GitHub for access token authorization
    pub async fn poll_access_token(client_id: &str, device_code: &str) -> Result<DeviceFlowPollStatus> {
        let client = reqwest::Client::new();
        let params = [
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let res = client
            .post(GITHUB_TOKEN_URL)
            .headers(Self::build_headers(None))
            .form(&params)
            .send()
            .await
            .context("Failed to poll GitHub access token")?;

        let raw: RawTokenResponse = res
            .json()
            .await
            .context("Failed to parse GitHub token polling response")?;

        if let Some(token) = raw.access_token {
            return Ok(DeviceFlowPollStatus::Success(token));
        }

        if let Some(err) = raw.error {
            match err.as_str() {
                "authorization_pending" => Ok(DeviceFlowPollStatus::Pending),
                "slow_down" => Ok(DeviceFlowPollStatus::SlowDown(raw.interval.unwrap_or(5))),
                "expired_token" => Ok(DeviceFlowPollStatus::Expired),
                "access_denied" => Ok(DeviceFlowPollStatus::Denied),
                other => Ok(DeviceFlowPollStatus::Error(
                    raw.error_description.unwrap_or_else(|| other.to_string()),
                )),
            }
        } else {
            Ok(DeviceFlowPollStatus::Error("Unknown GitHub OAuth response".to_string()))
        }
    }

    /// Step 3: Fetches the authenticated user's profile
    pub async fn fetch_user_profile(token: &str) -> Result<GitHubUser> {
        let client = reqwest::Client::new();
        let res = client
            .get(GITHUB_USER_API)
            .headers(Self::build_headers(Some(token)))
            .send()
            .await
            .context("Failed to fetch GitHub user profile")?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        let user: GitHubUser = res
            .json()
            .await
            .context("Failed to deserialize GitHub user payload")?;

        Ok(user)
    }

    /// Step 4: Lists the authenticated user's repositories
    pub async fn fetch_user_repositories(
        token: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<GitHubRepo>> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}?sort=updated&per_page={}&page={}",
            GITHUB_REPOS_API,
            per_page.min(100).max(1),
            page.max(1)
        );

        let res = client
            .get(&url)
            .headers(Self::build_headers(Some(token)))
            .send()
            .await
            .context("Failed to fetch GitHub repositories")?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error ({}): {}", status, body);
        }

        let repos: Vec<GitHubRepo> = res
            .json()
            .await
            .context("Failed to deserialize GitHub repos list")?;

        Ok(repos)
    }

    // --- Token Persistence & Status ---

    fn get_token_file_path() -> PathBuf {
        let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join(".locus").join("github_auth.json")
    }

    pub fn save_token(token: &str) -> Result<()> {
        let path = Self::get_token_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::json!({
            "access_token": token,
            "saved_at": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(&path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    pub fn load_token() -> Option<String> {
        let path = Self::get_token_file_path();
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        json.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn clear_token() -> Result<()> {
        let path = Self::get_token_file_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub async fn get_auth_status() -> GitHubAuthStatus {
        let token = Self::load_token();
        if let Some(t) = token {
            match Self::fetch_user_profile(&t).await {
                Ok(user) => {
                    let preview = if t.len() > 8 {
                        format!("gho_{}...{}", &t[4..8], &t[t.len() - 4..])
                    } else {
                        "gho_****".to_string()
                    };
                    GitHubAuthStatus {
                        is_authenticated: true,
                        user: Some(user),
                        token_preview: Some(preview),
                        error: None,
                    }
                }
                Err(e) => GitHubAuthStatus {
                    is_authenticated: false,
                    user: None,
                    token_preview: None,
                    error: Some(format!("Session expired or invalid: {}", e)),
                },
            }
        } else {
            GitHubAuthStatus {
                is_authenticated: false,
                user: None,
                token_preview: None,
                error: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_code_response_deserialization() {
        let json = r#"{
            "device_code": "3584d83530557fdd1f46af828236688880f679a9",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;

        let res: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.user_code, "WDJB-MJHT");
        assert_eq!(res.expires_in, 900);
        assert_eq!(res.interval, 5);
    }

    #[test]
    fn test_github_user_deserialization() {
        let json = r#"{
            "login": "octocat",
            "id": 1,
            "avatar_url": "https://github.com/images/error/octocat_happy.gif",
            "name": "The Octocat",
            "email": "octocat@github.com",
            "public_repos": 8,
            "html_url": "https://github.com/octocat"
        }"#;

        let user: GitHubUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "octocat");
        assert_eq!(user.public_repos, 8);
        assert_eq!(user.name.as_deref(), Some("The Octocat"));
    }

    #[test]
    fn test_github_repo_deserialization() {
        let json = r#"{
            "id": 1296269,
            "name": "Hello-World",
            "full_name": "octocat/Hello-World",
            "description": "This your first repo!",
            "html_url": "https://github.com/octocat/Hello-World",
            "clone_url": "https://github.com/octocat/Hello-World.git",
            "private": false,
            "default_branch": "main",
            "stargazers_count": 80
        }"#;

        let repo: GitHubRepo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.name, "Hello-World");
        assert_eq!(repo.default_branch, "main");
        assert_eq!(repo.stargazers_count, 80);
        assert!(!repo.private);
    }

    #[test]
    fn test_token_save_load_clear() {
        let test_token = "gho_sampletesttoken123456789";
        GitHubAuthClient::save_token(test_token).unwrap();

        let loaded = GitHubAuthClient::load_token();
        assert_eq!(loaded.as_deref(), Some(test_token));

        GitHubAuthClient::clear_token().unwrap();
        let cleared = GitHubAuthClient::load_token();
        assert!(cleared.is_none());
    }
}
