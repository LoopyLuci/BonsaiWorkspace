// Data Layer Management
// Handles separation of UMD (source), Generated (code), and User Data (artifacts)

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Data folder types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFolder {
    /// Universal Module Database - source modules (read-only)
    UmdSource,
    /// Generated code and cache (can be rebuilt)
    Generated,
    /// User data and artifacts (protected)
    UserData,
}

/// Manages data layer with proper isolation
#[derive(Debug, Clone)]
pub struct DataLayerManager {
    base_path: PathBuf,
    umd_source: PathBuf,
    generated: PathBuf,
    user_data: PathBuf,
}

impl DataLayerManager {
    /// Create new data layer manager
    pub async fn new(base_path: &Path) -> Result<Self> {
        let base = base_path.to_path_buf();

        let umd_source = base.join("umd");
        let generated = base.join("generated");
        let user_data = base.join("user-data");

        // Create folders if needed
        tokio::fs::create_dir_all(&umd_source).await?;
        tokio::fs::create_dir_all(&generated).await?;
        tokio::fs::create_dir_all(&user_data).await?;

        Ok(Self {
            base_path: base,
            umd_source,
            generated,
            user_data,
        })
    }

    /// Get UMD source path (read-only module database)
    pub async fn umd_source(&self) -> Result<PathBuf> {
        Ok(self.umd_source.clone())
    }

    /// Get generated code path
    pub async fn generated(&self) -> Result<PathBuf> {
        Ok(self.generated.clone())
    }

    /// Get user data path
    pub async fn user_data(&self) -> Result<PathBuf> {
        Ok(self.user_data.clone())
    }

    /// Get path for UMD module
    pub async fn umd_module_path(&self, module_name: &str) -> Result<PathBuf> {
        Ok(self.umd_source.join(module_name))
    }

    /// Get path for generated code (specific language)
    pub async fn generated_code_path(&self, language: &str) -> Result<PathBuf> {
        let path = self.generated.join(language);
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get path for module in specific language
    pub async fn generated_module_path(
        &self,
        language: &str,
        module_name: &str,
    ) -> Result<PathBuf> {
        let path = self.generated.join(language).join(module_name);
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get path for user data
    pub async fn user_data_path(&self, subdir: &str) -> Result<PathBuf> {
        let path = self.user_data.join(subdir);
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get cache path
    pub async fn cache_path(&self) -> Result<PathBuf> {
        let path = self.base_path.join("cache");
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get path for Titan transpilation cache
    pub async fn titan_cache_path(&self) -> Result<PathBuf> {
        let path = self.base_path.join("cache").join("titan");
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get logs path
    pub async fn logs_path(&self) -> Result<PathBuf> {
        let path = self.user_data.join("logs");
        tokio::fs::create_dir_all(&path).await?;
        Ok(path)
    }

    /// Get base path
    pub fn base(&self) -> &Path {
        &self.base_path
    }

    /// Verify data layer integrity
    pub async fn verify(&self) -> Result<()> {
        // Check UMD source is readable
        if !self.umd_source.exists() {
            return Err(anyhow::anyhow!("UMD source directory not found"));
        }

        // Check generated is writable
        if !self.generated.exists() {
            return Err(anyhow::anyhow!("Generated directory not found"));
        }

        // Check user data is writable
        if !self.user_data.exists() {
            return Err(anyhow::anyhow!("User data directory not found"));
        }

        // Verify registry exists in UMD
        let registry_file = self.umd_source.join("registry.json");
        if !registry_file.exists() {
            tracing::warn!("UMD registry not found at {:?}", registry_file);
        }

        Ok(())
    }

    /// Get directory structure info
    pub async fn info(&self) -> Result<DataLayerInfo> {
        let umd_size = self.dir_size(&self.umd_source).await.unwrap_or(0);
        let generated_size = self.dir_size(&self.generated).await.unwrap_or(0);
        let user_data_size = self.dir_size(&self.user_data).await.unwrap_or(0);

        Ok(DataLayerInfo {
            base_path: self.base_path.clone(),
            umd_source_path: self.umd_source.clone(),
            generated_path: self.generated.clone(),
            user_data_path: self.user_data.clone(),
            umd_source_bytes: umd_size,
            generated_bytes: generated_size,
            user_data_bytes: user_data_size,
        })
    }

    async fn dir_size(&self, path: &Path) -> Result<u64> {
        let mut size = 0u64;
        let mut entries = tokio::fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                size += metadata.len();
            } else if metadata.is_dir() {
                size += self.dir_size_impl(&entry.path()).await?;
            }
        }

        Ok(size)
    }

    fn dir_size_impl<'a>(&'a self, path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + 'a>> {
        Box::pin(async move {
            let mut size = 0u64;
            let mut entries = tokio::fs::read_dir(path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let metadata = entry.metadata().await?;
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += self.dir_size_impl(&entry.path()).await?;
                }
            }

            Ok(size)
        })
    }
}

/// Information about data layer
#[derive(Debug, Clone)]
pub struct DataLayerInfo {
    pub base_path: PathBuf,
    pub umd_source_path: PathBuf,
    pub generated_path: PathBuf,
    pub user_data_path: PathBuf,
    pub umd_source_bytes: u64,
    pub generated_bytes: u64,
    pub user_data_bytes: u64,
}

impl std::fmt::Display for DataLayerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataLayer {{\n  base: {},\n  umd_source: {} ({} bytes),\n  generated: {} ({} bytes),\n  user_data: {} ({} bytes)\n}}",
            self.base_path.display(),
            self.umd_source_path.display(),
            self.umd_source_bytes,
            self.generated_path.display(),
            self.generated_bytes,
            self.user_data_path.display(),
            self.user_data_bytes
        )
    }
}
