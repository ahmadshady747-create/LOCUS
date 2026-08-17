use crate::template::{Template, TemplateManifest};
use anyhow::Result;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

#[derive(RustEmbed)]
#[folder = "templates/"]
#[prefix = "templates/"]
struct TemplateAssets;

pub struct TemplateLoader {
    templates: HashMap<String, Template>,
    by_category: HashMap<String, Vec<String>>,
}

impl TemplateLoader {
    pub fn new() -> Result<Self> {
        let mut loader = Self {
            templates: HashMap::new(),
            by_category: HashMap::new(),
        };
        loader.load_embedded_templates()?;
        Ok(loader)
    }

    fn load_embedded_templates(&mut self) -> Result<()> {
        for path in TemplateAssets::iter() {
            if path.ends_with(".json") {
                if let Some(content) = TemplateAssets::get(&path) {
                    match self.parse_template_file(&path, content.data.as_ref()) {
                        Ok(template) => {
                            let id = template.id.clone();
                            let category = template.category.clone();
                            self.by_category.entry(category).or_default().push(id.clone());
                            self.templates.insert(id, template);
                        }
                        Err(e) => {
                            warn!("Failed to parse template {}: {}", path, e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_template_file(&self, path: &str, data: &[u8]) -> Result<Template> {
        let manifest: TemplateManifest = serde_json::from_slice(data)?;
        if manifest.templates.is_empty() {
            anyhow::bail!("No templates in {}", path);
        }
        Ok(manifest.templates.into_iter().next().unwrap())
    }

    pub fn load_from_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<()> {
        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(path)?;
                if let Ok(template) = self.parse_template_file(path.to_str().unwrap_or(""), content.as_bytes()) {
                    let id = template.id.clone();
                    let category = template.category.clone();
                    self.by_category.entry(category).or_default().push(id.clone());
                    self.templates.insert(id, template);
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    pub fn get_by_category(&self, category: &str) -> Vec<&Template> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.templates.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn all(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn categories(&self) -> Vec<&String> {
        self.by_category.keys().collect()
    }
}

impl Default for TemplateLoader {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            templates: HashMap::new(),
            by_category: HashMap::new(),
        })
    }
}