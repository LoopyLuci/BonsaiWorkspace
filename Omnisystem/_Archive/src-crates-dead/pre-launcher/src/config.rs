use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub version: String,
    pub log_level: String,
    pub max_concurrent_apps: usize,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            log_level: "info".to_string(),
            max_concurrent_apps: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LauncherConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.max_concurrent_apps, 100);
    }
}
