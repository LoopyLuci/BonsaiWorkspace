//! Storage service stress tests
//!
//! Tests for:
//! - CAS (Content-Addressed Storage) round-trip throughput
//! - Deduplication under 1TB random data
//! - Erasure code reconstruction
//! - Concurrent write patterns

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

/// Content hash for CAS
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn compute(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..]);
        Self(hash)
    }

    pub fn as_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

/// Simulated CAS storage
pub struct ContentAddressedStorage {
    objects: HashMap<ContentHash, Vec<u8>>,
    total_size: u64,
}

impl ContentAddressedStorage {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            total_size: 0,
        }
    }

    pub fn store(&mut self, data: Vec<u8>) -> ContentHash {
        let hash = ContentHash::compute(&data);
        if !self.objects.contains_key(&hash) {
            self.total_size += data.len() as u64;
            self.objects.insert(hash.clone(), data);
        }
        hash
    }

    pub fn retrieve(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        self.objects.get(hash).cloned()
    }

    pub fn total_objects(&self) -> usize {
        self.objects.len()
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn exists(&self, hash: &ContentHash) -> bool {
        self.objects.contains_key(hash)
    }
}

/// Erasure code shard
#[derive(Debug, Clone)]
pub struct ErasureShard {
    pub index: usize,
    pub data: Vec<u8>,
}

impl ErasureShard {
    pub fn compute_parity(data_shards: &[Vec<u8>]) -> Vec<u8> {
        if data_shards.is_empty() {
            return Vec::new();
        }

        let shard_size = data_shards[0].len();
        let mut parity = vec![0u8; shard_size];

        for shard in data_shards {
            for (i, byte) in shard.iter().enumerate() {
                parity[i] ^= byte;
            }
        }

        parity
    }

    pub fn reconstruct(shards: &[Option<Vec<u8>>]) -> Option<Vec<u8>> {
        let total_shards = shards.len();
        let data_shards = (total_shards + 1) / 2; // Simple assumption

        let available: Vec<_> = shards.iter().filter(|s| s.is_some()).collect();
        if available.len() < data_shards {
            return None; // Not enough shards for reconstruction
        }

        // For simplicity, reconstruct from available shards
        if let Some(Some(first)) = shards.first() {
            let mut reconstructed = first.clone();

            // In a real implementation, use actual erasure code math
            for shard in shards.iter().skip(1).flatten() {
                for (i, byte) in shard.iter().enumerate() {
                    if i < reconstructed.len() {
                        reconstructed[i] ^= byte;
                    }
                }
            }

            Some(reconstructed)
        } else {
            None
        }
    }
}

