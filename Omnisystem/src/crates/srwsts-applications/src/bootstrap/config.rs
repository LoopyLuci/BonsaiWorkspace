//! Bootstrap configuration

use std::path::PathBuf;

/// Configuration for bootstrapping Bonsai Ecosystem
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BootstrapConfig {
    /// Path to Omnisystem image
    pub omnisystem_image: PathBuf,

    /// Path to Bonsai Workspace image
    pub workspace_image: PathBuf,

    /// Path to Buddy agent image
    pub buddy_image: PathBuf,

    /// Path to Omni-Bot image
    pub omnibot_image: PathBuf,

    /// Timeout for bootstrap operations in seconds
    pub bootstrap_timeout_secs: u64,

    /// Maximum retries for failed components
    pub max_retries: usize,

    /// Enable health checks
    pub enable_health_checks: bool,

    /// Health check interval in milliseconds
    pub health_check_interval_ms: u64,

    /// Enable verbose bootstrap logging
    pub verbose: bool,

    /// Cleanup on failure
    pub cleanup_on_failure: bool,

    /// Parallel component initialization
    pub parallel_init: bool,

    /// Maximum parallel components
    pub max_parallel: usize,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            omnisystem_image: PathBuf::from("/var/lib/omnisystem/image.bin"),
            workspace_image: PathBuf::from("/var/lib/bonsai-ecosystem/workspace-image.bin"),
            buddy_image: PathBuf::from("/var/lib/bonsai-ecosystem/buddy-image.bin"),
            omnibot_image: PathBuf::from("/var/lib/bonsai-ecosystem/omnibot-image.bin"),
            bootstrap_timeout_secs: 120,
            max_retries: 3,
            enable_health_checks: true,
            health_check_interval_ms: 1000,
            verbose: false,
            cleanup_on_failure: true,
            parallel_init: true,
            max_parallel: 4,
        }
    }
}

impl BootstrapConfig {
    /// Create a new bootstrap configuration with custom image paths
    pub fn with_images(
        omnisystem: PathBuf,
        workspace: PathBuf,
        buddy: PathBuf,
        omnibot: PathBuf,
    ) -> Self {
        Self {
            omnisystem_image: omnisystem,
            workspace_image: workspace,
            buddy_image: buddy,
            omnibot_image: omnibot,
            ..Default::default()
        }
    }

    /// Set bootstrap timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.bootstrap_timeout_secs = secs;
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable parallel initialization
    pub fn with_parallel_init(mut self, parallel: bool, max_parallel: usize) -> Self {
        self.parallel_init = parallel;
        self.max_parallel = max_parallel;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BootstrapConfig::default();
        assert_eq!(config.bootstrap_timeout_secs, 120);
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_health_checks);
    }

    #[test]
    fn test_custom_config() {
        let config = BootstrapConfig::with_images(
            PathBuf::from("/custom/omnisystem"),
            PathBuf::from("/custom/workspace"),
            PathBuf::from("/custom/buddy"),
            PathBuf::from("/custom/omnibot"),
        );

        assert_eq!(config.omnisystem_image, PathBuf::from("/custom/omnisystem"));
        assert_eq!(config.workspace_image, PathBuf::from("/custom/workspace"));
    }

    #[test]
    fn test_builder_pattern() {
        let config = BootstrapConfig::default()
            .with_timeout(300)
            .with_max_retries(5)
            .with_verbose(true)
            .with_parallel_init(true, 8);

        assert_eq!(config.bootstrap_timeout_secs, 300);
        assert_eq!(config.max_retries, 5);
        assert!(config.verbose);
        assert!(config.parallel_init);
        assert_eq!(config.max_parallel, 8);
    }
}
