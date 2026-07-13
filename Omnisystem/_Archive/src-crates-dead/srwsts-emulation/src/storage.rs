//! Storage emulator (NVMe, SATA, RAM disk) with IOPS/latency models

use crate::errors::EmulationResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Storage device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// NVMe SSD
    NVMe,
    /// SATA SSD
    SataSsd,
    /// SATA HDD
    SataHdd,
    /// RAM disk (memory-backed)
    RamDisk,
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NVMe => write!(f, "NVMe"),
            Self::SataSsd => write!(f, "SATA SSD"),
            Self::SataHdd => write!(f, "SATA HDD"),
            Self::RamDisk => write!(f, "RAM Disk"),
        }
    }
}

/// Storage device profile with latency and IOPS models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProfile {
    /// Device type
    pub device_type: StorageType,
    /// Capacity in bytes
    pub capacity_bytes: u64,
    /// Random read IOPS
    pub random_read_iops: u64,
    /// Random write IOPS
    pub random_write_iops: u64,
    /// Sequential read throughput in MB/s
    pub seq_read_mbps: u64,
    /// Sequential write throughput in MB/s
    pub seq_write_mbps: u64,
    /// Average read latency in microseconds
    pub avg_read_latency_us: u32,
    /// Average write latency in microseconds
    pub avg_write_latency_us: u32,
    /// Queue depth
    pub queue_depth: usize,
}

impl StorageProfile {
    /// Create a profile for high-end NVMe (PCIe 4.0)
    pub fn nvme_gen4(capacity_gb: u64) -> Self {
        Self {
            device_type: StorageType::NVMe,
            capacity_bytes: capacity_gb * 1024 * 1024 * 1024,
            random_read_iops: 600_000,
            random_write_iops: 300_000,
            seq_read_mbps: 7000,
            seq_write_mbps: 5500,
            avg_read_latency_us: 30,
            avg_write_latency_us: 50,
            queue_depth: 32,
        }
    }

    /// Create a profile for SATA SSD
    pub fn sata_ssd(capacity_gb: u64) -> Self {
        Self {
            device_type: StorageType::SataSsd,
            capacity_bytes: capacity_gb * 1024 * 1024 * 1024,
            random_read_iops: 90_000,
            random_write_iops: 70_000,
            seq_read_mbps: 550,
            seq_write_mbps: 450,
            avg_read_latency_us: 100,
            avg_write_latency_us: 150,
            queue_depth: 32,
        }
    }

    /// Create a profile for SATA HDD
    pub fn sata_hdd(capacity_gb: u64) -> Self {
        Self {
            device_type: StorageType::SataHdd,
            capacity_bytes: capacity_gb * 1024 * 1024 * 1024,
            random_read_iops: 150,
            random_write_iops: 120,
            seq_read_mbps: 150,
            seq_write_mbps: 120,
            avg_read_latency_us: 8000,
            avg_write_latency_us: 9000,
            queue_depth: 32,
        }
    }

    /// Create a profile for RAM disk
    pub fn ram_disk(capacity_gb: u64) -> Self {
        Self {
            device_type: StorageType::RamDisk,
            capacity_bytes: capacity_gb * 1024 * 1024 * 1024,
            random_read_iops: 1_000_000,
            random_write_iops: 1_000_000,
            seq_read_mbps: 50_000,
            seq_write_mbps: 50_000,
            avg_read_latency_us: 1,
            avg_write_latency_us: 1,
            queue_depth: 256,
        }
    }
}

/// Storage configuration with multiple devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage devices
    pub devices: Vec<StorageProfile>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            devices: vec![
                StorageProfile::nvme_gen4(1),  // 1 TB NVMe
                StorageProfile::sata_ssd(2),   // 2 TB SATA SSD
            ],
        }
    }
}

impl StorageConfig {
    /// Create a minimal configuration (single RAM disk)
    pub fn minimal() -> Self {
        Self {
            devices: vec![StorageProfile::ram_disk(1)],
        }
    }

