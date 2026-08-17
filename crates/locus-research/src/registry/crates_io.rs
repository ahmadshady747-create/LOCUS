//! Crates.io official registry connector for Rust packages.

use crate::types::{Ecosystem, PackageMetadata};
use anyhow::{anyhow, Result};
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::time::Duration;
use tracing::debug;

const CRATES_IO_USER_AGENT: &str = "LOCUS-Autonomous-Dev-OS (contact@locus.dev)";

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CrateDetails,
}

#[derive(Debug, Deserialize)]
struct CrateDetails {
    name: String,
    max_version: String,
    description: Option<String>,
    repository: Option<String>,
    documentation: Option<String>,
    downloads: Option<u64>,
}

pub struct CratesIoClient {
    client: reqwest::Client,
}

impl Default for CratesIoClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CratesIoClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_package(&self, crate_name: &str) -> Result<PackageMetadata> {
        let clean_name = crate_name.trim().to_lowercase();
        let url = format!("https://crates.io/api/v1/crates/{}", clean_name);

        debug!("Fetching crates.io metadata for: {}", clean_name);
        let resp = self
            .client
            .get(&url)
            .header(USER_AGENT, CRATES_IO_USER_AGENT)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to query crates.io: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "crates.io returned status {}: crate '{}' not found or registry unavailable",
                resp.status(),
                clean_name
            ));
        }

        let data: CratesIoResponse = resp
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse crates.io JSON response: {}", e))?;

        let doc_url = data.krate.documentation.unwrap_or_else(|| {
            format!("https://docs.rs/{}/{}", data.krate.name, data.krate.max_version)
        });

        Ok(PackageMetadata {
            name: data.krate.name,
            version: data.krate.max_version,
            description: data
                .krate
                .description
                .unwrap_or_else(|| "No description provided on crates.io.".to_string()),
            repository_url: data.krate.repository,
            documentation_url: Some(doc_url),
            license: None,
            downloads: data.krate.downloads,
            ecosystem: Ecosystem::Rust,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crates_io_json_deserialization() {
        let sample_json = r#"{
            "crate": {
                "name": "tokio",
                "max_version": "1.38.0",
                "description": "An event-driven, non-blocking I/O platform for writing asynchronous applications.",
                "repository": "https://github.com/tokio-rs/tokio",
                "documentation": "https://docs.rs/tokio",
                "downloads": 150000000
            }
        }"#;

        let parsed: CratesIoResponse = serde_json::from_str(sample_json).unwrap();
        assert_eq!(parsed.krate.name, "tokio");
        assert_eq!(parsed.krate.max_version, "1.38.0");
        assert_eq!(parsed.krate.downloads, Some(150000000));
    }
}
