//! Bootstrap and initialization system for full-stack testing

use crate::errors::{FullStackTestError, FullStackTestResult};
use crate::vault::{BonsaiApplication, OmnisystemService, Vault, VaultConfig};
use serde::{Deserialize, Serialize};

/// Configuration for bootstrap process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// Vault configuration
    pub vault_config: VaultConfig,
    /// Whether to auto-initialize kernel
    pub auto_init_kernel: bool,
    /// Services to automatically register
    pub auto_services: Vec<String>,
    /// Applications to automatically register
    pub auto_applications: Vec<ApplicationBootstrapConfig>,
    /// Enable verbose logging during bootstrap
    pub verbose: bool,
}

/// Configuration for individual application bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationBootstrapConfig {
    pub name: String,
    pub language: String,
    pub initial_memory_mb: u64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            vault_config: VaultConfig::default(),
            auto_init_kernel: true,
            auto_services: vec![
                "SLM".to_string(),
                "Buddy".to_string(),
                "Workspace".to_string(),
                "Survival".to_string(),
            ],
            auto_applications: vec![
                ApplicationBootstrapConfig {
                    name: "python-runtime".to_string(),
                    language: "python".to_string(),
                    initial_memory_mb: 256,
                },
                ApplicationBootstrapConfig {
                    name: "rust-runtime".to_string(),
                    language: "rust".to_string(),
                    initial_memory_mb: 512,
                },
                ApplicationBootstrapConfig {
                    name: "javascript-runtime".to_string(),
                    language: "javascript".to_string(),
                    initial_memory_mb: 128,
                },
            ],
            verbose: false,
        }
    }
}

/// Bootstrap process for full-stack test environment
pub struct FullStackBootstrap {
    config: BootstrapConfig,
    initialized: bool,
}

impl FullStackBootstrap {
    /// Create new bootstrap with custom configuration
    pub fn with_config(config: BootstrapConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &BootstrapConfig {
        &self.config
    }

    /// Check if bootstrap has been completed
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initialize the complete system
    ///
    /// # Steps
    /// 1. Create vault container
    /// 2. Initialize UOSC kernel
    /// 3. Register Omnisystem services
    /// 4. Register Bonsai applications
    /// 5. Verify all components
    pub async fn initialize(mut self) -> FullStackTestResult<Vault> {
        if self.log_verbose("Creating vault container...") {
            println!("Creating vault container...");
        }
        let vault = Vault::new(self.config.vault_config.clone());

        if self.config.auto_init_kernel {
            if self.log_verbose("Initializing UOSC kernel...") {
                println!("Initializing UOSC kernel...");
            }
            vault.initialize_kernel().await?;
        }

        // Register services
        for service_name in &self.config.auto_services {
            if self.log_verbose(&format!("Registering service: {}", service_name)) {
                println!("Registering service: {}", service_name);
            }
            let service = OmnisystemService::new(service_name);
            vault.register_service(service).await?;
        }

        // Register applications
        for app_config in &self.config.auto_applications {
            if self.log_verbose(&format!("Registering application: {}", app_config.name)) {
                println!("Registering application: {}", app_config.name);
            }
            let mut app = BonsaiApplication::new(&app_config.name, &app_config.language);
            app.memory_usage = app_config.initial_memory_mb * 1024 * 1024;
            vault.register_application(app).await?;
        }

        if self.log_verbose("Verifying bootstrap...") {
            println!("Verifying bootstrap...");
        }
        self.verify_bootstrap(&vault).await?;

        self.initialized = true;

        if self.log_verbose("Bootstrap complete!") {
            println!("Bootstrap complete!");
        }

        Ok(vault)
    }

    /// Verify that all expected components are present
    async fn verify_bootstrap(&self, vault: &Vault) -> FullStackTestResult<()> {
        let kernel = vault.kernel_state().await?;
        if kernel.thread_count == 0 {
            return Err(FullStackTestError::bootstrap("Kernel initialization failed"));
        }

        let services = vault.all_services().await;
        if services.len() != self.config.auto_services.len() {
            return Err(FullStackTestError::bootstrap(format!(
                "Service registration mismatch: expected {}, got {}",
                self.config.auto_services.len(),
                services.len()
            )));
        }

        let apps = vault.all_applications().await;
        if apps.len() != self.config.auto_applications.len() {
            return Err(FullStackTestError::bootstrap(format!(
                "Application registration mismatch: expected {}, got {}",
                self.config.auto_applications.len(),
                apps.len()
            )));
        }

        Ok(())
    }

    /// Log message if verbose mode is enabled
    fn log_verbose(&self, msg: &str) -> bool {
        self.config.verbose && {
            println!("[BOOTSTRAP] {}", msg);
            true
        }
    }
}

impl Default for FullStackBootstrap {
    fn default() -> Self {
        Self::with_config(BootstrapConfig::default())
    }
}

/// Builder for customized bootstrap configuration
pub struct BootstrapBuilder {
    config: BootstrapConfig,
}

impl BootstrapBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            config: BootstrapConfig::default(),
        }
    }

    /// Set kernel thread count
    pub fn kernel_threads(mut self, threads: u32) -> Self {
        self.config.vault_config.kernel_threads = threads;
        self
    }

    /// Set maximum services
    pub fn max_services(mut self, max: usize) -> Self {
        self.config.vault_config.max_services = max;
        self
    }

    /// Set maximum applications
    pub fn max_applications(mut self, max: usize) -> Self {
        self.config.vault_config.max_applications = max;
        self
    }

    /// Set maximum memory
    pub fn max_memory_mb(mut self, mb: u64) -> Self {
        self.config.vault_config.max_memory_mb = mb;
        self
    }

    /// Add custom service
    pub fn add_service(mut self, name: impl Into<String>) -> Self {
        self.config.auto_services.push(name.into());
        self
    }

    /// Add custom application
    pub fn add_application(
        mut self,
        name: impl Into<String>,
        language: impl Into<String>,
        memory_mb: u64,
    ) -> Self {
        self.config.auto_applications.push(ApplicationBootstrapConfig {
            name: name.into(),
            language: language.into(),
            initial_memory_mb: memory_mb,
        });
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.config.verbose = enabled;
        self
    }

    /// Build bootstrap
    pub fn build(self) -> FullStackBootstrap {
        FullStackBootstrap::with_config(self.config)
    }
}

