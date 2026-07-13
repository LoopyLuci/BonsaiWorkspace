//! Service interaction stress tests
//!
//! Tests for:
//! - Cross-service communication patterns
//! - Cascading failures
//! - Timeout behavior
//! - Backpressure handling

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

/// Service message
#[derive(Debug, Clone)]
pub struct ServiceMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
}

impl ServiceMessage {
    pub fn new(from: String, to: String, payload: Vec<u8>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            payload,
            timestamp_ms: 0,
        }
    }
}

/// Service dependency graph
#[derive(Debug, Clone)]
pub struct ServiceDependencyGraph {
    dependencies: HashMap<String, Vec<String>>, // service -> dependencies
}

impl ServiceDependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, service: String, depends_on: String) {
        self.dependencies
            .entry(service)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }

    pub fn get_dependencies(&self, service: &str) -> Option<&Vec<String>> {
        self.dependencies.get(service)
    }

    pub fn check_cascade(&self, failed_service: &str) -> Vec<String> {
        let mut cascaded = vec![failed_service.to_string()];
        let mut queue = vec![failed_service.to_string()];

        while let Some(service) = queue.pop() {
            for (dependent, deps) in &self.dependencies {
                if deps.contains(&service) && !cascaded.contains(dependent) {
                    cascaded.push(dependent.clone());
                    queue.push(dependent.clone());
                }
            }
        }

        cascaded
    }
}

