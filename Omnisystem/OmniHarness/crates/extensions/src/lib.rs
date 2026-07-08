//! `bonsai-extensions` — Extension system for the Bonsai Ecosystem.
//!
//! # Architecture
//!
//! ```text
//! ExtensionRegistry   — installed + discovered extensions
//! ├── ExtensionManifest — schema (bonsai-extension.yaml)
//! ├── SecurityScanner   — static analysis → SecurityReport
//! └── Installer         — GitHub fetch → validate → scan → install
//! ```

pub mod installer;
pub mod manifest;
pub mod registry;
pub mod scanner;

pub use installer::{InstallProgress, InstallResult, Installer};
pub use manifest::{
    AuthorInfo, ConfigField, ConfigFieldType, EntryPoints, ExtensionCategory,
    ExtensionManifest, ExtensionPermissions, FileAccessLevel, NetworkAccessLevel,
    ResourceLimits, SecurityReviewStub, SecurityVerdict,
};
pub use registry::{ExtensionCard, ExtensionRegistry, ExtensionStatus, InstalledExtension};
pub use scanner::{Finding, SecurityReport, SecurityScanner, Severity};

/// Initialise the default extension registry under `~/.bonsai/extensions/`.
pub fn default_registry() -> ExtensionRegistry {
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai")
        .join("extensions");
    std::fs::create_dir_all(&base).ok();
    ExtensionRegistry::new(base)
}

/// Errors returned by extension operations.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("fetch error: {0}")]
    Fetch(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("extension not found: {0}")]
    NotFound(String),
    #[error("blocked by security scanner")]
    SecurityBlocked,
}

impl From<ExtensionError> for String {
    fn from(e: ExtensionError) -> Self {
        e.to_string()
    }
}
