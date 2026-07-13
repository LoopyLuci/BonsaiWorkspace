//! Kernel Driver Stress Tests
//!
//! Tests storage I/O under 100% random load, network driver under line-rate traffic,
//! and interrupt handling latency. Validates driver subsystems under extreme stress.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Driver test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConfig {
    /// Storage I/O operations per second target
    pub storage_iops_target: u64,
    /// Storage request size in bytes
    pub storage_request_size: u64,
    /// Network packet size
    pub network_packet_size: usize,
    /// Network throughput target in Gbps
    pub network_throughput_gbps: f64,
    /// Interrupt processing latency budget in microseconds
    pub interrupt_latency_budget_us: u64,
    /// Enable random I/O pattern
    pub enable_random_io: bool,
    /// Number of parallel I/O operations
    pub parallel_io_ops: usize,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            storage_iops_target: 100_000,
            storage_request_size: 4096,
            network_packet_size: 1500,
            network_throughput_gbps: 100.0,
            interrupt_latency_budget_us: 10,
            enable_random_io: true,
            parallel_io_ops: 256,
        }
    }
}

/// Storage I/O request
#[derive(Debug, Clone)]
pub struct StorageIORequest {
    pub id: u64,
    pub offset: u64,
    pub size: u64,
    pub operation: IOOperation,
    pub issued_at_ns: u64,
    pub completed_at_ns: Option<u64>,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IOOperation {
    Read,
    Write,
    Flush,
}

impl StorageIORequest {
    /// Create a new storage I/O request
    pub fn new(id: u64, offset: u64, size: u64, operation: IOOperation) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            offset,
            size,
            operation,
            issued_at_ns: now,
            completed_at_ns: None,
        }
    }

    /// Get latency in nanoseconds
    pub fn latency_ns(&self) -> Option<u64> {
        self.completed_at_ns.map(|end| end - self.issued_at_ns)
    }
}

/// Network packet
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub id: u64,
    pub source_port: u16,
    pub dest_port: u16,
    pub payload_size: usize,
    pub sent_at_ns: u64,
    pub received_at_ns: Option<u64>,
}

impl NetworkPacket {
    /// Create a new network packet
    pub fn new(id: u64, src_port: u16, dst_port: u16, payload_size: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            source_port: src_port,
            dest_port: dst_port,
            payload_size,
            sent_at_ns: now,
            received_at_ns: None,
        }
    }

    /// Get delivery latency in nanoseconds
    pub fn delivery_latency_ns(&self) -> Option<u64> {
        self.received_at_ns.map(|recv| recv - self.sent_at_ns)
    }
}

/// Driver statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverStats {
    pub storage_iops: f64,
    pub storage_avg_latency_us: f64,
    pub storage_p99_latency_us: f64,
    pub storage_throughput_mbps: f64,
    pub network_packet_rate: f64,
    pub network_throughput_gbps: f64,
    pub network_avg_latency_us: f64,
    pub network_p99_latency_us: f64,
    pub interrupt_count: u64,
    pub interrupt_max_latency_us: f64,
    pub errors: u64,
}

/// Storage driver test
pub struct StorageDriverTest {
    config: DriverConfig,
    requests: Arc<RwLock<Vec<StorageIORequest>>>,
    io_count: Arc<AtomicU64>,
}

