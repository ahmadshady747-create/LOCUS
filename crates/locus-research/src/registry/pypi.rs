//! PyPI official registry connector for Python packages.

use crate::types::{Ecosystem, PackageMetadata};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct PypiResponse {
    info: PypiInfo,
}

#[derive(Debug, Deserialize)]
struct PypiInfo {
    name: String,
    version: String,
    summary: Option<String>,
    home_page: Option<String>,
    project_urls: Option<HashMap<String, String>>,
    license: Option<String>,
    description: Option<String>,
}

pub struct PypiClient {
    client: reqwest::Client,
}

impl Default for PypiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PypiClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn fetch_package(&self, pkg_name: &str) -> Result<(PackageMetadata, Option<String>)> {
        let clean_name = pkg_name.trim().to_lowercase();
        let url = format!("https://pypi.org/pypi/{}/json", clean_name);
        debug!("Fetching PyPI metadata for: {}", clean_name);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to query PyPI registry: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "PyPI registry returned status {}: package '{}' not found",
                resp.status(),
                clean_name
            ));
        }

        let data: PypiResponse = resp
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse PyPI JSON response: {}", e))?;

        let doc_url = data
            .info
            .project_urls
            .as_ref()
            .and_then(|urls| {
                urls.get("Documentation")
                    .or_else(|| urls.get("documentation"))
                    .or_else(|| urls.get("Homepage"))
                    .or_else(|| urls.get("homepage"))
            })
            .cloned()
            .or(data.info.home_page.clone())
            .unwrap_or_else(|| format!("https://pypi.org/project/{}", data.info.name));

        let repo_url = data.info.project_urls.as_ref().and_then(|urls| {
            urls.get("Source")
                .or_else(|| urls.get("Repository"))
                .or_else(|| urls.get("Code"))
                .or_else(|| urls.get("GitHub"))
                .cloned()
        });

        let meta = PackageMetadata {
            name: data.info.name,
            version: data.info.version,
            description: data
                .info
                .summary
                .unwrap_or_else(|| "No summary provided on PyPI.".to_string()),
            repository_url: repo_url,
            documentation_url: Some(doc_url),
            license: data.info.license,
            downloads: None,
            ecosystem: Ecosystem::Python,
        };

        Ok((meta, data.info.description))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pypi_json_deserialization() {
        let sample = r#"{
            "info": {
                "name": "fastapi",
                "version": "0.111.0",
                "summary": "FastAPI framework, high performance, easy to learn, fast to code, ready for production",
                "home_page": "https://fastapi.tiangolo.com/",
                "license": "MIT"
            }
        }"#;

        let parsed: PypiResponse = serde_json::from_str(sample).unwrap();
        assert_eq!(parsed.info.name, "fastapi");
        assert_eq!(parsed.info.version, "0.111.0");
    }
}