impl Default for BootstrapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bootstrap_config() {
        let config = BootstrapConfig::default();
        assert!(config.auto_init_kernel);
        assert_eq!(config.auto_services.len(), 4);
        assert_eq!(config.auto_applications.len(), 3);
    }

    #[tokio::test]
    async fn test_bootstrap_initialization() {
        let bootstrap = FullStackBootstrap::default();
        let vault = bootstrap.initialize().await.unwrap();

        let services = vault.all_services().await;
        assert_eq!(services.len(), 4);

        let apps = vault.all_applications().await;
        assert_eq!(apps.len(), 3);
    }

    #[tokio::test]
    async fn test_bootstrap_builder() {
        let bootstrap = BootstrapBuilder::new()
            .kernel_threads(4)
            .add_service("CustomService")
            .add_application("custom-app", "python", 256)
            .verbose(false)
            .build();

        let vault = bootstrap.initialize().await.unwrap();
        let services = vault.all_services().await;
        assert!(services.len() > 4); // Default 4 + custom

        let apps = vault.all_applications().await;
        assert!(apps.len() > 3); // Default 3 + custom
    }

    #[tokio::test]
    async fn test_bootstrap_verification() {
        let bootstrap = FullStackBootstrap::default();
        let vault = bootstrap.initialize().await.unwrap();

        let kernel = vault.kernel_state().await.unwrap();
        assert!(kernel.thread_count > 0);

        let services = vault.all_services().await;
        assert!(!services.is_empty());

        let apps = vault.all_applications().await;
        assert!(!apps.is_empty());
    }

    #[test]
    fn test_bootstrap_builder_fluent() {
        let bootstrap = BootstrapBuilder::new()
            .kernel_threads(8)
            .max_services(100)
            .max_applications(200)
            .verbose(true)
            .build();

        assert_eq!(bootstrap.config().vault_config.kernel_threads, 8);
        assert_eq!(bootstrap.config().vault_config.max_services, 100);
        assert_eq!(bootstrap.config().verbose, true);
    }
}
