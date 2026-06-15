//! Vault: Unified container for UOSC kernel, Omnisystem services, and Bonsai applications

use crate::errors::{FullStackTestError, FullStackTestResult};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Health status of a system component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentHealth {
    /// Component is running normally
    Healthy,
    /// Component is degraded but operational
    Degraded,
    /// Component has failed
    Failed,
    /// Component is recovering
    Recovering,
    /// Component status is unknown
    Unknown,
}

impl ComponentHealth {
    /// Is the component operational?
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    /// Is the component failed?
    pub fn is_failed(&self) -> bool {
        *self == Self::Failed
    }
}

/// UOSC Kernel component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UoscKernel {
    pub id: String,
    pub health: ComponentHealth,
    pub thread_count: u32,
    pub memory_usage: u64,
    pub context_switches: u64,
}

impl Default for UoscKernel {
    fn default() -> Self {
        Self {
            id: format!("uosc-kernel-{}", Uuid::new_v4()),
            health: ComponentHealth::Healthy,
            thread_count: 1,
            memory_usage: 0,
            context_switches: 0,
        }
    }
}

/// Omnisystem service component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnisystemService {
    pub id: String,
    pub name: String,
    pub health: ComponentHealth,
    pub uptime_secs: u64,
    pub request_count: u64,
    pub error_count: u64,
}

impl OmnisystemService {
    /// Create a new service
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: format!("service-{}", Uuid::new_v4()),
            name: name.into(),
            health: ComponentHealth::Healthy,
            uptime_secs: 0,
            request_count: 0,
            error_count: 0,
        }
    }

    /// Error rate as percentage
    pub fn error_rate(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            (self.error_count as f64 / self.request_count as f64) * 100.0
        }
    }
}

/// Bonsai application instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonsaiApplication {
    pub id: String,
    pub name: String,
    pub language: String,
    pub health: ComponentHealth,
    pub pid: Option<u32>,
    pub memory_usage: u64,
    pub execution_count: u64,
    pub error_count: u64,
}

impl BonsaiApplication {
    /// Create a new application
    pub fn new(name: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            id: format!("app-{}", Uuid::new_v4()),
            name: name.into(),
            language: language.into(),
            health: ComponentHealth::Healthy,
            pid: None,
            memory_usage: 0,
            execution_count: 0,
            error_count: 0,
        }
    }

    /// Success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            100.0
        } else {
            ((self.execution_count - self.error_count) as f64 / self.execution_count as f64) * 100.0
        }
    }
}

/// Configuration for vault initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub kernel_threads: u32,
    pub max_services: usize,
    pub max_applications: usize,
    pub max_memory_mb: u64,
    pub enable_persistence: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            kernel_threads: num_cpus::get() as u32,
            max_services: 50,
            max_applications: 100,
            max_memory_mb: 8192,
            enable_persistence: true,
        }
    }
}

/// Event log entry for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub component_id: String,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub context: HashMap<String, String>,
}

/// Vault: Unified container for all system components
pub struct Vault {
    id: String,
    config: VaultConfig,
    kernel: Arc<RwLock<UoscKernel>>,
    services: Arc<DashMap<String, OmnisystemService>>,
    applications: Arc<DashMap<String, BonsaiApplication>>,
    audit_log: Arc<RwLock<Vec<AuditEvent>>>,
    event_counter: Arc<AtomicU64>,
}