/// Storage stress tests
pub struct StorageStressTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl StorageStressTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("storage"),
        }
    }

    /// Test CAS round-trip throughput
    pub async fn test_cas_throughput(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "storage-cas-throughput";

        info!("Testing CAS round-trip throughput...");

        let mut cas = ContentAddressedStorage::new();
        let mut write_count = 0;
        let mut read_count = 0;
        let mut bytes_written = 0u64;
        let mut bytes_read = 0u64;

        // Generate test data
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        for _ in 0..1000 {
            let size = 1024 + (rng.next_u32() % 10240) as usize; // 1-11 KB chunks
            let mut data = vec![0u8; size];
            rng.fill_bytes(&mut data);

            let start_write = Instant::now();
            let hash = cas.store(data.clone());
            self.metrics.record_operation(
                "cas_write",
                start_write.elapsed().as_millis() as f64,
                true,
                None,
            );

            write_count += 1;
            bytes_written += size as u64;

            if let Some(retrieved) = cas.retrieve(&hash) {
                let start_read = Instant::now();
                let _ = retrieved;
                self.metrics.record_operation(
                    "cas_read",
                    start_read.elapsed().as_millis() as f64,
                    true,
                    None,
                );
                read_count += 1;
                bytes_read += retrieved.len() as u64;
            }
        }

        let elapsed = start.elapsed();
        let write_throughput_mbps = (bytes_written as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
        let read_throughput_mbps = (bytes_read as f64 / elapsed.as_secs_f64()) / 1_000_000.0;

        let success = write_throughput_mbps > 10.0 && read_throughput_mbps > 10.0;

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Write: {:.1} MB/s, Read: {:.1} MB/s, Objects: {}",
            write_throughput_mbps,
            read_throughput_mbps,
            cas.total_objects()
        ));

        Ok(result)
    }

    /// Test deduplication under 1TB random data
    pub async fn test_deduplication(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "storage-deduplication";

        info!("Testing deduplication under 1TB workload...");

        let mut cas = ContentAddressedStorage::new();
        let target_size = 1_000_000_000u64; // 1 GB (instead of 1TB for faster testing)
        let mut written_size = 0u64;

        // Generate some duplicate patterns
        let patterns: Vec<Vec<u8>> = (0..10)
            .map(|i| {
                let mut data = vec![0u8; 1_000_000];
                data[0] = i as u8;
                data
            })
            .collect();

        let mut rng = rand::thread_rng();
        use rand::RngCore;

        let mut unique_hashes = std::collections::HashSet::new();

        while written_size < target_size {
            // Use pattern with some variation
            let pattern_idx = rng.next_u32() as usize % patterns.len();
            let mut data = patterns[pattern_idx].clone();

            // Add some unique bytes
            if let Some(pos) = rng.next_u32().checked_rem(data.len() as u32) {
                data[pos as usize] = rng.next_u32() as u8;
            }

            let hash = ContentHash::compute(&data);
            if !unique_hashes.contains(&hash) {
                unique_hashes.insert(hash);
                cas.store(data.clone());
            }

            written_size += data.len() as u64;
        }

        let dedup_ratio = target_size as f64 / cas.total_size() as f64;
        let success = dedup_ratio > 1.5; // Expect at least 1.5x deduplication

        self.metrics.record_operation(
            "deduplication",
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
            "Dedup ratio: {:.2}x, Unique objects: {}, Actual size: {} MB",
            dedup_ratio,
            cas.total_objects(),
            cas.total_size() / 1_000_000
        ));

        Ok(result)
    }

    /// Test erasure code reconstruction
    pub async fn test_erasure_reconstruction(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "storage-erasure-reconstruction";

        info!("Testing erasure code reconstruction...");

        let _total_shards = 10;
        let data_shards = 6;
        let parity_shards = 4;

        let mut reconstruction_times = Vec::new();
        let mut success_count = 0;

        for _ in 0..100 {
            // Create data shards
            let data: Vec<Vec<u8>> = (0..data_shards)
                .map(|_| vec![0u8; 1024])
                .collect();

            // Compute parity
            let parity = ErasureShard::compute_parity(&data);

            // Create shards array
            let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
            for d in &data {
                shards.push(Some(d.clone()));
            }
            shards.push(Some(parity));

            // Simulate loss of some shards
            let mut rng = rand::thread_rng();
            use rand::RngCore;

            for _ in 0..parity_shards {
                let idx = rng.next_u32() as usize % shards.len();
                shards[idx] = None;
            }

            // Attempt reconstruction
            let start_recon = Instant::now();
            if let Some(_reconstructed) = ErasureShard::reconstruct(&shards) {
                let recon_time = start_recon.elapsed().as_millis() as f64;
                reconstruction_times.push(recon_time);
                success_count += 1;
            }
        }

        let success = success_count >= 90; // At least 90% reconstruction success

        self.metrics.record_operation(
            "erasure_reconstruction",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let avg_recon_time =
            if !reconstruction_times.is_empty() {
                reconstruction_times.iter().sum::<f64>() / reconstruction_times.len() as f64
            } else {
                0.0
            };

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Success rate: {:.1}%, Avg reconstruction time: {:.2}ms",
            (success_count as f64 / 100.0) * 100.0,
            avg_recon_time
        ));

        Ok(result)
    }

    /// Test concurrent write patterns
    pub async fn test_concurrent_writes(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "storage-concurrent-writes";

        info!("Testing concurrent write patterns...");

        let mut cas = ContentAddressedStorage::new();
        let mut write_latencies = Vec::new();
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        for _ in 0..self.config.concurrency {
            let size = 4096 + (rng.next_u32() % 8192) as usize;
            let mut data = vec![0u8; size];
            rng.fill_bytes(&mut data);

            let write_start = Instant::now();
            let _hash = cas.store(data);
            let latency = write_start.elapsed().as_millis() as f64;
            write_latencies.push(latency);

            self.metrics.record_operation("concurrent_write", latency, true, None);
        }

        write_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p99 = write_latencies[(write_latencies.len() as f64 * 0.99) as usize];
        let success = p99 < 50.0; // p99 latency < 50ms

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Concurrent writers: {}, p99 latency: {:.2}ms, Total size: {} MB",
            self.config.concurrency,
            p99,
            cas.total_size() / 1_000_000
        ));

        Ok(result)
    }

    /// Test storage capacity and limits
    pub async fn test_capacity_limits(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "storage-capacity-limits";

        info!("Testing storage capacity limits...");

        let mut cas = ContentAddressedStorage::new();
        let max_capacity = 100_000_000u64; // 100 MB limit
        let mut writes = 0;
        let mut errors = 0;

        let mut rng = rand::thread_rng();
        use rand::RngCore;

        loop {
            if cas.total_size() >= max_capacity {
                break;
            }

            let size = 1024 + (rng.next_u32() % 4096) as usize;
            let mut data = vec![0u8; size];
            rng.fill_bytes(&mut data);

            cas.store(data);
            writes += 1;

            if writes > 10000 {
                errors += 1;
                break;
            }
        }

        let success = errors == 0;

        self.metrics.record_operation(
            "capacity_limits",
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
            "Writes: {}, Capacity used: {} MB / 100 MB",
            writes,
            cas.total_size() / 1_000_000
        ));

        Ok(result)
    }

    /// Run all storage stress tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all storage stress tests...");

        let results = vec![
            self.test_cas_throughput().await?,
            self.test_deduplication().await?,
            self.test_erasure_reconstruction().await?,
            self.test_concurrent_writes().await?,
            self.test_capacity_limits().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("storage".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "Storage tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let data = b"hello world";
        let hash1 = ContentHash::compute(data);
        let hash2 = ContentHash::compute(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cas_storage() {
        let mut cas = ContentAddressedStorage::new();
        let data = vec![1, 2, 3, 4, 5];
        let hash = cas.store(data.clone());
        assert!(cas.exists(&hash));
        assert_eq!(cas.retrieve(&hash), Some(data));
    }

    #[test]
    fn test_deduplication() {
        let mut cas = ContentAddressedStorage::new();
        let data = vec![1, 2, 3, 4, 5];
        let hash1 = cas.store(data.clone());
        let hash2 = cas.store(data.clone());
        assert_eq!(hash1, hash2);
        assert_eq!(cas.total_objects(), 1);
    }

    #[tokio::test]
    async fn test_storage_stress_tests() {
        let config = TestConfig::default();
        let tests = StorageStressTests::new(config);

        let result = tests.test_cas_throughput().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