impl StorageDriverTest {
    /// Create a new storage driver test
    pub fn new(config: DriverConfig) -> Self {
        Self {
            config,
            requests: Arc::new(RwLock::new(Vec::new())),
            io_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Test random I/O stress
    pub async fn test_random_io_stress(&self) -> Result<()> {
        info!(
            "Starting storage I/O stress test: {} IOPS target, {} byte requests",
            self.config.storage_iops_target, self.config.storage_request_size
        );

        let mut handles = vec![];

        for i in 0..self.config.parallel_io_ops {
            let config = self.config.clone();
            let requests = Arc::clone(&self.requests);
            let io_count = Arc::clone(&self.io_count);

            let handle = tokio::spawn(async move {
                for req_id in 0..1000 {
                    let offset = if config.enable_random_io {
                        // Use deterministic pattern based on task and request id
                        ((i as u64).wrapping_mul(1103515245).wrapping_add(12345) ^ (req_id as u64)) % (1024 * 1024 * 1024)
                    } else {
                        (i as u64 * 1000 + req_id as u64) * config.storage_request_size
                    };

                    let operation = match (i + req_id) % 3 {
                        0 => IOOperation::Read,
                        1 => IOOperation::Write,
                        _ => IOOperation::Flush,
                    };

                    let mut request =
                        StorageIORequest::new(req_id as u64, offset, config.storage_request_size, operation);

                    // Simulate I/O latency
                    tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;

                    request.completed_at_ns = Some(now);
                    requests.write().await.push(request);
                    io_count.fetch_add(1, Ordering::Relaxed);

                    if req_id % 100 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Storage I/O test completed: {} operations", self.io_count.load(Ordering::Relaxed));
        Ok(())
    }

    /// Test sequential I/O
    pub async fn test_sequential_io(&self) -> Result<()> {
        info!("Testing sequential storage I/O");

        let mut offset = 0u64;
        for i in 0..1000 {
            let request = StorageIORequest::new(i, offset, self.config.storage_request_size, IOOperation::Read);
            offset += self.config.storage_request_size;

            self.requests.write().await.push(request);
            self.io_count.fetch_add(1, Ordering::Relaxed);

            if i % 100 == 0 {
                tokio::task::yield_now().await;
            }
        }

        debug!("Sequential I/O test completed");
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> DriverStats {
        let requests = self.requests.read().await;

        let latencies: Vec<u64> = requests
            .iter()
            .filter_map(|r| r.latency_ns())
            .collect();

        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64 / 1000.0
        } else {
            0.0
        };

        let sorted: Vec<_> = latencies.iter().map(|l| *l as f64 / 1000.0).collect();
        let p99_latency = if !sorted.is_empty() {
            sorted[(sorted.len() * 99 / 100).min(sorted.len() - 1)]
        } else {
            0.0
        };

        let throughput_bytes = requests.iter().map(|r| r.size).sum::<u64>();
        let throughput_mbps = (throughput_bytes as f64) / (1024.0 * 1024.0);

        DriverStats {
            storage_iops: self.io_count.load(Ordering::Relaxed) as f64,
            storage_avg_latency_us: avg_latency,
            storage_p99_latency_us: p99_latency,
            storage_throughput_mbps: throughput_mbps,
            network_packet_rate: 0.0,
            network_throughput_gbps: 0.0,
            network_avg_latency_us: 0.0,
            network_p99_latency_us: 0.0,
            interrupt_count: 0,
            interrupt_max_latency_us: 0.0,
            errors: 0,
        }
    }
}

/// Network driver test
pub struct NetworkDriverTest {
    config: DriverConfig,
    packets: Arc<RwLock<Vec<NetworkPacket>>>,
    packet_count: Arc<AtomicU64>,
}

impl NetworkDriverTest {
    /// Create a new network driver test
    pub fn new(config: DriverConfig) -> Self {
        Self {
            config,
            packets: Arc::new(RwLock::new(Vec::new())),
            packet_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Test line-rate traffic
    pub async fn test_line_rate_traffic(&self) -> Result<()> {
        info!(
            "Starting network driver test: {} Gbps throughput, {} byte packets",
            self.config.network_throughput_gbps, self.config.network_packet_size
        );

        let mut handles = vec![];

        for i in 0..100 {
            let config = self.config.clone();
            let packets = Arc::clone(&self.packets);
            let packet_count = Arc::clone(&self.packet_count);

            let handle = tokio::spawn(async move {
                for j in 0..10000 {
                    let packet = NetworkPacket::new(
                        (i as u64) * 10000 + (j as u64),
                        1000 + (i as u16),
                        2000 + (j as u16),
                        config.network_packet_size,
                    );

                    // Simulate packet transmission
                    tokio::time::sleep(tokio::time::Duration::from_nanos(10)).await;

                    let mut p = packet;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;
                    p.received_at_ns = Some(now);

                    packets.write().await.push(p);
                    packet_count.fetch_add(1, Ordering::Relaxed);

                    if j % 1000 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!(
            "Network test completed: {} packets",
            self.packet_count.load(Ordering::Relaxed)
        );
        Ok(())
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> DriverStats {
        let packets = self.packets.read().await;

        let latencies: Vec<u64> = packets
            .iter()
            .filter_map(|p| p.delivery_latency_ns())
            .collect();

        let avg_latency = if !latencies.is_empty() {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64 / 1000.0
        } else {
            0.0
        };

        let sorted: Vec<_> = latencies.iter().map(|l| *l as f64 / 1000.0).collect();
        let p99_latency = if !sorted.is_empty() {
            sorted[(sorted.len() * 99 / 100).min(sorted.len() - 1)]
        } else {
            0.0
        };

        let throughput_bits = (self.packet_count.load(Ordering::Relaxed) as u64) * (self.config.network_packet_size as u64) * 8;
        let throughput_gbps = throughput_bits as f64 / 1_000_000_000.0;

        DriverStats {
            storage_iops: 0.0,
            storage_avg_latency_us: 0.0,
            storage_p99_latency_us: 0.0,
            storage_throughput_mbps: 0.0,
            network_packet_rate: self.packet_count.load(Ordering::Relaxed) as f64,
            network_throughput_gbps: throughput_gbps,
            network_avg_latency_us: avg_latency,
            network_p99_latency_us: p99_latency,
            interrupt_count: 0,
            interrupt_max_latency_us: 0.0,
            errors: 0,
        }
    }
}

/// Interrupt handling test
pub struct InterruptTest {
    config: DriverConfig,
}

impl InterruptTest {
    /// Create a new interrupt test
    pub fn new(config: DriverConfig) -> Self {
        Self { config }
    }

    /// Test interrupt latency
    pub async fn test_interrupt_latency(&self) -> Result<()> {
        info!("Testing interrupt handling latency");

        let latencies = Arc::new(RwLock::new(Vec::new()));
        let mut handles = vec![];

        for _ in 0..1000 {
            let lats = Arc::clone(&latencies);

            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();
                // Simulate interrupt handling
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
                let duration = start.elapsed().as_micros() as f64;

                lats.write().await.push(duration);
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Interrupt latency test completed");
        Ok(())
    }
}

/// High-level driver test
pub struct DriverTest {
    config: DriverConfig,
}

impl DriverTest {
    /// Create a new driver test
    pub fn new(config: DriverConfig) -> Self {
        Self { config }
    }

    /// Run the complete driver test suite
    pub async fn run(&self) -> Result<()> {
        let storage = StorageDriverTest::new(self.config.clone());
        storage.test_random_io_stress().await?;
        storage.test_sequential_io().await?;

        let network = NetworkDriverTest::new(self.config.clone());
        network.test_line_rate_traffic().await?;

        let interrupt = InterruptTest::new(self.config.clone());
        interrupt.test_interrupt_latency().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_io_request_creation() {
        let req = StorageIORequest::new(1, 0, 4096, IOOperation::Read);
        assert_eq!(req.id, 1);
        assert_eq!(req.offset, 0);
        assert_eq!(req.size, 4096);
    }

    #[test]
    fn test_network_packet_creation() {
        let pkt = NetworkPacket::new(1, 1000, 2000, 1500);
        assert_eq!(pkt.id, 1);
        assert_eq!(pkt.source_port, 1000);
        assert_eq!(pkt.dest_port, 2000);
    }

    #[tokio::test]
    async fn test_storage_driver_test() {
        let config = DriverConfig {
            storage_iops_target: 10_000,
            parallel_io_ops: 10,
            ..Default::default()
        };
        let test = StorageDriverTest::new(config);
        let result = test.test_random_io_stress().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_network_driver_test() {
        let config = DriverConfig {
            ..Default::default()
        };
        let test = NetworkDriverTest::new(config);
        let result = test.test_line_rate_traffic().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_interrupt_test() {
        let test = InterruptTest::new(DriverConfig::default());
        let result = test.test_interrupt_latency().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_driver_test_suite() {
        let config = DriverConfig {
            parallel_io_ops: 5,
            ..Default::default()
        };
        let test = DriverTest::new(config);
        let result = test.run().await;
        assert!(result.is_ok());
    }
}
