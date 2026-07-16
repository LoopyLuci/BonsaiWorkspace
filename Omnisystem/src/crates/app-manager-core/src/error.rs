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
    /// Module has no recorded lifecycle state
    ModuleNotLoaded(String),
    /// Attempted an illegal module lifecycle state transition
    InvalidStateTransition(String),
    /// Circular dependency detected in the dependency graph
    CircularDependency(String),
    /// (De)serialization of a value failed
    SerializationError(String),
    /// Internal invariant violation
    Internal(String),
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
            AppManagerError::ModuleNotLoaded(id) => write!(f, "module not loaded: {}", id),
            AppManagerError::InvalidStateTransition(msg) => write!(f, "invalid state transition: {}", msg),
            AppManagerError::CircularDependency(msg) => write!(f, "circular dependency: {}", msg),
            AppManagerError::SerializationError(msg) => write!(f, "serialization error: {}", msg),
            AppManagerError::Internal(msg) => write!(f, "internal error: {}", msg),
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
