use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocusError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, LocusError>;

impl From<toml::de::Error> for LocusError {
    fn from(err: toml::de::Error) -> Self {
        LocusError::Config(err.to_string())
    }
}

impl From<toml_edit::TomlError> for LocusError {
    fn from(err: toml_edit::TomlError) -> Self {
        LocusError::Config(err.to_string())
    }
}

impl From<walkdir::Error> for LocusError {
    fn from(err: walkdir::Error) -> Self {
        LocusError::FileSystem(err.to_string())
    }
}

impl From<notify::Error> for LocusError {
    fn from(err: notify::Error) -> Self {
        LocusError::FileSystem(err.to_string())
    }
}

impl From<tera::Error> for LocusError {
    fn from(err: tera::Error) -> Self {
        LocusError::Template(err.to_string())
    }
}

impl From<minijinja::Error> for LocusError {
    fn from(err: minijinja::Error) -> Self {
        LocusError::Template(err.to_string())
    }
}

impl From<toml::ser::Error> for LocusError {
    fn from(err: toml::ser::Error) -> Self {
        LocusError::Config(err.to_string())
    }
}