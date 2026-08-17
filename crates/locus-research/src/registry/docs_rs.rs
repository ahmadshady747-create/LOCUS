//! Docs.rs documentation parser and signature extractor.

use anyhow::{anyhow, Result};
use reqwest::header::USER_AGENT;
use std::time::Duration;
use tracing::debug;

const DOCS_RS_USER_AGENT: &str = "LOCUS-Autonomous-Dev-OS (contact@locus.dev)";

pub struct DocsRsClient {
    client: reqwest::Client,
}

impl Default for DocsRsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DocsRsClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_raw_docs(&self, crate_name: &str, version: Option<&str>) -> Result<String> {
        let clean_name = crate_name.trim().to_lowercase();
        let url = match version {
            Some(v) => format!("https://docs.rs/{}/{}/", clean_name, v),
            None => format!("https://docs.rs/{}/latest/{}/", clean_name, clean_name),
        };

        debug!("Fetching docs.rs content from: {}", url);
        let resp = self
            .client
            .get(&url)
            .header(USER_AGENT, DOCS_RS_USER_AGENT)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to query docs.rs: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "docs.rs returned status {}: documentation for '{}' unavailable",
                resp.status(),
                clean_name
            ));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read docs.rs response body: {}", e))?;
        Ok(body)
    }
}
