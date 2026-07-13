//! Test plan definitions and fault specifications
//!
//! Provides the core types for defining stress test plans,
//! including fault definitions, workload parameters, and resource constraints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of fault that can be injected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaultType {
    /// CPU stress fault
    CpuStress,
    /// Memory exhaustion fault
    MemoryExhaustion,
    /// Disk I/O stress fault
    DiskIoStress,
    /// Network packet loss
    NetworkPacketLoss,
    /// Network latency
    NetworkLatency,
    /// Kernel panic simulation
    KernelPanic,
    /// Task scheduling stress
    TaskSchedulingStress,
    /// File descriptor exhaustion
    FileDescriptorExhaustion,
    /// Process termination (SIGKILL)
    ProcessTermination,
    /// Signal injection
    SignalInjection,
    /// Custom fault (user-defined)
    Custom(String),
}

impl FaultType {
    /// Get a human-readable name
    pub fn name(&self) -> String {
        match self {
            Self::CpuStress => "CPU Stress".to_string(),
            Self::MemoryExhaustion => "Memory Exhaustion".to_string(),
            Self::DiskIoStress => "Disk I/O Stress".to_string(),
            Self::NetworkPacketLoss => "Network Packet Loss".to_string(),
            Self::NetworkLatency => "Network Latency".to_string(),
            Self::KernelPanic => "Kernel Panic".to_string(),
            Self::TaskSchedulingStress => "Task Scheduling Stress".to_string(),
            Self::FileDescriptorExhaustion => "File Descriptor Exhaustion".to_string(),
            Self::ProcessTermination => "Process Termination".to_string(),
            Self::SignalInjection => "Signal Injection".to_string(),
            Self::Custom(name) => format!("Custom: {}", name),
        }
    }
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Definition of a fault to be injected during testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultDefinition {
    /// Unique identifier for this fault
    pub id: String,
    /// Type of fault
    pub fault_type: FaultType,
    /// When to inject the fault (seconds since test start)
    pub inject_at_secs: u64,
    /// Duration of the fault in seconds
    pub duration_secs: u64,
    /// Parameters specific to the fault type
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether this fault should be injected
    pub enabled: bool,
}

impl FaultDefinition {
    /// Create a new fault definition
    pub fn new(
        id: impl Into<String>,
        fault_type: FaultType,
        inject_at_secs: u64,
        duration_secs: u64,
    ) -> Self {
        Self {
            id: id.into(),
            fault_type,
            inject_at_secs,
            duration_secs,
            parameters: HashMap::new(),
            enabled: true,
        }
    }

    /// Add a parameter to this fault definition
    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    /// Validate this fault definition
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        if self.id.is_empty() {
            return Err(SrwstsError::InvalidFaultParameters {
                reason: "fault id cannot be empty".to_string(),
            });
        }

        if self.duration_secs == 0 {
            return Err(SrwstsError::InvalidFaultParameters {
                reason: "fault duration must be > 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Workload definition for a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    /// Unique identifier for this workload
    pub id: String,
    /// Type/name of the workload
    pub workload_type: String,
    /// Number of concurrent tasks/threads
    pub concurrency: u32,
    /// Duration of the workload in seconds
    pub duration_secs: u64,
    /// Operations per second target
    pub ops_per_sec: u64,
    /// Custom workload parameters
    pub parameters: HashMap<String, String>,
}

impl Workload {
    /// Create a new workload
    pub fn new(
        id: impl Into<String>,
        workload_type: impl Into<String>,
        concurrency: u32,
        duration_secs: u64,
    ) -> Self {
        Self {
            id: id.into(),
            workload_type: workload_type.into(),
            concurrency,
            duration_secs,
            ops_per_sec: 0,
            parameters: HashMap::new(),
        }
    }

    /// Validate this workload definition
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        if self.id.is_empty() {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "workload id cannot be empty".to_string(),
            });
        }

        if self.concurrency == 0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "workload concurrency must be > 0".to_string(),
            });
        }

        if self.duration_secs == 0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "workload duration must be > 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Resource limits for test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage in percentage (0-100)
    pub max_cpu_percent: f64,
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum disk space in bytes
    pub max_disk_bytes: u64,
    /// Maximum number of file descriptors
    pub max_file_descriptors: u32,
    /// Maximum number of threads
    pub max_threads: u32,
    /// Maximum network bandwidth in Mbps
    pub max_network_mbps: u64,
}

