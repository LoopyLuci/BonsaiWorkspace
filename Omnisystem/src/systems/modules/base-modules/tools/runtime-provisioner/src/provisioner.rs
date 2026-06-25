//! Core runtime provisioner

use crate::{CacheManager, RuntimeType, Result, ProvisionerError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionerConfig {
    /// Cache directory for runtimes
    pub cache_dir: PathBuf,
    /// Auto-download missing runtimes
    pub auto_download: bool,
    /// Verify checksums
    pub verify_checksums: bool,
    /// Max parallel downloads
    pub max_parallel_downloads: usize,
    /// Timeout in seconds
    pub download_timeout_seconds: u64,
}

impl Default for ProvisionerConfig {
    fn default() -> Self {
        let cache_dir = directories::ProjectDirs::from("dev", "omnisystem", "runtime-cache")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| {
                if cfg!(target_os = "windows") {
                    PathBuf::from(format!(
                        "{}\\AppData\\Local\\Omnisystem\\runtime-cache",
                        std::env::var("USERNAME").unwrap_or_default()
                    ))
                } else {
                    PathBuf::from("~/.omnisystem/runtime-cache")
                }
            });

        Self {
            cache_dir,
            auto_download: true,
            verify_checksums: true,
            max_parallel_downloads: 4,
            download_timeout_seconds: 300,
        }
    }
}

/// Core runtime provisioner
pub struct RuntimeProvisioner {
    config: ProvisionerConfig,
    cache_manager: CacheManager,
}

impl RuntimeProvisioner {
    /// Create new provisioner
    pub fn new(config: ProvisionerConfig) -> Result<Self> {
        let cache_manager = CacheManager::new(config.cache_dir.clone())?;
        cache_manager.load()?;

        Ok(Self {
            config,
            cache_manager,
        })
    }

    /// Ensure runtime is available (download if needed)
    pub async fn ensure_runtime(
        &self,
        runtime_type: RuntimeType,
        version: &str,
    ) -> Result<String> {
        // Check cache first
        if let Some(entry) = self.cache_manager.cache().get(runtime_type, version) {
            log::info!(
                "Runtime {}/{} found in cache at {}",
                runtime_type.name(),
                version,
                entry.location.display()
            );
            return Ok(entry.location.to_string_lossy().to_string());
        }

        if !self.config.auto_download {
            return Err(ProvisionerError::RuntimeNotFound(format!(
                "{}/{}",
                runtime_type.name(),
                version
            )));
        }

        // Download if not cached
        log::info!(
            "Downloading runtime {}/{} (auto-provisioning for independence)...",
            runtime_type.name(),
            version
        );
        self.download_runtime(runtime_type, version).await
    }

    /// Download runtime
    async fn download_runtime(&self, runtime_type: RuntimeType, version: &str) -> Result<String> {
        // Stub: real implementation would:
        // 1. Query runtime index
        // 2. Download from official sources
        // 3. Verify checksums
        // 4. Extract to cache
        // 5. Update manifest

        let runtime_dir = self
            .config
            .cache_dir
            .join(runtime_type.name())
            .join(version);

        std::fs::create_dir_all(&runtime_dir)?;

        log::info!(
            "Runtime {}/{} provisioned to {}",
            runtime_type.name(),
            version,
            runtime_dir.display()
        );

        Ok(runtime_dir.to_string_lossy().to_string())
    }

    /// List all available runtimes
    pub fn list_available() -> Vec<&'static str> {
        RuntimeType::all()
            .iter()
            .map(|rt| rt.name())
            .collect()
    }

    /// List installed runtimes
    pub fn list_installed(&self) -> Vec<(String, String)> {
        self.cache_manager
            .cache()
            .list()
            .into_iter()
            .map(|e| (format!("{}/{}", e.runtime_type.name(), e.version), e.location.to_string_lossy().to_string()))
            .collect()
    }

    /// Get cache info
    pub fn cache_info(&self) -> CacheInfo {
        let entries = self.cache_manager.cache().list();
        let total_size = self.cache_manager.cache().total_size();

        CacheInfo {
            entry_count: entries.len(),
            total_size_bytes: total_size,
            total_size_mb: total_size as f64 / 1024.0 / 1024.0,
            entries,
        }
    }

    /// Clear all cached runtimes
    pub fn clear_cache(&self) -> Result<()> {
        self.cache_manager.cache().clear()?;
        self.cache_manager.save()?;
        Ok(())
    }

    /// Save cache manifest
    pub fn save_cache(&self) -> Result<()> {
        self.cache_manager.save()
    }

    pub fn config(&self) -> &ProvisionerConfig {
        &self.config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub entry_count: usize,
    pub total_size_bytes: u64,
    pub total_size_mb: f64,
    pub entries: Vec<crate::cache::CacheEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provisioner_creation() {
        let config = ProvisionerConfig::default();
        let provisioner = RuntimeProvisioner::new(config);
        assert!(provisioner.is_ok());
    }

    #[test]
    fn test_list_available_runtimes() {
        let available = RuntimeProvisioner::list_available();
        assert!(available.contains(&"rust"));
        assert!(available.contains(&"python"));
        assert!(available.contains(&"go"));
    }

    #[test]
    fn test_cache_info() {
        let config = ProvisionerConfig::default();
        let provisioner = RuntimeProvisioner::new(config).unwrap();
        let info = provisioner.cache_info();
        assert_eq!(info.entry_count, 0); // Empty cache
    }
}
