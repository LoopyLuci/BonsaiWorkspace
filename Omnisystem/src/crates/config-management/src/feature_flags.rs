use crate::{ConfigError, ConfigResult, FeatureFlag};
use dashmap::DashMap;
use std::sync::Arc;

pub struct FeatureFlagManager {
    flags: Arc<DashMap<String, FeatureFlag>>,
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        Self {
            flags: Arc::new(DashMap::new()),
        }
    }

    pub async fn enable_flag(&self, flag: &FeatureFlag) -> ConfigResult<()> {
        self.flags.insert(flag.flag_id.clone(), flag.clone());
        Ok(())
    }

    pub async fn is_enabled(&self, flag_id: &str) -> ConfigResult<bool> {
        if let Some(flag) = self.flags.get(flag_id) {
            Ok(flag.enabled)
        } else {
            Err(ConfigError::FeatureFlagNotFound)
        }
    }

    pub fn flag_count(&self) -> usize {
        self.flags.len()
    }
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enable_flag() {
        let mgr = FeatureFlagManager::new();
        let flag = FeatureFlag {
            flag_id: "new_ui".to_string(),
            enabled: true,
            percentage: 50,
        };

        mgr.enable_flag(&flag).await.unwrap();
        assert_eq!(mgr.flag_count(), 1);
    }

    #[tokio::test]
    async fn test_is_enabled() {
        let mgr = FeatureFlagManager::new();
        let flag = FeatureFlag {
            flag_id: "new_ui".to_string(),
            enabled: true,
            percentage: 50,
        };

        mgr.enable_flag(&flag).await.unwrap();
        let enabled = mgr.is_enabled("new_ui").await.unwrap();
        assert!(enabled);
    }
}