impl ResourceLimits {
    /// Create resource limits with default values
    pub fn new() -> Self {
        Self {
            max_cpu_percent: 100.0,
            max_memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
            max_disk_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
            max_file_descriptors: 65536,
            max_threads: 1024,
            max_network_mbps: 1000,
        }
    }

    /// Validate these resource limits
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        if self.max_cpu_percent <= 0.0 || self.max_cpu_percent > 100.0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "max_cpu_percent must be between 0 and 100".to_string(),
            });
        }

        if self.max_memory_bytes == 0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "max_memory_bytes must be > 0".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete test plan specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlan {
    /// Unique identifier for this test plan
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this test validates
    pub description: String,
    /// Version of this test plan
    pub version: String,
    /// Workloads to execute
    pub workloads: Vec<Workload>,
    /// Faults to inject
    pub faults: Vec<FaultDefinition>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Maximum total test duration in seconds
    pub max_duration_secs: u64,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl TestPlan {
    /// Create a new test plan
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            version: "0.1.0".to_string(),
            workloads: Vec::new(),
            faults: Vec::new(),
            resource_limits: ResourceLimits::new(),
            max_duration_secs: 3600, // 1 hour default
            metadata: HashMap::new(),
        }
    }

    /// Add a workload to this test plan
    pub fn with_workload(mut self, workload: Workload) -> Self {
        self.workloads.push(workload);
        self
    }

    /// Add a fault to this test plan
    pub fn with_fault(mut self, fault: FaultDefinition) -> Self {
        self.faults.push(fault);
        self
    }

    /// Add metadata to this test plan
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Validate the entire test plan
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        if self.id.is_empty() {
            return Err(SrwstsError::InvalidTestPlan {
                reason: "test plan id cannot be empty".to_string(),
            });
        }

        if self.workloads.is_empty() {
            return Err(SrwstsError::InvalidTestPlan {
                reason: "test plan must have at least one workload".to_string(),
            });
        }

        // Validate each workload
        for workload in &self.workloads {
            workload.validate()?;
        }

        // Validate each fault
        for fault in &self.faults {
            fault.validate()?;
        }

        // Validate resource limits
        self.resource_limits.validate()?;

        // Validate max_duration is reasonable
        if self.max_duration_secs < self.workloads.iter().map(|w| w.duration_secs).max().unwrap_or(0) {
            return Err(SrwstsError::InvalidTestPlan {
                reason: "max_duration_secs must be >= longest workload duration".to_string(),
            });
        }

        Ok(())
    }

    /// Get the longest workload duration
    pub fn max_workload_duration(&self) -> u64 {
        self.workloads.iter().map(|w| w.duration_secs).max().unwrap_or(0)
    }

    /// Get the total number of faults
    pub fn total_faults(&self) -> usize {
        self.faults.iter().filter(|f| f.enabled).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_type_name() {
        assert_eq!(FaultType::CpuStress.name(), "CPU Stress");
        assert!(FaultType::Custom("custom".to_string()).name().contains("Custom"));
    }

    #[test]
    fn test_fault_definition_validation() {
        let fault = FaultDefinition::new("f1", FaultType::CpuStress, 0, 10);
        assert!(fault.validate().is_ok());

        let invalid = FaultDefinition::new("", FaultType::CpuStress, 0, 10);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_workload_validation() {
        let workload = Workload::new("w1", "cpu-stress", 4, 300);
        assert!(workload.validate().is_ok());

        let invalid = Workload::new("w1", "cpu-stress", 0, 300);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_resource_limits_validation() {
        let limits = ResourceLimits::new();
        assert!(limits.validate().is_ok());

        let mut invalid = ResourceLimits::new();
        invalid.max_cpu_percent = 150.0;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_test_plan_validation() {
        let plan = TestPlan::new("test1", "My Test", "Test description")
            .with_workload(Workload::new("w1", "cpu-stress", 4, 300));
        assert!(plan.validate().is_ok());

        let invalid = TestPlan::new("", "My Test", "Test description")
            .with_workload(Workload::new("w1", "cpu-stress", 4, 300));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_test_plan_max_workload_duration() {
        let plan = TestPlan::new("test1", "My Test", "Test description")
            .with_workload(Workload::new("w1", "cpu-stress", 4, 300))
            .with_workload(Workload::new("w2", "memory-stress", 2, 600));
        assert_eq!(plan.max_workload_duration(), 600);
    }

    #[test]
    fn test_fault_definition_with_parameters() {
        let fault = FaultDefinition::new("f1", FaultType::CpuStress, 10, 60)
            .with_parameter("cpu_count", serde_json::json!(4));
        assert!(fault.parameters.contains_key("cpu_count"));
    }
}
