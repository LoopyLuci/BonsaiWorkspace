//! Error types

#[derive(Debug, Clone)]
pub enum AppManagerError {
    /// Manifest failed validation
    InvalidManifest(String),
    /// JSON (de)serialization failed
    JsonError(String),
    /// Semver parsing failed
    SemverError(String),
    /// App not found in the registry
    AppNotFound(String),
    /// Module not found in the registry
    ModuleNotFound(String),
    /// Invalid app id
    InvalidAppId(String),
    /// Invalid version string
    InvalidVersion(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for AppManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppManagerError::InvalidManifest(msg) => write!(f, "invalid manifest: {}", msg),
            AppManagerError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            AppManagerError::SemverError(msg) => write!(f, "version parse error: {}", msg),
            AppManagerError::AppNotFound(id) => write!(f, "app not found: {}", id),
            AppManagerError::ModuleNotFound(id) => write!(f, "module not found: {}", id),
            AppManagerError::InvalidAppId(id) => write!(f, "invalid app id: {}", id),
            AppManagerError::InvalidVersion(s) => write!(f, "invalid version: {}", s),
            AppManagerError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AppManagerError {}

impl From<serde_json::Error> for AppManagerError {
    fn from(e: serde_json::Error) -> Self {
        AppManagerError::JsonError(e.to_string())
    }
}

impl From<semver::Error> for AppManagerError {
    fn from(e: semver::Error) -> Self {
        AppManagerError::SemverError(e.to_string())
    }
}

/// Result type
pub type AppManagerResult<T> = std::result::Result<T, AppManagerError>;
