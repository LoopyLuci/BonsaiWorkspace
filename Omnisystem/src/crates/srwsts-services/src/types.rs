//! Core types for service stress testing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Unique service identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(String);

impl ServiceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique test case identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestCaseId(String);

impl TestCaseId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TestCaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is initializing
    Initializing,
    /// Service is running normally
    Running,
    /// Service is degraded but operational
    Degraded,
    /// Service has failed
    Failed,
    /// Service is stopped
    Stopped,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initializing => write!(f, "Initializing"),
            Self::Running => write!(f, "Running"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Failed => write!(f, "Failed"),
            Self::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Omnisystem core service types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// P2P mesh networking
    P2P,
    /// Content-addressed storage
    Storage,
    /// Network stack
    Network,
    /// GPU compositor
    Compositor,
    /// Service discovery and DNS
    ServiceDiscovery,
    /// Message relay and routing
    MessageRelay,
    /// Consensus and distributed state
    Consensus,
    /// Cryptographic operations
    Cryptography,
    /// Resource management
    ResourceManager,
    /// Custom service
    Custom,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P2P => write!(f, "P2P"),
            Self::Storage => write!(f, "Storage"),
            Self::Network => write!(f, "Network"),
            Self::Compositor => write!(f, "Compositor"),
            Self::ServiceDiscovery => write!(f, "ServiceDiscovery"),
            Self::MessageRelay => write!(f, "MessageRelay"),
            Self::Consensus => write!(f, "Consensus"),
            Self::Cryptography => write!(f, "Cryptography"),
            Self::ResourceManager => write!(f, "ResourceManager"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// Service instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: ServiceId,
    pub service_type: ServiceType,
    pub status: ServiceStatus,
    pub started_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Service {
    pub fn new(id: impl Into<String>, service_type: ServiceType) -> Self {
        Self {
            id: ServiceId::new(id),
            service_type,
            status: ServiceStatus::Initializing,
            started_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, ServiceStatus::Running)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self.status, ServiceStatus::Degraded)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, ServiceStatus::Failed)
    }
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Maximum duration for test execution
    pub timeout: Duration,
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Operations per second target
    pub ops_per_sec: u64,
    /// Enable detailed logging
    pub enable_logging: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Deterministic mode (reproducible failures)
    pub deterministic: bool,
    /// Custom parameters
    pub parameters: HashMap<String, String>,
}

impl TestConfig {
    pub fn new(timeout: Duration, concurrency: usize) -> Self {
        Self {
            timeout,
            concurrency,
            ops_per_sec: 1000,
            enable_logging: true,
            enable_metrics: true,
            deterministic: false,
            parameters: HashMap::new(),
        }
    }

    pub fn with_ops_per_sec(mut self, ops: u64) -> Self {
        self.ops_per_sec = ops;
        self
    }

    pub fn with_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            concurrency: 10,
            ops_per_sec: 1000,
            enable_logging: true,
            enable_metrics: true,
            deterministic: false,
            parameters: HashMap::new(),
        }
    }
}

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestResultStatus {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test timed out
    TimedOut,
    /// Test had errors
    Error,
}

impl std::fmt::Display for TestResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passed => write!(f, "Passed"),
            Self::Failed => write!(f, "Failed"),
            Self::Skipped => write!(f, "Skipped"),
            Self::TimedOut => write!(f, "TimedOut"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: TestCaseId,
    pub status: TestResultStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub message: Option<String>,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
}

impl TestResult {
    pub fn new(test_id: impl Into<String>, status: TestResultStatus) -> Self {
        let now = Utc::now();
        Self {
            test_id: TestCaseId::new(test_id),
            status,
            started_at: now,
            completed_at: now,
            duration_ms: 0,
            message: None,
            assertions_passed: 0,
            assertions_failed: 0,
        }
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    pub fn with_assertions(mut self, passed: usize, failed: usize) -> Self {
        self.assertions_passed = passed;
        self.assertions_failed = failed;
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TestResultStatus::Passed)
    }
}

/// Dependency relationship between services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    pub service_id: ServiceId,
    pub depends_on: ServiceId,
    pub optional: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = Service::new("svc-1", ServiceType::P2P);
        assert_eq!(service.id.as_str(), "svc-1");
        assert_eq!(service.service_type, ServiceType::P2P);
        assert!(!service.is_healthy());
    }

    #[test]
    fn test_service_health() {
        let mut service = Service::new("svc-1", ServiceType::Storage);
        assert!(!service.is_healthy());
        service.status = ServiceStatus::Running;
        assert!(service.is_healthy());
    }

    #[test]
    fn test_test_config_default() {
        let config = TestConfig::default();
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.ops_per_sec, 1000);
    }

    #[test]
    fn test_test_result() {
        let result = TestResult::new("test-1", TestResultStatus::Passed);
        assert!(result.is_success());
    }
}