impl Vault {
    /// Create a new vault with configuration
    pub fn new(config: VaultConfig) -> Self {
        Self {
            id: format!("vault-{}", Uuid::new_v4()),
            config,
            kernel: Arc::new(RwLock::new(UoscKernel::default())),
            services: Arc::new(DashMap::new()),
            applications: Arc::new(DashMap::new()),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            event_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get vault ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Initialize all kernel threads
    pub async fn initialize_kernel(&self) -> FullStackTestResult<()> {
        let mut kernel = self.kernel.write().await;
        kernel.thread_count = self.config.kernel_threads;

        self.log_event("KERNEL_INIT".to_string(), "info", "Kernel initialized").await;
        Ok(())
    }

    /// Register a service in the vault
    pub async fn register_service(&self, service: OmnisystemService) -> FullStackTestResult<()> {
        if self.services.len() >= self.config.max_services {
            return Err(FullStackTestError::ResourceExhaustion(format!(
                "Service limit reached: {}",
                self.config.max_services
            )));
        }

        let service_id = service.id.clone();
        self.services.insert(service.id.clone(), service.clone());

        self.log_event(
            service_id,
            "info",
            format!("Service registered: {}", service.name),
        )
        .await;

        Ok(())
    }

    /// Register an application in the vault
    pub async fn register_application(
        &self,
        app: BonsaiApplication,
    ) -> FullStackTestResult<()> {
        if self.applications.len() >= self.config.max_applications {
            return Err(FullStackTestError::ResourceExhaustion(format!(
                "Application limit reached: {}",
                self.config.max_applications
            )));
        }

        let app_id = app.id.clone();
        self.applications.insert(app.id.clone(), app.clone());

        self.log_event(
            app_id,
            "info",
            format!("Application registered: {} ({})", app.name, app.language),
        )
        .await;

        Ok(())
    }

    /// Update component health status
    pub async fn set_component_health(
        &self,
        component_id: &str,
        health: ComponentHealth,
    ) -> FullStackTestResult<()> {
        // Try services first
        if let Some(mut service) = self.services.get_mut(component_id) {
            service.health = health;
            self.log_event(
                component_id.to_string(),
                "warning",
                format!("Health status changed: {:?}", health),
            )
            .await;
            return Ok(());
        }

        // Try applications
        if let Some(mut app) = self.applications.get_mut(component_id) {
            app.health = health;
            self.log_event(
                component_id.to_string(),
                "warning",
                format!("Health status changed: {:?}", health),
            )
            .await;
            return Ok(());
        }

        Err(FullStackTestError::ExecutionError(format!(
            "Component not found: {}",
            component_id
        )))
    }

    /// Get current kernel state
    pub async fn kernel_state(&self) -> FullStackTestResult<UoscKernel> {
        Ok(self.kernel.read().await.clone())
    }

    /// Get service by ID
    pub async fn get_service(&self, service_id: &str) -> FullStackTestResult<OmnisystemService> {
        self.services
            .get(service_id)
            .map(|s| s.clone())
            .ok_or_else(|| {
                FullStackTestError::ExecutionError(format!("Service not found: {}", service_id))
            })
    }

    /// Get application by ID
    pub async fn get_application(
        &self,
        app_id: &str,
    ) -> FullStackTestResult<BonsaiApplication> {
        self.applications
            .get(app_id)
            .map(|a| a.clone())
            .ok_or_else(|| {
                FullStackTestError::ExecutionError(format!("Application not found: {}", app_id))
            })
    }

    /// Get all services
    pub async fn all_services(&self) -> Vec<OmnisystemService> {
        self.services.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Get all applications
    pub async fn all_applications(&self) -> Vec<BonsaiApplication> {
        self.applications.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Record audit event
    async fn log_event(&self, component_id: String, severity: &str, message: impl Into<String>) {
        let event = AuditEvent {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            component_id,
            event_type: "GENERIC".to_string(),
            severity: severity.to_string(),
            message: message.into(),
            context: HashMap::new(),
        };

        let mut log = self.audit_log.write().await;
        log.push(event);
        self.event_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Get audit log
    pub async fn audit_log(&self) -> Vec<AuditEvent> {
        self.audit_log.read().await.clone()
    }

    /// Get total event count
    pub fn event_count(&self) -> u64 {
        self.event_counter.load(Ordering::SeqCst)
    }

    /// Snapshot vault state
    pub async fn snapshot(&self) -> VaultSnapshot {
        VaultSnapshot {
            vault_id: self.id.clone(),
            kernel: self.kernel.read().await.clone(),
            services: self.all_services().await,
            applications: self.all_applications().await,
            event_count: self.event_count(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Restore vault from snapshot
    pub async fn restore(&self, snapshot: &VaultSnapshot) -> FullStackTestResult<()> {
        let mut kernel = self.kernel.write().await;
        *kernel = snapshot.kernel.clone();

        self.services.clear();
        for service in &snapshot.services {
            self.services.insert(service.id.clone(), service.clone());
        }

        self.applications.clear();
        for app in &snapshot.applications {
            self.applications.insert(app.id.clone(), app.clone());
        }

        self.log_event("VAULT".to_string(), "info", "Restored from snapshot").await;
        Ok(())
    }
}

/// Point-in-time snapshot of vault state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSnapshot {
    pub vault_id: String,
    pub kernel: UoscKernel,
    pub services: Vec<OmnisystemService>,
    pub applications: Vec<BonsaiApplication>,
    pub event_count: u64,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vault_creation() {
        let vault = Vault::new(VaultConfig::default());
        assert!(!vault.id().is_empty());
    }

    #[tokio::test]
    async fn test_kernel_initialization() {
        let vault = Vault::new(VaultConfig::default());
        vault.initialize_kernel().await.unwrap();

        let kernel = vault.kernel_state().await.unwrap();
        assert!(kernel.thread_count > 0);
    }

    #[tokio::test]
    async fn test_service_registration() {
        let vault = Vault::new(VaultConfig::default());
        let service = OmnisystemService::new("test-service");
        vault.register_service(service.clone()).await.unwrap();

        let retrieved = vault.get_service(&service.id).await.unwrap();
        assert_eq!(retrieved.name, "test-service");
    }

    #[tokio::test]
    async fn test_application_registration() {
        let vault = Vault::new(VaultConfig::default());
        let app = BonsaiApplication::new("test-app", "rust");
        vault.register_application(app.clone()).await.unwrap();

        let retrieved = vault.get_application(&app.id).await.unwrap();
        assert_eq!(retrieved.name, "test-app");
        assert_eq!(retrieved.language, "rust");
    }

    #[tokio::test]
    async fn test_health_status_update() {
        let vault = Vault::new(VaultConfig::default());
        let service = OmnisystemService::new("test");
        vault.register_service(service.clone()).await.unwrap();

        vault.set_component_health(&service.id, ComponentHealth::Degraded).await.unwrap();
        let updated = vault.get_service(&service.id).await.unwrap();
        assert_eq!(updated.health, ComponentHealth::Degraded);
    }

    #[tokio::test]
    async fn test_vault_snapshot() {
        let vault = Vault::new(VaultConfig::default());
        vault.initialize_kernel().await.unwrap();

        let service = OmnisystemService::new("test");
        vault.register_service(service).await.unwrap();

        let snapshot = vault.snapshot().await;
        assert_eq!(snapshot.services.len(), 1);
    }

    #[tokio::test]
    async fn test_vault_restore() {
        let vault = Vault::new(VaultConfig::default());
        vault.initialize_kernel().await.unwrap();

        let service = OmnisystemService::new("test");
        vault.register_service(service).await.unwrap();

        let snapshot = vault.snapshot().await;

        let vault2 = Vault::new(VaultConfig::default());
        vault2.restore(&snapshot).await.unwrap();

        let services = vault2.all_services().await;
        assert_eq!(services.len(), 1);
    }

    #[test]
    fn test_component_health() {
        assert!(ComponentHealth::Healthy.is_operational());
        assert!(ComponentHealth::Degraded.is_operational());
        assert!(!ComponentHealth::Failed.is_operational());
        assert!(ComponentHealth::Failed.is_failed());
    }

    #[test]
    fn test_service_error_rate() {
        let mut service = OmnisystemService::new("test");
        service.request_count = 100;
        service.error_count = 10;

        let rate = service.error_rate();
        assert!((rate - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_application_success_rate() {
        let mut app = BonsaiApplication::new("test", "rust");
        app.execution_count = 100;
        app.error_count = 25;

        let rate = app.success_rate();
        assert!((rate - 75.0).abs() < 0.01);
    }
}
