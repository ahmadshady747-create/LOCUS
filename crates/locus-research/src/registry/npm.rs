//! NPM official registry connector for TypeScript & JavaScript packages.

use crate::types::{Ecosystem, PackageMetadata};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct NpmPackageResponse {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<HashMap<String, String>>,
    homepage: Option<String>,
    repository: Option<NpmRepository>,
    license: Option<serde_json::Value>,
    readme: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmRepository {
    Obj { url: Option<String> },
    Str(String),
}

pub struct NpmClient {
    client: reqwest::Client,
}

impl Default for NpmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NpmClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_package(&self, pkg_name: &str) -> Result<(PackageMetadata, Option<String>)> {
        let clean_name = pkg_name.trim();
        // Support scoped packages: @tanstack/react-query -> @tanstack%2Freact-query
        let encoded_name = if clean_name.starts_with('@') {
            clean_name.replace('/', "%2F")
        } else {
            clean_name.to_string()
        };

        let url = format!("https://registry.npmjs.org/{}", encoded_name);
        debug!("Fetching npm metadata for: {}", clean_name);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to query npm registry: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "NPM registry returned status {}: package '{}' not found",
                resp.status(),
                clean_name
            ));
        }

        let data: NpmPackageResponse = resp
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse npm JSON response: {}", e))?;

        let version = data
            .dist_tags
            .and_then(|dt| dt.get("latest").cloned())
            .unwrap_or_else(|| "latest".to_string());

        let repo_url = match data.repository {
            Some(NpmRepository::Obj { url }) => url.map(|u| u.trim_start_matches("git+").to_string()),
            Some(NpmRepository::Str(s)) => Some(s.trim_start_matches("git+").to_string()),
            None => None,
        };

        let license = data.license.and_then(|l| match l {
            serde_json::Value::String(s) => Some(s),
            serde_json::Value::Object(map) => map.get("type").and_then(|v| v.as_str().map(|s| s.to_string())),
            _ => None,
        });

        let meta = PackageMetadata {
            name: data.name,
            version,
            description: data
                .description
                .unwrap_or_else(|| "No description provided on npm.".to_string()),
            repository_url: repo_url,
            documentation_url: data.homepage.or_else(|| Some(format!("https://www.npmjs.com/package/{}", clean_name))),
            license,
            downloads: None,
            ecosystem: Ecosystem::TypeScript,
        };

        Ok((meta, data.readme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_response_deserialization() {
        let sample = r#"{
            "name": "zustand",
            "description": "Bear necessities for state management in React",
            "dist-tags": { "latest": "5.0.0" },
            "homepage": "https://github.com/pmndrs/zustand",
            "repository": { "type": "git", "url": "git+https://github.com/pmndrs/zustand.git" },
            "license": "MIT"
        }"#;

        let parsed: NpmPackageResponse = serde_json::from_str(sample).unwrap();
        assert_eq!(parsed.name, "zustand");
        assert_eq!(parsed.dist_tags.unwrap().get("latest").unwrap(), "5.0.0");
    }
}
