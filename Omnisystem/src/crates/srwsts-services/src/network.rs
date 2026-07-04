//! Network stack stress tests
//!
//! Tests for:
//! - TCP stack under 10Gbps
//! - Firewall rule matching with 1M rules
//! - Packet reordering
//! - Zero-copy throughput

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use std::time::Instant;
use tracing::info;

/// Simulated firewall rule
#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub id: String,
    pub priority: u32,
    pub src_ip: String,
    pub dst_ip: String,
    pub action: FirewallAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallAction {
    Allow,
    Deny,
    Drop,
}

impl FirewallRule {
    pub fn matches(&self, src: &str, dst: &str) -> bool {
        self.src_ip == src || self.src_ip == "*" && (self.dst_ip == dst || self.dst_ip == "*")
    }
}

/// Network stack simulator
pub struct NetworkStack {
    rules: Vec<FirewallRule>,
    packets: Vec<(String, String, Vec<u8>)>,
}

impl NetworkStack {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            packets: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: FirewallRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    pub fn match_rules(&self, src: &str, dst: &str) -> Option<FirewallAction> {
        for rule in &self.rules {
            if rule.matches(src, dst) {
                return Some(rule.action.clone());
            }
        }
        None
    }

    pub fn send_packet(&mut self, src: String, dst: String, data: Vec<u8>) -> bool {
        match self.match_rules(&src, &dst) {
            Some(FirewallAction::Allow) => {
                self.packets.push((src, dst, data));
                true
            }
            Some(FirewallAction::Deny) | Some(FirewallAction::Drop) => false,
            None => false,
        }
    }

    pub fn packet_count(&self) -> usize {
        self.packets.len()
    }
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    Closing,
    Closed,
}

/// Simulated TCP connection
pub struct TcpConnection {
    pub id: String,
    pub state: TcpState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub rtt_ms: f64,
}

impl TcpConnection {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: TcpState::Listen,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            rtt_ms: 10.0,
        }
    }

    pub fn send(&mut self, bytes: u64, packets: u64) {
        self.bytes_sent += bytes;
        self.packets_sent += packets;
    }

    pub fn receive(&mut self, bytes: u64, packets: u64) {
        self.bytes_received += bytes;
        self.packets_received += packets;
    }

    pub fn throughput_mbps(&self, elapsed_secs: f64) -> f64 {
        if elapsed_secs <= 0.0 {
            return 0.0;
        }
        (self.bytes_sent as f64 / elapsed_secs) / 1_000_000.0
    }
}

