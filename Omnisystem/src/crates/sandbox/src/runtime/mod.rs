//! Language runtime management and provisioning

pub mod manifest;
pub mod downloader;
pub mod plugin;

pub use manifest::{RuntimeManifest, RuntimeEntry};
pub use downloader::RuntimeDownloader;
pub use plugin::{LanguagePlugin, PluginRegistry, create_builtin_registry};

use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct Runtime {
    pub language: String,
    pub version: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RuntimeManager {
    cache_dir: PathBuf,
}

impl RuntimeManager {
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir).await?;
        Ok(Self { cache_dir })
    }

    /// Get or download a language runtime
    pub async fn get_runtime(&self, language: &str, version: &str) -> Result<Runtime> {
        let runtime_dir = self.cache_dir.join(format!("{}-{}", language, version));

        if !runtime_dir.exists() {
            // In production: download/build runtime
            fs::create_dir_all(&runtime_dir).await?;
        }

        Ok(Runtime {
            language: language.to_string(),
            version: version.to_string(),
            path: runtime_dir,
        })
    }
}
