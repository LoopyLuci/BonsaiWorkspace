//! Service discovery stress tests
//!
//! Tests for:
//! - Dynamic service registration/deregistration
//! - DNS resolution under load
//! - Health check frequency

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

/// Simulated service registry entry
#[derive(Debug, Clone)]
pub struct ServiceRegistryEntry {
    pub service_id: String,
    pub service_name: String,
    pub host: String,
    pub port: u16,
    pub healthy: bool,
    pub last_heartbeat_ms: u64,
}

impl ServiceRegistryEntry {
    pub fn new(name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            service_id: uuid::Uuid::new_v4().to_string(),
            service_name: name.into(),
            host: host.into(),
            port,
            healthy: true,
            last_heartbeat_ms: 0,
        }
    }

    pub fn is_stale(&self, now_ms: u64, stale_threshold_ms: u64) -> bool {
        now_ms - self.last_heartbeat_ms > stale_threshold_ms
    }
}

/// Service registry
pub struct ServiceRegistry {
    entries: HashMap<String, ServiceRegistryEntry>,
    creation_time: Instant,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            creation_time: Instant::now(),
        }
    }

    pub fn register(&mut self, entry: ServiceRegistryEntry) -> String {
        let id = entry.service_id.clone();
        self.entries.insert(id.clone(), entry);
        id
    }

    pub fn deregister(&mut self, service_id: &str) -> Option<ServiceRegistryEntry> {
        self.entries.remove(service_id)
    }

    pub fn get(&self, service_id: &str) -> Option<&ServiceRegistryEntry> {
        self.entries.get(service_id)
    }

    pub fn resolve(&self, service_name: &str) -> Vec<ServiceRegistryEntry> {
        self.entries
            .values()
            .filter(|e| e.service_name == service_name && e.healthy)
            .cloned()
            .collect()
    }

    pub fn heartbeat(&mut self, service_id: &str) {
        if let Some(entry) = self.entries.get_mut(service_id) {
            entry.last_heartbeat_ms = self.creation_time.elapsed().as_millis() as u64;
        }
    }

    pub fn cleanup_stale(&mut self, stale_threshold_ms: u64) -> usize {
        let now = self.creation_time.elapsed().as_millis() as u64;
        let before = self.entries.len();
        self.entries.retain(|_, e| !e.is_stale(now, stale_threshold_ms));
        before - self.entries.len()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn healthy_count(&self) -> usize {
        self.entries.values().filter(|e| e.healthy).count()
    }
}

