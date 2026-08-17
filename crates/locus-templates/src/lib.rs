pub mod loader;
pub mod searcher;
pub mod template;

use crate::loader::TemplateLoader;
use crate::searcher::TemplateSearcher;
use crate::template::Template;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TemplateStore {
    loader: Arc<RwLock<TemplateLoader>>,
}

impl TemplateStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            loader: Arc::new(RwLock::new(TemplateLoader::new()?)),
        })
    }

    pub async fn load_templates(&self) -> Vec<Template> {
        let loader = self.loader.read().await;
        loader.all().into_iter().cloned().collect()
    }

    pub async fn get_template(&self, category: &str, name: &str) -> Option<Template> {
        let loader = self.loader.read().await;
        let id = format!("{}/{}", category, name);
        loader.get(&id).cloned()
    }

    pub async fn search_templates(&self, query: &str) -> Vec<Template> {
        let loader = self.loader.read().await;
        let templates: Vec<&Template> = loader.all();
        TemplateSearcher::search(&templates, query)
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn get_by_category(&self, category: &str) -> Vec<Template> {
        let loader = self.loader.read().await;
        loader.get_by_category(category).into_iter().cloned().collect()
    }

    pub async fn get_categories(&self) -> Vec<String> {
        let loader = self.loader.read().await;
        loader.categories().into_iter().cloned().collect()
    }

    pub async fn reload(&self) -> Result<()> {
        let mut loader = self.loader.write().await;
        *loader = TemplateLoader::new()?;
        Ok(())
    }
}

impl Default for TemplateStore {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            loader: Arc::new(RwLock::new(TemplateLoader::default())),
        })
    }
}