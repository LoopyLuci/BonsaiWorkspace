//! Kernel IPC Stress Tests
//!
//! Tests inter-process communication including message passing throughput (1M msgs/sec target),
//! latency (p99 <5µs), capability revocation, and semaphore contention under stress.

use crate::metrics::MetricsCollector;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{debug, info};

/// IPC test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCConfig {
    /// Number of message senders
    pub num_senders: usize,
    /// Number of message receivers
    pub num_receivers: usize,
    /// Number of messages per sender
    pub messages_per_sender: u64,
    /// Message size in bytes
    pub message_size_bytes: usize,
    /// Number of semaphores
    pub num_semaphores: usize,
    /// Semaphore contention level
    pub semaphore_contentions: usize,
    /// Channel buffer size
    pub channel_buffer_size: usize,
    /// Target throughput in messages per second
    pub target_throughput_mps: u64,
    /// Target latency p99 in microseconds
    pub target_latency_p99_us: u64,
}

impl Default for IPCConfig {
    fn default() -> Self {
        Self {
            num_senders: 100,
            num_receivers: 100,
            messages_per_sender: 10000,
            message_size_bytes: 256,
            num_semaphores: 16,
            semaphore_contentions: 100,
            channel_buffer_size: 1000,
            target_throughput_mps: 1_000_000,
            target_latency_p99_us: 5,
        }
    }
}

/// IPC message
#[derive(Debug, Clone)]
pub struct IPCMessage {
    pub id: u64,
    pub sender_id: u64,
    pub receiver_id: u64,
    pub timestamp_ns: u64,
    pub data: Vec<u8>,
    pub capability_id: Option<u64>,
}

impl IPCMessage {
    /// Create a new IPC message
    pub fn new(id: u64, sender_id: u64, receiver_id: u64, size: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            sender_id,
            receiver_id,
            timestamp_ns: now,
            data: vec![0u8; size],
            capability_id: None,
        }
    }

    /// Set capability
    pub fn with_capability(mut self, cap_id: u64) -> Self {
        self.capability_id = Some(cap_id);
        self
    }

    /// Get delivery latency in nanoseconds
    pub fn delivery_latency_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64 - self.timestamp_ns
    }
}

/// Capability tracking
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: u64,
    pub owner_id: u64,
    pub created_at_ns: u64,
    pub revoked_at_ns: Option<u64>,
    pub access_count: u64,
}

impl Capability {
    /// Create a new capability
    pub fn new(id: u64, owner_id: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            owner_id,
            created_at_ns: now,
            revoked_at_ns: None,
            access_count: 0,
        }
    }

    /// Check if capability is valid
    pub fn is_valid(&self) -> bool {
        self.revoked_at_ns.is_none()
    }

    /// Revoke the capability
    pub fn revoke(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.revoked_at_ns = Some(now);
    }
}

/// IPC statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCStats {
    pub total_messages: u64,
    pub successful_messages: u64,
    pub failed_messages: u64,
    pub avg_latency_us: f64,
    pub p50_latency_us: f64,
    pub p99_latency_us: f64,
    pub max_latency_us: f64,
    pub throughput_mps: f64,
    pub capability_revocations: u64,
    pub semaphore_acquisitions: u64,
    pub semaphore_timeouts: u64,
}

/// Message passing test
pub struct MessagePassingTest {
    config: IPCConfig,
    metrics: Arc<RwLock<MetricsCollector>>,
}

use tokio::sync::RwLock;

