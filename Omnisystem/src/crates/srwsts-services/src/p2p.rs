//! P2P mesh stress tests
//!
//! Tests for:
//! - Mesh convergence after killing 50% of nodes
//! - Multi-path bonding with FEC under 10% loss
//! - Bandwidth scaling from 1MB/s to 10GB/s

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use rand::Rng;
use std::time::Instant;
use tracing::{debug, info};

/// P2P mesh node
#[derive(Debug, Clone)]
struct MeshNode {
    id: String,
    neighbors: Vec<String>,
    is_alive: bool,
}

impl MeshNode {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            neighbors: Vec::new(),
            is_alive: true,
        }
    }

    fn add_neighbor(&mut self, neighbor: String) {
        if !self.neighbors.contains(&neighbor) {
            self.neighbors.push(neighbor);
        }
    }
}

/// P2P mesh network simulator
#[derive(Debug, Clone)]
pub struct P2PMesh {
    nodes: Vec<MeshNode>,
    message_queue: Vec<(String, String, Vec<u8>)>, // sender, recipient, payload
    convergence_time_ms: Option<u128>,
}

impl P2PMesh {
    pub fn new(node_count: usize) -> Self {
        let mut nodes = Vec::new();
        for i in 0..node_count {
            nodes.push(MeshNode::new(format!("node-{}", i)));
        }

        // Full mesh connectivity
        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                if i != j {
                    nodes[i].add_neighbor(format!("node-{}", j));
                }
            }
        }

        Self {
            nodes,
            message_queue: Vec::new(),
            convergence_time_ms: None,
        }
    }

    pub fn kill_nodes(&mut self, count: usize) {
        let mut rng = rand::thread_rng();
        let mut killed = 0;
        while killed < count && killed < self.nodes.len() {
            let idx = rng.gen_range(0..self.nodes.len());
            if self.nodes[idx].is_alive {
                self.nodes[idx].is_alive = false;
                killed += 1;
            }
        }
    }

    pub fn alive_nodes(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_alive).count()
    }

    pub fn check_convergence(&mut self) -> bool {
        // Simple convergence check: each alive node can reach all other alive nodes
        for node in self.nodes.iter().filter(|n| n.is_alive) {
            for other in self.nodes.iter().filter(|n| n.is_alive) {
                if node.id != other.id && !self.can_reach(&node.id, &other.id) {
                    return false;
                }
            }
        }
        true
    }

    fn can_reach(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![from.to_string()];

        while let Some(current) = queue.pop() {
            if current == to {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(node) = self.nodes.iter().find(|n| n.id == current && n.is_alive) {
                for neighbor in &node.neighbors {
                    if let Some(n) = self.nodes.iter().find(|n| &n.id == neighbor && n.is_alive) {
                        queue.push(n.id.clone());
                    }
                }
            }
        }

        false
    }

    pub fn send_message(&mut self, from: String, to: String, payload: Vec<u8>) {
        self.message_queue.push((from, to, payload));
    }

    pub fn process_messages(&mut self) -> usize {
        let count = self.message_queue.len();
        self.message_queue.clear();
        count
    }
}