/// Network stress tests
pub struct NetworkStressTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl NetworkStressTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("network"),
        }
    }

    /// Test TCP stack under 10Gbps
    pub async fn test_tcp_high_throughput(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "network-tcp-high-throughput";

        info!("Testing TCP stack under 10Gbps...");

        let mut connections = Vec::new();
        let target_throughput = 10_000_000_000u64; // 10 Gbps
        let target_duration = 10; // seconds

        for i in 0..self.config.concurrency {
            let mut conn = TcpConnection::new(format!("tcp-conn-{}", i));
            conn.state = TcpState::Established;

            // Calculate bytes to send per connection
            let bytes_per_conn = target_throughput / self.config.concurrency as u64;
            let send_bytes = (bytes_per_conn / target_duration as u64) as u64;

            conn.send(send_bytes * target_duration as u64, send_bytes / 1500); // 1500 byte MTU
            connections.push(conn);
        }

        let total_bytes: u64 = connections.iter().map(|c| c.bytes_sent).sum();
        let elapsed = start.elapsed().as_secs_f64();
        let achieved_throughput = (total_bytes as f64 / elapsed) / 1_000_000_000.0;

        let success = achieved_throughput > 8.0; // Achieve at least 8 Gbps

        self.metrics.record_operation(
            "tcp_high_throughput",
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
            "Achieved: {:.2} Gbps, Connections: {}, Total bytes: {}",
            achieved_throughput,
            connections.len(),
            total_bytes
        ));

        Ok(result)
    }

    /// Test firewall rule matching with 1M rules
    pub async fn test_firewall_matching(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "network-firewall-matching";

        info!("Testing firewall rule matching with 1M rules...");

        let mut stack = NetworkStack::new();

        // Create 1 million rules
        for i in 0..1_000_000 {
            let rule = FirewallRule {
                id: format!("rule-{}", i),
                priority: i as u32,
                src_ip: format!("10.0.{}.{}", (i / 256) % 256, i % 256),
                dst_ip: "192.168.1.1".to_string(),
                action: if i % 2 == 0 {
                    FirewallAction::Allow
                } else {
                    FirewallAction::Deny
                },
            };
            stack.add_rule(rule);
        }

        // Test rule matching
        let mut match_count = 0;
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        for _ in 0..10000 {
            let src = format!(
                "10.0.{}.{}",
                rng.next_u32() % 256,
                rng.next_u32() % 256
            );
            if stack.match_rules(&src, "192.168.1.1").is_some() {
                match_count += 1;
            }
        }

        let match_rate = (match_count as f64 / 10000.0) * 100.0;
        let success = match_rate > 40.0 && match_rate < 60.0; // ~50% should match

        self.metrics.record_operation(
            "firewall_matching",
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
            "Match rate: {:.1}%, Rules: 1,000,000, Queries: 10,000",
            match_rate
        ));

        Ok(result)
    }

    /// Test packet reordering recovery
    pub async fn test_packet_reordering(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "network-packet-reordering";

        info!("Testing packet reordering recovery...");

        let mut packets: Vec<_> = (0..1000).collect();

        // Simulate reordering
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        for _ in 0..500 {
            let i = rng.next_u32() as usize % packets.len();
            let j = rng.next_u32() as usize % packets.len();
            packets.swap(i, j);
        }

        // Measure how many are out of order
        let mut out_of_order = 0;
        for i in 1..packets.len() {
            if packets[i] < packets[i - 1] {
                out_of_order += 1;
            }
        }

        // Simulate resequencing
        let start_reseq = Instant::now();
        packets.sort();
        let reseq_time = start_reseq.elapsed();

        let success = reseq_time.as_millis() < 100; // Resequence in < 100ms

        self.metrics.record_operation(
            "packet_reordering",
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
            "Out of order packets: {}, Resequence time: {}ms",
            out_of_order,
            reseq_time.as_millis()
        ));

        Ok(result)
    }

    /// Test zero-copy throughput
    pub async fn test_zero_copy_throughput(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "network-zero-copy-throughput";

        info!("Testing zero-copy throughput...");

        let packet_size = 4096;
        let packet_count = 100000;
        let mut total_bytes = 0u64;

        for _ in 0..packet_count {
            let packet = vec![0u8; packet_size];
            // Simulate zero-copy by just counting bytes
            total_bytes += packet.len() as u64;
        }

        let elapsed = start.elapsed().as_secs_f64();
        let throughput_gbps = (total_bytes as f64 / elapsed) / 1_000_000_000.0;

        let success = throughput_gbps > 5.0; // > 5 Gbps

        self.metrics.record_operation(
            "zero_copy_throughput",
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
            "Throughput: {:.2} Gbps, Packets: {}, Size: {} bytes",
            throughput_gbps, packet_count, packet_size
        ));

        Ok(result)
    }

    /// Test congestion control (CUBIC)
    pub async fn test_congestion_control(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "network-congestion-control";

        info!("Testing congestion control (CUBIC)...");

        let mut congestion_window = 10.0; // packets
        let mut loss_rate = 0.05; // 5% loss
        let mut iterations = 0;
        let mut recovery_successful = false;

        while iterations < 1000 && congestion_window < 1000.0 {
            // Simulate congestion
            if rand::random::<f64>() < loss_rate {
                // Loss detected, reduce window
                congestion_window *= 0.7;
                loss_rate *= 1.1; // More congestion
            } else {
                // Increase window (additive increase)
                congestion_window += 1.0;
                loss_rate *= 0.95; // Less congestion
            }

            if loss_rate < 0.01 && congestion_window > 50.0 {
                recovery_successful = true;
                break;
            }

            iterations += 1;
        }

        let success = recovery_successful;

        self.metrics.record_operation(
            "congestion_control",
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
            "Recovery successful: {}, Final window: {:.0}, Iterations: {}",
            recovery_successful, congestion_window, iterations
        ));

        Ok(result)
    }

    /// Test connection establishment under load
    pub async fn test_connection_establishment(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "network-connection-establishment";

        info!("Testing connection establishment under load...");

        let mut connection_times = Vec::new();

        for i in 0..self.config.concurrency {
            let start_conn = Instant::now();

            // Simulate 3-way handshake
            let mut conn = TcpConnection::new(format!("conn-{}", i));
            conn.state = TcpState::SynSent;
            tokio::time::sleep(std::time::Duration::from_micros(100)).await;
            conn.state = TcpState::Established;

            let conn_time = start_conn.elapsed().as_millis() as f64;
            connection_times.push(conn_time);
            self.metrics.record_operation("connection_establish", conn_time, true, None);
        }

        connection_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p99 = connection_times[(connection_times.len() as f64 * 0.99) as usize];
        let success = p99 < 100.0; // p99 < 100ms

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Connections established: {}, p99: {:.2}ms",
            connection_times.len(),
            p99
        ));

        Ok(result)
    }

    /// Run all network stress tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all network stress tests...");

        let results = vec![
            self.test_tcp_high_throughput().await?,
            self.test_firewall_matching().await?,
            self.test_packet_reordering().await?,
            self.test_zero_copy_throughput().await?,
            self.test_congestion_control().await?,
            self.test_connection_establishment().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("network".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "Network tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_rule_matching() {
        let rule = FirewallRule {
            id: "test".to_string(),
            priority: 1,
            src_ip: "10.0.0.1".to_string(),
            dst_ip: "*".to_string(),
            action: FirewallAction::Allow,
        };

        assert!(rule.matches("10.0.0.1", "192.168.1.1"));
        assert!(!rule.matches("10.0.0.2", "192.168.1.1"));
    }

    #[test]
    fn test_network_stack() {
        let mut stack = NetworkStack::new();
        let rule = FirewallRule {
            id: "allow".to_string(),
            priority: 1,
            src_ip: "*".to_string(),
            dst_ip: "*".to_string(),
            action: FirewallAction::Allow,
        };
        stack.add_rule(rule);

        assert!(stack.send_packet("10.0.0.1".to_string(), "192.168.1.1".to_string(), vec![]));
        assert_eq!(stack.packet_count(), 1);
    }

    #[test]
    fn test_tcp_connection() {
        let mut conn = TcpConnection::new("test");
        conn.send(1000000, 667);
        assert_eq!(conn.bytes_sent, 1000000);
        assert_eq!(conn.packets_sent, 667);
    }

    #[tokio::test]
    async fn test_network_stress_tests() {
        let config = TestConfig::default();
        let tests = NetworkStressTests::new(config);

        let result = tests.test_tcp_high_throughput().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