    /// Create a high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            devices: vec![
                StorageProfile::nvme_gen4(2),
                StorageProfile::nvme_gen4(2),
            ],
        }
    }

    /// Calculate total capacity across all devices
    pub fn total_capacity(&self) -> u64 {
        self.devices.iter().map(|d| d.capacity_bytes).sum::<u64>()
    }
}

/// Storage access statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total read operations
    pub read_ops: u64,
    /// Total write operations
    pub write_ops: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Total accumulated latency for reads (microseconds)
    pub total_read_latency_us: u64,
    /// Total accumulated latency for writes (microseconds)
    pub total_write_latency_us: u64,
}

impl StorageStats {
    /// Average read latency in microseconds
    pub fn avg_read_latency_us(&self) -> f64 {
        if self.read_ops == 0 {
            return 0.0;
        }
        self.total_read_latency_us as f64 / self.read_ops as f64
    }

    /// Average write latency in microseconds
    pub fn avg_write_latency_us(&self) -> f64 {
        if self.write_ops == 0 {
            return 0.0;
        }
        self.total_write_latency_us as f64 / self.write_ops as f64
    }

    /// Throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        let total_bytes = self.bytes_read + self.bytes_written;
        let total_time_us = self.total_read_latency_us + self.total_write_latency_us;

        if total_time_us == 0 {
            return 0.0;
        }

        (total_bytes as f64 / 1024.0 / 1024.0) / (total_time_us as f64 / 1_000_000.0)
    }
}

/// Storage emulator trait
#[async_trait]
pub trait StorageEmulator: Send + Sync {
    /// Read from storage
    async fn read(&self, address: u64, size: usize) -> EmulationResult<Vec<u8>>;

    /// Write to storage
    async fn write(&self, address: u64, data: &[u8]) -> EmulationResult<()>;

    /// Reset storage to initial state
    async fn reset(&self) -> EmulationResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_profile_nvme_gen4() {
        let profile = StorageProfile::nvme_gen4(2);
        assert_eq!(profile.device_type, StorageType::NVMe);
        assert_eq!(profile.capacity_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(profile.random_read_iops, 600_000);
    }

    #[test]
    fn test_storage_profile_sata_ssd() {
        let profile = StorageProfile::sata_ssd(1);
        assert_eq!(profile.device_type, StorageType::SataSsd);
        assert_eq!(profile.random_read_iops, 90_000);
    }

    #[test]
    fn test_storage_profile_sata_hdd() {
        let profile = StorageProfile::sata_hdd(4);
        assert_eq!(profile.device_type, StorageType::SataHdd);
        assert_eq!(profile.random_read_iops, 150);
    }

    #[test]
    fn test_storage_profile_ram_disk() {
        let profile = StorageProfile::ram_disk(1);
        assert_eq!(profile.device_type, StorageType::RamDisk);
        assert_eq!(profile.random_read_iops, 1_000_000);
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.devices.len(), 2);
    }

    #[test]
    fn test_storage_config_total_capacity() {
        let config = StorageConfig::default();
        let total = config.total_capacity();
        assert!(total > 0);
    }

    #[test]
    fn test_storage_stats_avg_latency() {
        let mut stats = StorageStats::default();
        stats.read_ops = 100;
        stats.total_read_latency_us = 5000;

        assert_eq!(stats.avg_read_latency_us(), 50.0);
    }

    #[test]
    fn test_storage_stats_throughput() {
        let mut stats = StorageStats::default();
        stats.bytes_read = 1024 * 1024; // 1 MB
        stats.total_read_latency_us = 1_000_000; // 1 second

        assert_eq!(stats.throughput_mbps(), 1.0);
    }

    #[test]
    fn test_storage_type_display() {
        assert_eq!(StorageType::NVMe.to_string(), "NVMe");
        assert_eq!(StorageType::SataSsd.to_string(), "SATA SSD");
        assert_eq!(StorageType::SataHdd.to_string(), "SATA HDD");
        assert_eq!(StorageType::RamDisk.to_string(), "RAM Disk");
    }
}
