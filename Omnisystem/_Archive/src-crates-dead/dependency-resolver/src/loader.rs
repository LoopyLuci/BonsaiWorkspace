//! Lazy module loading system

use crate::{Dependency, Result};
use std::fs;

pub struct LazyModuleLoader {
    cache_dir: String,
}

impl LazyModuleLoader {
    pub fn new(cache_dir: &str) -> Result<Self> {
        fs::create_dir_all(cache_dir).ok();
        Ok(Self {
            cache_dir: cache_dir.to_string(),
        })
    }

    /// Load modules
    pub async fn load_modules(&self, dependencies: &[Dependency]) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        for dep in dependencies {
            log::info!("Loading module: {} v{}", dep.module.name, dep.module.version);
            loaded.push(dep.module.name.clone());

            // Simulate module loading
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleId;

    #[tokio::test]
    async fn test_loader_creation() -> Result<()> {
        let loader = LazyModuleLoader::new(".omnisystem/test")?;
        assert!(!loader.cache_dir.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_load_modules() -> Result<()> {
        let loader = LazyModuleLoader::new(".omnisystem/test2")?;
        let deps = vec![Dependency {
            module: ModuleId {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            required_version: "1.0.0".to_string(),
            optional: false,
        }];

        let loaded = loader.load_modules(&deps).await?;
        assert!(!loaded.is_empty());
        Ok(())
    }
}