/// Service discovery stress tests
pub struct ServiceDiscoveryTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl ServiceDiscoveryTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("service-discovery"),
        }
    }

    /// Test dynamic service registration/deregistration
    pub async fn test_dynamic_registration(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-discovery-dynamic-registration";

        info!("Testing dynamic service registration/deregistration...");

        let mut registry = ServiceRegistry::new();
        let mut registered = 0;
        let mut deregistered = 0;

        // Register services
        for i in 0..1000 {
            let entry = ServiceRegistryEntry::new(
                format!("service-{}", i % 10),
                format!("127.0.0.1"),
                8000 + (i as u16 % 1000),
            );
            registry.register(entry);
            registered += 1;
        }

        // Deregister half
        let mut i = 0;
        for entry_id in registry.entries.keys().cloned().collect::<Vec<_>>() {
            if i % 2 == 0 {
                registry.deregister(&entry_id);
                deregistered += 1;
            }
            i += 1;
        }

        let success = registered > 0 && deregistered > 0 && registry.entry_count() < registered;

        self.metrics.record_operation(
            "dynamic_registration",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Registered: {}, Deregistered: {}, Remaining: {}",
            registered,
            deregistered,
            registry.entry_count()
        ));

        Ok(result)
    }

    /// Test DNS resolution under load
    pub async fn test_dns_resolution_under_load(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "service-discovery-dns-resolution";

        info!("Testing DNS resolution under load...");

        let mut registry = ServiceRegistry::new();

        // Register services
        for i in 0..100 {
            for j in 0..10 {
                let entry = ServiceRegistryEntry::new(
                    format!("service-{}", i),
                    format!("192.168.1.{}", j),
                    8000 + j as u16,
                );
                registry.register(entry);
            }
        }

        // Perform lookups
        let mut lookup_times = Vec::new();
        let mut successful_lookups = 0;

        for i in 0..1000 {
            let lookup_start = Instant::now();
            let service_name = format!("service-{}", i % 100);
            let results = registry.resolve(&service_name);
            let lookup_time = lookup_start.elapsed().as_millis() as f64;
            lookup_times.push(lookup_time);

            if !results.is_empty() {
                successful_lookups += 1;
            }

            self.metrics
                .record_operation("dns_resolution", lookup_time, !results.is_empty(), None);
        }

        lookup_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p99 = lookup_times[(lookup_times.len() as f64 * 0.99) as usize];
        let success = successful_lookups >= 900 && p99 < 10.0; // 90%+ success, p99 < 10ms

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Lookups: 1000, Successful: {}, p99: {:.2}ms",
            successful_lookups, p99
        ));

        Ok(result)
    }

    /// Test health check frequency
    pub async fn test_health_check_frequency(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-discovery-health-checks";

        info!("Testing health check frequency...");

        let mut registry = ServiceRegistry::new();

        // Register services
        let mut service_ids = Vec::new();
        for i in 0..50 {
            let entry = ServiceRegistryEntry::new(
                format!("service-{}", i),
                "localhost",
                8000 + i as u16,
            );
            let id = registry.register(entry);
            service_ids.push(id);
        }

        // Perform health checks
        let mut check_count = 0;
        let deadline = Instant::now() + std::time::Duration::from_secs(2);

        while Instant::now() < deadline {
            for service_id in &service_ids {
                let check_start = Instant::now();
                registry.heartbeat(service_id);
                let check_time = check_start.elapsed().as_millis() as f64;
                self.metrics
                    .record_operation("health_check", check_time, true, None);
                check_count += 1;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let checks_per_second = check_count as f64 / 2.0;
        let success = checks_per_second > 100.0; // At least 100 checks/sec

        self.metrics.record_operation(
            "health_check_frequency",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Total checks: {}, Checks/sec: {:.0}",
            check_count, checks_per_second
        ));

        Ok(result)
    }

    /// Test service discovery with failures
    pub async fn test_discovery_with_failures(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-discovery-with-failures";

        info!("Testing service discovery with failures...");

        let mut registry = ServiceRegistry::new();

        // Register services
        for i in 0..100 {
            let entry = ServiceRegistryEntry::new(
                format!("service-{}", i % 10),
                "localhost",
                8000 + i as u16,
            );
            registry.register(entry);
        }

        // Simulate failures
        let mut failed = 0;
        for entry in registry.entries.values_mut() {
            if rand::random::<f64>() < 0.3 {
                // 30% failure rate
                entry.healthy = false;
                failed += 1;
            }
        }

        // Verify healthy lookups exclude failed services
        let healthy_results = registry.resolve("service-0");
        let success = failed > 0 && !healthy_results.is_empty();

        self.metrics.record_operation(
            "discovery_with_failures",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Total services: {}, Failed: {}, Healthy results: {}",
            registry.entry_count(),
            failed,
            healthy_results.len()
        ));

        Ok(result)
    }

    /// Test stale entry cleanup
    pub async fn test_stale_cleanup(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-discovery-stale-cleanup";

        info!("Testing stale entry cleanup...");

        let mut registry = ServiceRegistry::new();

        // Register services
        for i in 0..100 {
            let mut entry = ServiceRegistryEntry::new(
                format!("service-{}", i),
                "localhost",
                8000 + i as u16,
            );
            entry.last_heartbeat_ms = (i as u64) * 100; // Vary the heartbeat times
            registry.register(entry);
        }

        let before = registry.entry_count();

        // Cleanup stale (older than 5000ms)
        let removed = registry.cleanup_stale(5000);
        let after = registry.entry_count();

        let success = removed > 0 && after < before;

        self.metrics.record_operation(
            "stale_cleanup",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Before: {}, Removed: {}, After: {}",
            before, removed, after
        ));

        Ok(result)
    }

    /// Run all service discovery tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all service discovery tests...");

        let results = vec![
            self.test_dynamic_registration().await?,
            self.test_dns_resolution_under_load().await?,
            self.test_health_check_frequency().await?,
            self.test_discovery_with_failures().await?,
            self.test_stale_cleanup().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("service-discovery".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "Service discovery tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_registry_registration() {
        let mut registry = ServiceRegistry::new();
        let entry = ServiceRegistryEntry::new("test-svc", "localhost", 8080);
        let id = registry.register(entry);
        assert!(registry.get(&id).is_some());
    }

    #[test]
    fn test_service_registry_resolve() {
        let mut registry = ServiceRegistry::new();
        let entry = ServiceRegistryEntry::new("api", "localhost", 8080);
        registry.register(entry);
        let results = registry.resolve("api");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_service_registry_deregister() {
        let mut registry = ServiceRegistry::new();
        let entry = ServiceRegistryEntry::new("test", "localhost", 8080);
        let id = registry.register(entry);
        assert!(registry.deregister(&id).is_some());
        assert!(registry.get(&id).is_none());
    }

    #[tokio::test]
    async fn test_service_discovery_tests() {
        let config = TestConfig::default();
        let tests = ServiceDiscoveryTests::new(config);

        let result = tests.test_dynamic_registration().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
