//! Unified registry dispatcher for multi-ecosystem package and docs lookups.

pub mod crates_io;
pub mod docs_rs;
pub mod npm;
pub mod pypi;

use crate::types::{DocQuery, DocSearchResult, Ecosystem};
use anyhow::{anyhow, Result};
use crates_io::CratesIoClient;
use docs_rs::DocsRsClient;
use npm::NpmClient;
use pypi::PypiClient;
use tracing::info;

pub struct RegistryDispatcher {
    crates_io: CratesIoClient,
    pub docs_rs: DocsRsClient,
    npm: NpmClient,
    pypi: PypiClient,
}

impl Default for RegistryDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryDispatcher {
    pub fn new() -> Self {
        Self {
            crates_io: CratesIoClient::new(),
            docs_rs: DocsRsClient::new(),
            npm: NpmClient::new(),
            pypi: PypiClient::new(),
        }
    }

    pub async fn fetch_package_doc(&self, query: &DocQuery) -> Result<DocSearchResult> {
        let pkg_name = query.query.trim();
        if pkg_name.is_empty() {
            return Err(anyhow!("Package name query cannot be empty"));
        }

        match query.ecosystem {
            Ecosystem::Rust => self.fetch_rust_doc(pkg_name, query.version.as_deref()).await,
            Ecosystem::TypeScript => self.fetch_typescript_doc(pkg_name).await,
            Ecosystem::Python => self.fetch_python_doc(pkg_name).await,
            Ecosystem::General => {
                // Auto-detect fallback chain: Rust -> NPM -> PyPI
                if let Ok(res) = self.fetch_rust_doc(pkg_name, query.version.as_deref()).await {
                    return Ok(res);
                }
                if let Ok(res) = self.fetch_typescript_doc(pkg_name).await {
                    return Ok(res);
                }
                if let Ok(res) = self.fetch_python_doc(pkg_name).await {
                    return Ok(res);
                }
                Err(anyhow!(
                    "Package '{}' could not be resolved in any supported registry (crates.io, npm, PyPI)",
                    pkg_name
                ))
            }
        }
    }

    async fn fetch_rust_doc(&self, pkg_name: &str, _version: Option<&str>) -> Result<DocSearchResult> {
        info!("Resolving Rust crate '{}' via crates.io & docs.rs", pkg_name);
        let meta = self.crates_io.fetch_package(pkg_name).await?;
        let doc_url = meta.documentation_url.clone().unwrap_or_else(|| {
            format!("https://docs.rs/{}", meta.name)
        });

        let summary = format!(
            "# {} (v{})\n\n**Ecosystem:** Rust (crates.io)\n**Description:** {}\n\n- **Cargo Dependency:** `{} = \"{}\"`\n- **Docs:** {}\n- **Repository:** {}\n",
            meta.name,
            meta.version,
            meta.description,
            meta.name,
            meta.version,
            doc_url,
            meta.repository_url.as_deref().unwrap_or("N/A")
        );

        let signatures = vec![
            format!("use {}::*;", meta.name.replace('-', "_")),
            format!("// Add to Cargo.toml:\n{} = \"{}\"", meta.name, meta.version),
        ];

        Ok(DocSearchResult {
            package: meta,
            summary_markdown: summary,
            signatures,
            cached: false,
            source_url: doc_url,
        })
    }

    async fn fetch_typescript_doc(&self, pkg_name: &str) -> Result<DocSearchResult> {
        info!("Resolving TypeScript/JavaScript package '{}' via NPM", pkg_name);
        let (meta, readme) = self.npm.fetch_package(pkg_name).await?;

        let readme_excerpt = readme
            .map(|r| {
                let lines: Vec<&str> = r.lines().take(40).collect();
                lines.join("\n")
            })
            .unwrap_or_else(|| "No README provided on NPM.".to_string());

        let summary = format!(
            "# {} (v{})\n\n**Ecosystem:** TypeScript / JavaScript (NPM)\n**Description:** {}\n\n- **Install:** `npm install {}` or `pnpm add {}`\n- **Docs:** {}\n- **Repository:** {}\n\n### Overview\n{}\n",
            meta.name,
            meta.version,
            meta.description,
            meta.name,
            meta.name,
            meta.documentation_url.as_deref().unwrap_or("N/A"),
            meta.repository_url.as_deref().unwrap_or("N/A"),
            readme_excerpt
        );

        let signatures = vec![
            format!("import {{ ... }} from \"{}\";", meta.name),
            format!("const pkg = require(\"{}\");", meta.name),
        ];

        let source_url = meta.documentation_url.clone().unwrap_or_else(|| {
            format!("https://www.npmjs.com/package/{}", meta.name)
        });

        Ok(DocSearchResult {
            package: meta,
            summary_markdown: summary,
            signatures,
            cached: false,
            source_url,
        })
    }

    async fn fetch_python_doc(&self, pkg_name: &str) -> Result<DocSearchResult> {
        info!("Resolving Python package '{}' via PyPI", pkg_name);
        let (meta, desc) = self.pypi.fetch_package(pkg_name).await?;

        let desc_excerpt = desc
            .map(|d| {
                let lines: Vec<&str> = d.lines().take(40).collect();
                lines.join("\n")
            })
            .unwrap_or_else(|| "No description provided on PyPI.".to_string());

        let summary = format!(
            "# {} (v{})\n\n**Ecosystem:** Python (PyPI)\n**Description:** {}\n\n- **Install:** `pip install {}` or `uv add {}`\n- **Docs:** {}\n- **Repository:** {}\n\n### Overview\n{}\n",
            meta.name,
            meta.version,
            meta.description,
            meta.name,
            meta.name,
            meta.documentation_url.as_deref().unwrap_or("N/A"),
            meta.repository_url.as_deref().unwrap_or("N/A"),
            desc_excerpt
        );

        let signatures = vec![
            format!("import {}", meta.name.replace('-', "_")),
            format!("from {} import ...", meta.name.replace('-', "_")),
        ];

        let source_url = meta.documentation_url.clone().unwrap_or_else(|| {
            format!("https://pypi.org/project/{}/", meta.name)
        });

        Ok(DocSearchResult {
            package: meta,
            summary_markdown: summary,
            signatures,
            cached: false,
            source_url,
        })
    }
}
