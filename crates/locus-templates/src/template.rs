use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Template {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub security_level: SecurityLevel,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    Safe,
    ReviewRequired,
    Dangerous,
}

impl Template {
    pub fn new(
        id: impl Into<String>,
        category: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        code: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            category: category.into(),
            name: name.into(),
            description: description.into(),
            code: code.into(),
            language: language.into(),
            security_level: SecurityLevel::Safe,
            tags: vec![],
            dependencies: vec![],
            version: "1.0.0".to_string(),
        }
    }

    pub fn with_security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
     pub struct TemplateManifest {
    pub templates: Vec<Template>,
}



