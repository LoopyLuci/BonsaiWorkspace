//! Fault type definitions and specifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique fault identifier.
pub type FaultId = Uuid;

/// Fault category enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaultTypeKind {
    Memory,
    Cpu,
    Network,
    Storage,
    Time,
    Hardware,
}

impl FaultTypeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FaultTypeKind::Memory => "Memory",
            FaultTypeKind::Cpu => "CPU",
            FaultTypeKind::Network => "Network",
            FaultTypeKind::Storage => "Storage",
            FaultTypeKind::Time => "Time",
            FaultTypeKind::Hardware => "Hardware",
        }
    }
}

impl std::fmt::Display for FaultTypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Specific fault types with parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum FaultType {
    /// Memory pressure: increase system memory pressure.
    MemoryPressure {
        /// Percentage of memory to consume (0-100).
        pressure_percent: u8,
        /// Page faults to inject.
        page_faults: u32,
    },

    /// Out-of-memory: trigger OOM killer.
    OutOfMemory {
        /// Target PID (0 = random process).
        target_pid: u32,
    },

    /// Allocation failures: block certain allocations.
    AllocationFailure {
        /// Percentage of allocations to fail (0-100).
        failure_rate: u8,
        /// Minimum allocation size to target (bytes).
        min_size: u64,
    },

    /// CPU overload: consume CPU cycles.
    CpuOverload {
        /// Percentage of CPU to consume per core (0-100).
        cpu_percent: u8,
        /// Number of cores to stress (0 = all).
        core_count: u8,
    },

    /// Cache contention: increase cache misses.
    CacheContention {
        /// Working set size in bytes.
        working_set_size: u64,
        /// Cache thrashing intensity (0-100).
        intensity: u8,
    },

    /// Thermal throttling simulation.
    ThermalThrottling {
        /// Temperature threshold in Celsius.
        temperature: u8,
        /// Throttle percentage (0-100).
        throttle_percent: u8,
    },

    /// Network partition: drop packets between nodes.
    NetworkPartition {
        /// CIDR blocks to isolate.
        isolated_subnets: Vec<String>,
        /// Partition duration (will be enforced separately).
        affect_bidirectional: bool,
    },

    /// Network latency: introduce packet delay.
    NetworkLatency {
        /// Latency in milliseconds.
        latency_ms: u32,
        /// Jitter in milliseconds (±).
        jitter_ms: u32,
        /// Affected ports (empty = all).
        ports: Vec<u16>,
    },

    /// Packet loss: drop random packets.
    PacketLoss {
        /// Loss percentage (0-100).
        loss_percent: u8,
        /// Correlation: loss burst probability (0-100).
        correlation_percent: u8,
    },

    /// Disk full: consume disk space.
    DiskFull {
        /// Filesystem mount point.
        mount_point: String,
        /// Percentage of disk to fill (0-100).
        fill_percent: u8,
    },

    /// I/O error injection: simulate disk errors.
    IoError {
        /// Failure rate (0-100).
        failure_rate: u8,
        /// Affected filesystem (empty = all).
        filesystem: Option<String>,
        /// Error types: "EIO", "ENOSPC", etc.
        error_codes: Vec<String>,
    },

    /// Data corruption: flip bits in files.
    DataCorruption {
        /// Corruption rate (0-100).
        corruption_rate: u8,
        /// Filesystem paths to target.
        paths: Vec<String>,
    },

    /// Clock skew: alter system time perception.
    ClockSkew {
        /// Time offset in seconds (negative = rewind, positive = forward).
        offset_secs: i64,
        /// Drift rate (nanoseconds per second).
        drift_rate_nanos_per_sec: i32,
    },

    /// Time jump: sudden time change.
    TimeJump {
        /// Jump offset in seconds.
        offset_secs: i64,
    },

    /// GPU reset: trigger GPU device reset.
    GpuReset {
        /// GPU device ID.
        device_id: u8,
    },

    /// Power failure: simulate power loss.
    PowerFailure {
        /// Recovery time in seconds (before simulated restart).
        recovery_secs: u32,
    },

    /// Thermal shutdown: simulate thermal emergency.
    ThermalShutdown {
        /// Temperature threshold in Celsius.
        temperature: u8,
    },

    /// Fan failure: disable cooling.
    FanFailure {
        /// Fan identifier.
        fan_id: u8,
    },
}

