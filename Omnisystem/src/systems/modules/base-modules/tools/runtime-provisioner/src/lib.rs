//! Omnisystem Runtime Provisioner
//!
//! Self-contained runtime provisioning system that automatically downloads,
//! verifies, and caches language-specific toolchains (Rust, Python, Go, Java, etc.)
//! enabling 100% independent operation with zero external dependencies.

pub mod error;
pub mod provisioner;
pub mod runtimes;
pub mod cache;
pub mod platform;

pub use error::{ProvisionerError, Result};
pub use provisioner::{RuntimeProvisioner, ProvisionerConfig};
pub use runtimes::{Runtime, RuntimeType, RuntimeVersion};
pub use cache::{Cache, CacheManager};
pub use platform::{Platform, Architecture, detect_platform};

use std::path::PathBuf;

/// Get the default provisioner with auto-detection
pub fn default_provisioner() -> Result<RuntimeProvisioner> {
    let config = ProvisionerConfig::default();
    RuntimeProvisioner::new(config)
}

/// Get provisioner with custom cache directory
pub fn provisioner_with_cache(cache_dir: PathBuf) -> Result<RuntimeProvisioner> {
    let config = ProvisionerConfig {
        cache_dir,
        ..Default::default()
    };
    RuntimeProvisioner::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provisioner_creation() {
        let result = default_provisioner();
        assert!(result.is_ok());
    }
}