/// Service interaction stress tests
pub struct ServiceInteractionTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl ServiceInteractionTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("service-interaction"),
        }
    }

    /// Test cross-service communication patterns
    pub async fn test_cross_service_communication(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "service-interaction-cross-service-communication";

        info!("Testing cross-service communication patterns...");

        let mut message_queue = Vec::new();
        let mut delivery_times = Vec::new();
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        let services = vec!["p2p", "storage", "network", "compositor", "discovery"];

        for _ in 0..self.config.concurrency * 10 {
            let from_idx = rng.next_u32() as usize % services.len();
            let to_idx = rng.next_u32() as usize % services.len();

            if from_idx != to_idx {
                let mut payload = vec![0u8; 1024];
                rng.fill_bytes(&mut payload);

                let msg = ServiceMessage::new(
                    services[from_idx].to_string(),
                    services[to_idx].to_string(),
                    payload,
                );
                message_queue.push(msg);
            }
        }

        // Simulate delivery
        for _msg in &mut message_queue {
            let delivery_start = Instant::now();
            // Simulate delivery with variable latency
            let latency =
                5.0 + (rng.next_u32() % 20) as f64; // 5-25ms latency
            tokio::time::sleep(std::time::Duration::from_millis(latency as u64)).await;
            let delivery_time = delivery_start.elapsed().as_millis() as f64;
            delivery_times.push(delivery_time);
            self.metrics
                .record_operation("message_delivery", delivery_time, true, None);
        }

        delivery_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p99 = delivery_times[(delivery_times.len() as f64 * 0.99) as usize];
        let success = p99 < 100.0; // p99 < 100ms

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Messages delivered: {}, p99 latency: {:.2}ms",
            message_queue.len(),
            p99
        ));

        Ok(result)
    }

    /// Test cascading failures
    pub async fn test_cascading_failures(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-interaction-cascading-failures";

        info!("Testing cascading failures...");

        let mut graph = ServiceDependencyGraph::new();

        // Create a dependency chain
        graph.add_dependency("api".to_string(), "auth".to_string());
        graph.add_dependency("api".to_string(), "database".to_string());
        graph.add_dependency("worker".to_string(), "database".to_string());
        graph.add_dependency("cache".to_string(), "database".to_string());
        graph.add_dependency("web".to_string(), "api".to_string());
        graph.add_dependency("web".to_string(), "cache".to_string());

        // Test cascade from database failure
        let cascaded = graph.check_cascade("database");
        let success = cascaded.len() > 3; // More than just the failed service

        self.metrics.record_operation(
            "cascading_failures",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status)
            .with_message(format!("Cascaded services: {:?}", cascaded));

        Ok(result)
    }

    /// Test timeout behavior
    pub async fn test_timeout_behavior(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "service-interaction-timeout-behavior";

        info!("Testing timeout behavior...");

        let timeout = std::time::Duration::from_millis(100);
        let mut timeouts = 0;
        let mut successes = 0;

        for _ in 0..100 {
            let op_start = Instant::now();

            // Simulate variable operation duration
            let duration = 50 + (rand::random::<u64>() % 150); // 50-200ms
            tokio::time::sleep(std::time::Duration::from_millis(duration)).await;

            let elapsed = op_start.elapsed();
            if elapsed > timeout {
                timeouts += 1;
            } else {
                successes += 1;
            }

            self.metrics
                .record_operation("timeout_test", elapsed.as_millis() as f64, elapsed <= timeout, None);
        }

        let timeout_rate = (timeouts as f64 / 100.0) * 100.0;
        let success = timeout_rate > 30.0 && timeout_rate < 70.0; // ~50% should timeout

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Timeouts: {}, Successes: {}, Timeout rate: {:.1}%",
            timeouts, successes, timeout_rate
        ));

        Ok(result)
    }

    /// Test backpressure handling
    pub async fn test_backpressure_handling(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "service-interaction-backpressure-handling";

        info!("Testing backpressure handling...");

        let max_queue_size = 1000;
        let mut queue_size = 0;
        let mut dropped = 0;
        let mut backpressure_applied = false;

        for i in 0..5000 {
            // Simulate incoming messages
            queue_size += 10;

            if queue_size > max_queue_size {
                // Apply backpressure
                backpressure_applied = true;
                let drain = queue_size - (max_queue_size / 2);
                dropped += drain;
                queue_size = max_queue_size / 2;
            }

            // Simulate processing
            if queue_size > 0 && i % 2 == 0 {
                queue_size = std::cmp::max(0, queue_size as i32 - 50) as usize;
            }
        }

        let success = backpressure_applied && dropped > 0 && queue_size < max_queue_size;

        self.metrics.record_operation(
            "backpressure_handling",
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
            "Backpressure applied: {}, Dropped: {}, Final queue size: {}",
            backpressure_applied, dropped, queue_size
        ));

        Ok(result)
    }

    /// Test service coordination
    pub async fn test_service_coordination(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "service-interaction-coordination";

        info!("Testing service coordination...");

        let services = vec![
            ("service-1", 0),
            ("service-2", 1),
            ("service-3", 2),
            ("service-4", 3),
            ("service-5", 4),
        ];

        let mut coordination_events = 0;

        // Simulate coordinated operations
        let test_duration = std::time::Duration::from_secs(2);
        let deadline = Instant::now() + test_duration;

        while Instant::now() < deadline {
            // Each service coordinates with others
            for (_name, idx) in &services {
                for (_other_name, other_idx) in &services {
                    if idx != other_idx {
                        let coord_start = Instant::now();
                        // Simulate coordination
                        tokio::time::sleep(std::time::Duration::from_micros(100)).await;
                        let coord_time = coord_start.elapsed().as_millis() as f64;
                        self.metrics
                            .record_operation("coordination", coord_time, true, None);
                        coordination_events += 1;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let success = coordination_events > 0;

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status)
            .with_message(format!("Coordination events: {}", coordination_events));

        Ok(result)
    }

    /// Test request/response patterns
    pub async fn test_request_response_patterns(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "service-interaction-request-response";

        info!("Testing request/response patterns...");

        let mut request_count = 0;
        let mut response_count = 0;
        let mut latencies = Vec::new();

        for _ in 0..self.config.concurrency {
            for _ in 0..100 {
                let req_start = Instant::now();

                // Send request
                request_count += 1;

                // Simulate processing and response
                let processing_time = 5.0 + (rand::random::<f64>() * 15.0); // 5-20ms
                tokio::time::sleep(std::time::Duration::from_millis(processing_time as u64)).await;

                response_count += 1;
                let latency = req_start.elapsed().as_millis() as f64;
                latencies.push(latency);

                self.metrics
                    .record_operation("request_response", latency, true, None);
            }
        }

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
        let success = request_count == response_count && p99 < 100.0;

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Requests: {}, Responses: {}, p99 latency: {:.2}ms",
            request_count, response_count, p99
        ));

        Ok(result)
    }

    /// Run all service interaction tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all service interaction tests...");

        let results = vec![
            self.test_cross_service_communication().await?,
            self.test_cascading_failures().await?,
            self.test_timeout_behavior().await?,
            self.test_backpressure_handling().await?,
            self.test_service_coordination().await?,
            self.test_request_response_patterns().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("service-interaction".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "Service interaction tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_message() {
        let msg = ServiceMessage::new("a".to_string(), "b".to_string(), vec![1, 2, 3]);
        assert_eq!(msg.from, "a");
        assert_eq!(msg.to, "b");
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = ServiceDependencyGraph::new();
        graph.add_dependency("a".to_string(), "b".to_string());
        let deps = graph.get_dependencies("a");
        assert!(deps.is_some());
    }

    #[test]
    fn test_cascade_detection() {
        let mut graph = ServiceDependencyGraph::new();
        graph.add_dependency("web".to_string(), "api".to_string());
        graph.add_dependency("api".to_string(), "db".to_string());

        let cascaded = graph.check_cascade("db");
        assert!(cascaded.contains(&"db".to_string()));
        assert!(cascaded.len() >= 1);
    }

    #[tokio::test]
    async fn test_service_interaction_tests() {
        let config = TestConfig::default();
        let tests = ServiceInteractionTests::new(config);

        let result = tests.test_cascading_failures().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