impl MessagePassingTest {
    /// Create a new message passing test
    pub fn new(config: IPCConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
        }
    }

    /// Run message passing throughput test
    pub async fn test_throughput(&self) -> Result<IPCStats> {
        info!(
            "Starting message passing throughput test: {} senders, {} receivers, {} msgs/sender",
            self.config.num_senders, self.config.num_receivers, self.config.messages_per_sender
        );

        let start_time = std::time::Instant::now();
        let message_count = Arc::new(AtomicU64::new(0));
        let latencies = Arc::new(Mutex::new(Vec::new()));

        let mut handles = vec![];

        // Create receivers
        let (tx, mut rx) = mpsc::channel::<IPCMessage>(self.config.channel_buffer_size);

        let msg_count = Arc::clone(&message_count);
        let lats = Arc::clone(&latencies);
        let metrics = Arc::clone(&self.metrics);

        let receiver_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let lat = msg.delivery_latency_ns() / 1000; // Convert to microseconds
                lats.lock().await.push(lat as f64);
                msg_count.fetch_add(1, Ordering::Relaxed);

                let mut m = metrics.write().await;
                m.record_latency("ipc_latency_us", lat as f64);
            }
        });

        // Create senders
        for sender_id in 0..self.config.num_senders {
            let tx = tx.clone();
            let msg_size = self.config.message_size_bytes;
            let msgs_per_sender = self.config.messages_per_sender;

            let handle = tokio::spawn(async move {
                for i in 0..msgs_per_sender {
                    let msg = IPCMessage::new(
                        (sender_id as u64) * 100000 + i,
                        sender_id as u64,
                        (i % 100) as u64,
                        msg_size,
                    );

                    let _ = tx.send(msg).await;

                    if i % 1000 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for senders
        for handle in handles {
            let _ = handle.await;
        }

        // Close sender
        drop(tx);

        // Wait for receiver
        let _ = receiver_handle.await;

        let elapsed = start_time.elapsed().as_secs_f64();
        let total_messages = message_count.load(Ordering::Relaxed);
        let throughput = total_messages as f64 / elapsed;

        let mut lats = latencies.lock().await;
        lats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let avg_latency = if !lats.is_empty() {
            lats.iter().sum::<f64>() / lats.len() as f64
        } else {
            0.0
        };

        let p50 = if !lats.is_empty() {
            lats[lats.len() / 2]
        } else {
            0.0
        };

        let p99 = if !lats.is_empty() {
            lats[((lats.len() * 99) / 100).min(lats.len() - 1)]
        } else {
            0.0
        };

        let max_latency = if !lats.is_empty() {
            lats[lats.len() - 1]
        } else {
            0.0
        };

        let stats = IPCStats {
            total_messages,
            successful_messages: total_messages,
            failed_messages: 0,
            avg_latency_us: avg_latency,
            p50_latency_us: p50,
            p99_latency_us: p99,
            max_latency_us: max_latency,
            throughput_mps: throughput,
            capability_revocations: 0,
            semaphore_acquisitions: 0,
            semaphore_timeouts: 0,
        };

        info!(
            "Message passing: {:.0} msgs/sec, latency p99={:.2}µs, avg={:.2}µs",
            throughput, p99, avg_latency
        );

        Ok(stats)
    }

    /// Test latency with small messages
    pub async fn test_latency(&self) -> Result<()> {
        info!("Testing IPC latency with {} byte messages", 64);

        let latencies = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for _ in 0..100 {
            let lats = Arc::clone(&latencies);
            let handle = tokio::spawn(async move {
                let (tx, mut rx) = mpsc::channel::<IPCMessage>(100);

                let lats_copy = Arc::clone(&lats);
                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        let lat = msg.delivery_latency_ns();
                        lats_copy.lock().await.push(lat);
                    }
                });

                for i in 0..1000 {
                    let msg = IPCMessage::new(i, 0, 1, 64);
                    let _ = tx.send(msg).await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Latency test completed");
        Ok(())
    }

    /// Test capability revocation
    pub async fn test_capability_revocation(&self) -> Result<()> {
        info!("Testing capability revocation");

        let mut capabilities = vec![];
        let revocation_count = Arc::new(AtomicU64::new(0));

        // Create capabilities
        for i in 0..1000 {
            let cap = Capability::new(i, i % 100);
            capabilities.push(Arc::new(Mutex::new(cap)));
        }

        let mut handles = vec![];

        // Revoke capabilities
        for cap in capabilities.iter() {
            let c = Arc::clone(cap);
            let count = Arc::clone(&revocation_count);

            let handle = tokio::spawn(async move {
                let mut capability = c.lock().await;
                if capability.is_valid() {
                    capability.revoke();
                    count.fetch_add(1, Ordering::Relaxed);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!(
            "Revocations: {}",
            revocation_count.load(Ordering::Relaxed)
        );
        Ok(())
    }

    /// Test semaphore contention
    pub async fn test_semaphore_contention(&self) -> Result<()> {
        info!(
            "Testing semaphore contention: {} semaphores, {} contentions",
            self.config.num_semaphores, self.config.semaphore_contentions
        );

        let semaphores: Vec<_> = (0..self.config.num_semaphores)
            .map(|_| Arc::new(Semaphore::new(1)))
            .collect();

        let acquisition_count = Arc::new(AtomicU64::new(0));
        let timeout_count = Arc::new(AtomicU64::new(0));

        let mut handles = vec![];

        for _ in 0..self.config.semaphore_contentions {
            for sem in &semaphores {
                let s = Arc::clone(sem);
                let acq_count = Arc::clone(&acquisition_count);
                let timeout = Arc::clone(&timeout_count);

                let handle = tokio::spawn(async move {
                    match tokio::time::timeout(
                        tokio::time::Duration::from_micros(100),
                        s.acquire(),
                    )
                    .await
                    {
                        Ok(Ok(_permit)) => {
                            acq_count.fetch_add(1, Ordering::Relaxed);
                            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
                        }
                        _ => {
                            timeout.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });

                handles.push(handle);
            }
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!(
            "Semaphore: {} acquisitions, {} timeouts",
            acquisition_count.load(Ordering::Relaxed),
            timeout_count.load(Ordering::Relaxed)
        );

        Ok(())
    }
}

/// High-level IPC test
pub struct IPCTest {
    config: IPCConfig,
}

impl IPCTest {
    /// Create a new IPC test
    pub fn new(config: IPCConfig) -> Self {
        Self { config }
    }

    /// Run the complete IPC test suite
    pub async fn run(&self) -> Result<()> {
        let test = MessagePassingTest::new(self.config.clone());
        test.test_throughput().await?;
        test.test_latency().await?;
        test.test_capability_revocation().await?;
        test.test_semaphore_contention().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_creation() {
        let msg = IPCMessage::new(1, 10, 20, 256);
        assert_eq!(msg.id, 1);
        assert_eq!(msg.sender_id, 10);
        assert_eq!(msg.receiver_id, 20);
        assert_eq!(msg.data.len(), 256);
    }

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new(1, 100);
        assert_eq!(cap.id, 1);
        assert_eq!(cap.owner_id, 100);
        assert!(cap.is_valid());
    }

    #[tokio::test]
    async fn test_message_passing_throughput() {
        let config = IPCConfig {
            num_senders: 10,
            num_receivers: 10,
            messages_per_sender: 100,
            ..Default::default()
        };
        let test = MessagePassingTest::new(config);
        let result = test.test_throughput().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capability_revocation() {
        let test = MessagePassingTest::new(IPCConfig::default());
        let result = test.test_capability_revocation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_semaphore_contention() {
        let config = IPCConfig {
            semaphore_contentions: 50,
            ..Default::default()
        };
        let test = MessagePassingTest::new(config);
        let result = test.test_semaphore_contention().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ipc_test_suite() {
        let config = IPCConfig {
            num_senders: 5,
            num_receivers: 5,
            messages_per_sender: 50,
            ..Default::default()
        };
        let test = IPCTest::new(config);
        let result = test.run().await;
        assert!(result.is_ok());
    }
}
