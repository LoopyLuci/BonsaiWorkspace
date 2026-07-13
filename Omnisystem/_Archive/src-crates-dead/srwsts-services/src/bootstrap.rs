//! Service bootstrap and lifecycle management
//!
//! Initializes and manages Omnisystem core services without UOSC kernel or applications.

use crate::types::{Service, ServiceId, ServiceStatus, ServiceType, TestConfig};
use crate::{
    CompositorStressTests, NetworkStressTests, P2PStressTests, ServiceDiscoveryTests,
    ServiceError, ServiceInteractionTests, ServiceMetricsCollector, ServiceResult, StorageStressTests,
    FaultScenarioTests, TestReport,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Configuration for service bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBootstrapConfig {
    /// Services to initialize
    pub services: Vec<ServiceType>,
    /// Timeout for service startup
    pub startup_timeout: Duration,
    /// Test configuration for stress tests
    pub test_config: TestConfig,
    /// Enable fault injection tests
    pub enable_faults: bool,
    /// Services to exclude from testing
    pub exclude_services: Vec<ServiceType>,
}

impl ServiceBootstrapConfig {
    pub fn new(services: Vec<ServiceType>) -> Self {
        Self {
            services,
            startup_timeout: Duration::from_secs(60),
            test_config: TestConfig::default(),
            enable_faults: true,
            exclude_services: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_test_config(mut self, config: TestConfig) -> Self {
        self.test_config = config;
        self
    }

    pub fn with_faults(mut self, enable: bool) -> Self {
        self.enable_faults = enable;
        self
    }
}

impl Default for ServiceBootstrapConfig {
    fn default() -> Self {
        Self {
            services: vec![
                ServiceType::P2P,
                ServiceType::Storage,
                ServiceType::Network,
                ServiceType::Compositor,
                ServiceType::ServiceDiscovery,
                ServiceType::MessageRelay,
                ServiceType::Consensus,
            ],
            startup_timeout: Duration::from_secs(60),
            test_config: TestConfig::default(),
            enable_faults: true,
            exclude_services: Vec::new(),
        }
    }
}

/// Service bootstrap manager
pub struct ServiceBootstrap {
    config: ServiceBootstrapConfig,
    services: Arc<RwLock<HashMap<ServiceId, Service>>>,
    metrics: Arc<DashMap<String, ServiceMetricsCollector>>,
    test_results: Arc<RwLock<Vec<TestReport>>>,
}

impl ServiceBootstrap {
    /// Create a new service bootstrap with default configuration
    pub fn new(config: ServiceBootstrapConfig) -> Self {
        Self {
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(DashMap::new()),
            test_results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize all configured services
    pub async fn initialize(&mut self) -> ServiceResult<()> {
        info!("Initializing Omnisystem services...");

        let mut services = self.services.write().await;

        for service_type in &self.config.services {
            if self.config.exclude_services.contains(service_type) {
                debug!("Skipping excluded service: {}", service_type);
                continue;
            }

            let service_id = self.generate_service_id(*service_type);
            let mut service = Service::new(&service_id, *service_type);

            // Simulate service initialization
            tokio::time::sleep(Duration::from_millis(100)).await;

            service.status = ServiceStatus::Running;
            services.insert(service.id.clone(), service.clone());

            // Initialize metrics collector
            self.metrics.insert(
                service_id.clone(),
                ServiceMetricsCollector::new(&service_id),
            );

            info!("Initialized service: {} ({})", service_id, service_type);
        }

        Ok(())
    }

    /// Run all stress tests
    pub async fn run_all_tests(&self) -> ServiceResult<()> {
        info!("Starting comprehensive service stress testing...");

        let services = self.services.read().await;
        let mut results = Vec::new();

        // P2P stress tests
        if self.has_service(&services, ServiceType::P2P) {
            info!("Running P2P stress tests...");
            let p2p_tests = P2PStressTests::new(self.config.test_config.clone());
            if let Ok(report) = p2p_tests.run_all_tests().await {
                results.push(report);
            }
        }

        // Storage stress tests
        if self.has_service(&services, ServiceType::Storage) {
            info!("Running Storage stress tests...");
            let storage_tests = StorageStressTests::new(self.config.test_config.clone());
            if let Ok(report) = storage_tests.run_all_tests().await {
                results.push(report);
            }
        }

        // Network stress tests
        if self.has_service(&services, ServiceType::Network) {
            info!("Running Network stress tests...");
            let network_tests = NetworkStressTests::new(self.config.test_config.clone());
            if let Ok(report) = network_tests.run_all_tests().await {
                results.push(report);
            }
        }

        // Compositor stress tests
        if self.has_service(&services, ServiceType::Compositor) {
            info!("Running Compositor stress tests...");
            let compositor_tests = CompositorStressTests::new(self.config.test_config.clone());
            if let Ok(report) = compositor_tests.run_all_tests().await {
                results.push(report);
            }
        }

        // Service Discovery tests
        if self.has_service(&services, ServiceType::ServiceDiscovery) {
            info!("Running Service Discovery tests...");
            let discovery_tests = ServiceDiscoveryTests::new(self.config.test_config.clone());
            if let Ok(report) = discovery_tests.run_all_tests().await {
                results.push(report);
            }
        }

        // Service Interaction tests
        info!("Running Service Interaction tests...");
        let interaction_tests = ServiceInteractionTests::new(self.config.test_config.clone());
        if let Ok(report) = interaction_tests.run_all_tests().await {
            results.push(report);
        }

        // Fault Scenario tests (if enabled)
        if self.config.enable_faults {
            info!("Running Fault Scenario tests...");
            let fault_tests = FaultScenarioTests::new(self.config.test_config.clone());
            if let Ok(report) = fault_tests.run_all_tests().await {
                results.push(report);
            }
        }

        let mut test_results = self.test_results.write().await;
        test_results.extend(results);

        info!("Completed all stress tests");
        Ok(())
    }

    /// Get metrics for a specific service
    pub fn get_service_metrics(&self, service_id: &str) -> Option<crate::ServiceMetrics> {
        self.metrics.get(service_id).map(|collector| collector.aggregate())
    }

    /// Generate a comprehensive test report
    pub async fn generate_report(&self) -> ServiceResult<TestReport> {
        let services = self.services.read().await;
        let test_reports = self.test_results.read().await;

        let mut service_metrics = HashMap::new();
        for service in services.values() {
            if let Some(metrics) = self.get_service_metrics(service.id.as_str()) {
                service_metrics.insert(service.id.to_string(), metrics);
            }
        }

        let mut report = TestReport::new();
        // Extract all test results from individual reports
        for test_report in test_reports.iter() {
            report.test_results.extend(test_report.test_results.clone());
        }
        report.service_metrics = service_metrics;

        // Calculate summary statistics
        let total_tests = report.test_results.len();
        let passed = report.test_results.iter().filter(|r| r.is_success()).count();
        report.total_tests = total_tests;
        report.passed_tests = passed;
        report.failed_tests = total_tests - passed;
        report.success_rate = if total_tests > 0 {
            (passed as f64 / total_tests as f64) * 100.0
        } else {
            100.0
        };

        Ok(report)
    }

    /// Shutdown all services
    pub async fn shutdown(&mut self) -> ServiceResult<()> {
        info!("Shutting down services...");

        let mut services = self.services.write().await;
        for service in services.values_mut() {
            service.status = ServiceStatus::Stopped;
            debug!("Stopped service: {}", service.id);
        }

        self.metrics.clear();
        Ok(())
    }

    /// Check if a specific service type is available
    fn has_service(
        &self,
        services: &HashMap<ServiceId, Service>,
        service_type: ServiceType,
    ) -> bool {
        services
            .values()
            .any(|s| s.service_type == service_type && s.is_healthy())
    }

    /// Generate a unique service ID based on service type
    fn generate_service_id(&self, service_type: ServiceType) -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("{}-{}", service_type, timestamp)
    }

    /// Get all running services
    pub async fn get_services(&self) -> ServiceResult<Vec<Service>> {
        let services = self.services.read().await;
        Ok(services.values().cloned().collect())
    }

    /// Get service status by ID
    pub async fn get_service_status(&self, service_id: &str) -> ServiceResult<ServiceStatus> {
        let services = self.services.read().await;
        for service in services.values() {
            if service.id.as_str() == service_id {
                return Ok(service.status);
            }
        }
        Err(ServiceError::ServiceNotFound {
            service_name: service_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bootstrap_creation() {
        let config = ServiceBootstrapConfig::default();
        let bootstrap = ServiceBootstrap::new(config);
        assert!(!bootstrap.config.services.is_empty());
    }

    #[tokio::test]
    async fn test_bootstrap_initialization() {
        let config = ServiceBootstrapConfig::new(vec![ServiceType::P2P, ServiceType::Storage]);
        let mut bootstrap = ServiceBootstrap::new(config);
        assert!(bootstrap.initialize().await.is_ok());

        let services = bootstrap.get_services().await.unwrap();
        assert_eq!(services.len(), 2);
    }

    #[tokio::test]
    async fn test_service_status_lookup() {
        let config = ServiceBootstrapConfig::new(vec![ServiceType::P2P]);
        let mut bootstrap = ServiceBootstrap::new(config);
        bootstrap.initialize().await.unwrap();

        let services = bootstrap.get_services().await.unwrap();
        let service_id = services.first().unwrap().id.as_str();
        let status = bootstrap.get_service_status(service_id).await.unwrap();
        assert_eq!(status, ServiceStatus::Running);
    }

    #[tokio::test]
    async fn test_service_not_found() {
        let config = ServiceBootstrapConfig::default();
        let bootstrap = ServiceBootstrap::new(config);
        let result = bootstrap.get_service_status("non-existent").await;
        assert!(result.is_err());
    }
}