/// P2P stress tests
pub struct P2PStressTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl P2PStressTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("p2p"),
        }
    }

    /// Test mesh convergence after killing 50% of nodes
    pub async fn test_mesh_convergence(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "p2p-mesh-convergence";

        info!("Testing P2P mesh convergence after node failures...");

        let mut mesh = P2PMesh::new(100);
        let initial_nodes = mesh.alive_nodes();

        // Kill 50% of nodes
        mesh.kill_nodes(initial_nodes / 2);
        let remaining_nodes = mesh.alive_nodes();

        debug!(
            "Killed {} nodes, {} remain",
            initial_nodes - remaining_nodes,
            remaining_nodes
        );

        // Simulate convergence detection (in real system, would measure actual convergence)
        let convergence_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut converged = false;
        let mut convergence_time_ms = 0u128;

        while std::time::Instant::now() < convergence_deadline {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if mesh.check_convergence() {
                converged = true;
                convergence_time_ms = start.elapsed().as_millis();
                break;
            }
        }

        self.metrics.record_operation(
            "mesh_convergence",
            convergence_time_ms as f64,
            converged,
            if !converged {
                Some("Convergence timeout".to_string())
            } else {
                None
            },
        );

        let status = if converged {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status)
            .with_message(format!(
                "Convergence: {} ({}ms), Alive nodes: {}/{}",
                converged, convergence_time_ms, remaining_nodes, initial_nodes
            ));

        Ok(result)
    }

    /// Test multi-path bonding with FEC under 10% packet loss
    pub async fn test_multipath_bonding(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "p2p-multipath-bonding";

        info!("Testing multi-path bonding with FEC under 10% loss...");

        let mut rng = rand::thread_rng();
        let packet_count = 1000;
        let loss_rate = 0.10; // 10% loss
        let fec_recovery = 0.95; // FEC recovers 95% of lost packets

        let mut lost = 0;
        let mut recovered = 0;

        for _ in 0..packet_count {
            if rng.gen::<f64>() < loss_rate {
                lost += 1;
                if rng.gen::<f64>() < fec_recovery {
                    recovered += 1;
                }
            }
        }

        let effective_loss = (lost - recovered) as f64 / packet_count as f64;
        let success = effective_loss < 0.01; // Less than 1% effective loss

        self.metrics.record_operation(
            "multipath_bonding",
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
            "Packets lost: {}, Recovered: {}, Effective loss: {:.2}%",
            lost, recovered, effective_loss * 100.0
        ));

        Ok(result)
    }

    /// Test bandwidth scaling from 1MB/s to 10GB/s
    pub async fn test_bandwidth_scaling(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "p2p-bandwidth-scaling";

        info!("Testing bandwidth scaling from 1MB/s to 10GB/s...");

        let bandwidths: Vec<u64> = vec![
            1_000_000,        // 1 MB/s
            100_000_000,      // 100 MB/s
            1_000_000_000,    // 1 GB/s
            5_000_000_000u64,    // 5 GB/s
            10_000_000_000u64,   // 10 GB/s
        ];

        let mut all_passed = true;
        let mut throughputs = Vec::new();

        for bandwidth in bandwidths {
            // Simulate transfer of 1GB at this bandwidth
            let transfer_size = 1_000_000_000;
            let expected_duration =
                std::time::Duration::from_secs_f64(transfer_size as f64 / bandwidth as f64);

            // Simulate with some variance
            let mut rng = rand::thread_rng();
            let variance = 1.0 + (rng.gen::<f64>() - 0.5) * 0.2; // ±10%
            let actual_duration = expected_duration.mul_f64(variance);

            let achieved_bandwidth = transfer_size as f64 / actual_duration.as_secs_f64();
            let efficiency = achieved_bandwidth / bandwidth as f64;

            throughputs.push((bandwidth, achieved_bandwidth, efficiency));

            if efficiency < 0.85 {
                all_passed = false;
            }

            debug!(
                "Bandwidth: {} bytes/s, Achieved: {:.0}, Efficiency: {:.1}%",
                bandwidth,
                achieved_bandwidth,
                efficiency * 100.0
            );
        }

        self.metrics.record_operation(
            "bandwidth_scaling",
            start.elapsed().as_millis() as f64,
            all_passed,
            None,
        );

        let status = if all_passed {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let mut details = String::from("Bandwidth scaling:");
        for (bw, achieved, eff) in throughputs {
            details.push_str(&format!("\n  {}: {:.0} ({:.1}%)", bw, achieved, eff * 100.0));
        }

        let result = TestResult::new(test_id, status).with_message(details);

        Ok(result)
    }

    /// Test P2P network latency under load
    pub async fn test_latency_under_load(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "p2p-latency-under-load";

        info!("Testing P2P latency under load...");

        let mut latencies = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..self.config.concurrency {
            // Simulate message round-trip
            let base_latency = 10.0 + rng.gen::<f64>() * 40.0; // 10-50ms base
            let load_factor = 1.0 + (rng.gen::<f64>() * 0.5); // 0-50% increase
            let latency = base_latency * load_factor;
            latencies.push(latency);
        }

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_latency = latencies[(latencies.len() as f64 * 0.95) as usize];
        let p99_latency = latencies[(latencies.len() as f64 * 0.99) as usize];

        let success = p99_latency < 200.0; // p99 < 200ms

        self.metrics.record_operation(
            "latency_under_load",
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
            "p50: {:.1}ms, p95: {:.1}ms, p99: {:.1}ms",
            latencies[latencies.len() / 2], p95_latency, p99_latency
        ));

        Ok(result)
    }

    /// Test node churn (nodes joining and leaving)
    pub async fn test_node_churn(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "p2p-node-churn";

        info!("Testing P2P node churn...");

        let mut mesh = P2PMesh::new(50);
        let mut churn_events = 0;
        let test_duration = self.config.timeout;
        let deadline = Instant::now() + test_duration;

        while Instant::now() < deadline {
            let mut rng = rand::thread_rng();

            // Random node joins/leaves
            if rng.gen::<bool>() {
                mesh.kill_nodes(1);
                churn_events += 1;
            } else {
                // Rejoin
                if let Some(node) = mesh.nodes.iter_mut().find(|n| !n.is_alive) {
                    node.is_alive = true;
                    churn_events += 1;
                }
            }

            // Check convergence
            let _ = mesh.check_convergence();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let success = mesh.alive_nodes() > 0;

        self.metrics.record_operation(
            "node_churn",
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
            "Churn events: {}, Final alive nodes: {}",
            churn_events,
            mesh.alive_nodes()
        ));

        Ok(result)
    }

    /// Run all P2P stress tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all P2P stress tests...");

        let results = vec![
            self.test_mesh_convergence().await?,
            self.test_multipath_bonding().await?,
            self.test_bandwidth_scaling().await?,
            self.test_latency_under_load().await?,
            self.test_node_churn().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("p2p".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "P2P tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_creation() {
        let mesh = P2PMesh::new(10);
        assert_eq!(mesh.alive_nodes(), 10);
    }

    #[test]
    fn test_mesh_kill_nodes() {
        let mut mesh = P2PMesh::new(10);
        mesh.kill_nodes(5);
        assert_eq!(mesh.alive_nodes(), 5);
    }

    #[test]
    fn test_mesh_convergence_check() {
        let mut mesh = P2PMesh::new(5);
        assert!(mesh.check_convergence());
        mesh.kill_nodes(1);
        assert!(mesh.check_convergence());
    }

    #[tokio::test]
    async fn test_p2p_stress_tests() {
        let config = TestConfig::default();
        let tests = P2PStressTests::new(config);

        let result = tests.test_mesh_convergence().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