impl FaultType {
    /// Get the category of this fault.
    pub fn category(&self) -> FaultTypeKind {
        match self {
            FaultType::MemoryPressure { .. }
            | FaultType::OutOfMemory { .. }
            | FaultType::AllocationFailure { .. } => FaultTypeKind::Memory,

            FaultType::CpuOverload { .. }
            | FaultType::CacheContention { .. }
            | FaultType::ThermalThrottling { .. } => FaultTypeKind::Cpu,

            FaultType::NetworkPartition { .. }
            | FaultType::NetworkLatency { .. }
            | FaultType::PacketLoss { .. } => FaultTypeKind::Network,

            FaultType::DiskFull { .. }
            | FaultType::IoError { .. }
            | FaultType::DataCorruption { .. } => FaultTypeKind::Storage,

            FaultType::ClockSkew { .. } | FaultType::TimeJump { .. } => FaultTypeKind::Time,

            FaultType::GpuReset { .. }
            | FaultType::PowerFailure { .. }
            | FaultType::ThermalShutdown { .. }
            | FaultType::FanFailure { .. } => FaultTypeKind::Hardware,
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            FaultType::MemoryPressure { .. } => "MemoryPressure",
            FaultType::OutOfMemory { .. } => "OutOfMemory",
            FaultType::AllocationFailure { .. } => "AllocationFailure",
            FaultType::CpuOverload { .. } => "CpuOverload",
            FaultType::CacheContention { .. } => "CacheContention",
            FaultType::ThermalThrottling { .. } => "ThermalThrottling",
            FaultType::NetworkPartition { .. } => "NetworkPartition",
            FaultType::NetworkLatency { .. } => "NetworkLatency",
            FaultType::PacketLoss { .. } => "PacketLoss",
            FaultType::DiskFull { .. } => "DiskFull",
            FaultType::IoError { .. } => "IoError",
            FaultType::DataCorruption { .. } => "DataCorruption",
            FaultType::ClockSkew { .. } => "ClockSkew",
            FaultType::TimeJump { .. } => "TimeJump",
            FaultType::GpuReset { .. } => "GpuReset",
            FaultType::PowerFailure { .. } => "PowerFailure",
            FaultType::ThermalShutdown { .. } => "ThermalShutdown",
            FaultType::FanFailure { .. } => "FanFailure",
        }
    }

    /// Check if this is a destructive fault (cannot be easily recovered).
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            FaultType::DataCorruption { .. }
                | FaultType::PowerFailure { .. }
                | FaultType::GpuReset { .. }
        )
    }

    /// Check if this is a transient fault.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            FaultType::NetworkLatency { .. }
                | FaultType::PacketLoss { .. }
                | FaultType::CpuOverload { .. }
                | FaultType::MemoryPressure { .. }
        )
    }
}

/// Fault definition with timing and execution parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultDefinition {
    /// Unique fault ID.
    pub id: FaultId,
    /// The fault to inject.
    pub fault_type: FaultType,
    /// When to inject (epoch seconds).
    pub inject_at: u64,
    /// How long to maintain the fault (seconds).
    pub duration_secs: u64,
    /// When this definition was created.
    pub created_at: DateTime<Utc>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl FaultDefinition {
    /// Create a new fault definition.
    pub fn new(fault_type: FaultType, inject_at: u64, duration_secs: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            fault_type,
            inject_at,
            duration_secs,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Get when this fault should recover (inject_at + duration).
    pub fn recovery_time(&self) -> u64 {
        self.inject_at + self.duration_secs
    }

    /// Check if this fault is currently active at a given time.
    pub fn is_active_at(&self, time: u64) -> bool {
        time >= self.inject_at && time < self.recovery_time()
    }

    /// Add metadata to the fault definition.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate the fault definition.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.duration_secs == 0 {
            return Err(crate::error::FaultError::InvalidConfiguration(
                "duration must be > 0".to_string(),
            ));
        }

        match &self.fault_type {
            FaultType::MemoryPressure { pressure_percent, .. } => {
                if *pressure_percent > 100 {
                    return Err(crate::error::FaultError::InvalidConfiguration(
                        "pressure_percent must be 0-100".to_string(),
                    ));
                }
            }
            FaultType::CpuOverload { cpu_percent, .. } => {
                if *cpu_percent > 100 {
                    return Err(crate::error::FaultError::InvalidConfiguration(
                        "cpu_percent must be 0-100".to_string(),
                    ));
                }
            }
            FaultType::DiskFull { fill_percent, .. } => {
                if *fill_percent > 100 {
                    return Err(crate::error::FaultError::InvalidConfiguration(
                        "fill_percent must be 0-100".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_type_category() {
        let fault = FaultType::MemoryPressure {
            pressure_percent: 50,
            page_faults: 1000,
        };
        assert_eq!(fault.category(), FaultTypeKind::Memory);
    }

    #[test]
    fn test_fault_definition_creation() {
        let fault = FaultType::CpuOverload {
            cpu_percent: 80,
            core_count: 4,
        };
        let def = FaultDefinition::new(fault, 10, 5);
        assert_eq!(def.inject_at, 10);
        assert_eq!(def.duration_secs, 5);
        assert_eq!(def.recovery_time(), 15);
    }

    #[test]
    fn test_fault_active_timing() {
        let fault = FaultType::NetworkLatency {
            latency_ms: 100,
            jitter_ms: 10,
            ports: vec![],
        };
        let def = FaultDefinition::new(fault, 10, 5);

        assert!(!def.is_active_at(5));  // Before
        assert!(def.is_active_at(10));  // Start
        assert!(def.is_active_at(14));  // During
        assert!(!def.is_active_at(15)); // After
    }

    #[test]
    fn test_fault_validation() {
        let fault = FaultType::MemoryPressure {
            pressure_percent: 150, // Invalid
            page_faults: 1000,
        };
        let def = FaultDefinition::new(fault, 10, 5);
        assert!(def.validate().is_err());
    }

    #[test]
    fn test_fault_properties() {
        let destructive = FaultType::DataCorruption {
            corruption_rate: 50,
            paths: vec![],
        };
        assert!(destructive.is_destructive());

        let transient = FaultType::NetworkLatency {
            latency_ms: 100,
            jitter_ms: 10,
            ports: vec![],
        };
        assert!(transient.is_transient());
    }
}
