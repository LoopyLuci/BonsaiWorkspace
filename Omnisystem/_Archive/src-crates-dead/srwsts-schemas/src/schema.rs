//! YAML schema definitions for SRWSTS test plans
//!
//! Provides Serde-serializable types that mirror the YAML structure,
//! allowing direct deserialization with validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level test plan YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlanYaml {
    /// YAML schema version
    pub version: String,

    /// Metadata about this test plan
    pub metadata: MetadataYaml,

    /// Resource constraints
    #[serde(default)]
    pub resource_limits: Option<ResourceLimitsYaml>,

    /// Workloads to execute
    #[serde(default)]
    pub workloads: Vec<WorkloadYaml>,

    /// Faults to inject
    #[serde(default)]
    pub faults: Vec<FaultDefinitionYaml>,

    /// Maximum test duration in seconds
    #[serde(default = "default_max_duration")]
    pub max_duration_secs: u64,
}

fn default_max_duration() -> u64 {
    3600
}

impl TestPlanYaml {
    /// Convert to srwsts_core::TestPlan
    pub fn to_core_plan(&self) -> srwsts_core::SrwstsResult<srwsts_core::TestPlan> {
        use srwsts_core::SrwstsError;

        // Validate version
        if self.version != "1.0" {
            return Err(SrwstsError::InvalidTestPlan {
                reason: format!("unsupported schema version: {}", self.version),
            });
        }

        // Build core plan
        let mut plan = srwsts_core::TestPlan::new(
            self.metadata.id.clone(),
            self.metadata.name.clone(),
            self.metadata.description.clone(),
        );

        plan.version = self.version.clone();

        // Set resource limits
        if let Some(limits) = &self.resource_limits {
            plan.resource_limits = limits.to_core_limits();
        }

        // Convert workloads
        for workload_yaml in &self.workloads {
            plan = plan.with_workload(workload_yaml.to_core_workload());
        }

        // Convert faults
        for fault_yaml in &self.faults {
            plan = plan.with_fault(fault_yaml.to_core_fault());
        }

        // Set max duration
        plan.max_duration_secs = self.max_duration_secs;

        // Add metadata tags
        if let Some(tags) = &self.metadata.tags {
            for tag in tags {
                plan = plan.with_metadata(format!("tag:{}", tag), "true");
            }
        }

        // Validate the plan
        plan.validate()?;

        Ok(plan)
    }
}

/// Metadata about a test plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataYaml {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: String,

    /// Optional version
    #[serde(default)]
    pub plan_version: Option<String>,

    /// Optional tags
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Resource limits YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsYaml {
    /// Max CPU usage percentage
    #[serde(default = "default_cpu_percent")]
    pub max_cpu_percent: f64,

    /// Max memory in bytes
    #[serde(default = "default_memory_bytes")]
    pub max_memory_bytes: u64,

    /// Max disk space in bytes
    #[serde(default = "default_disk_bytes")]
    pub max_disk_bytes: u64,

    /// Max file descriptors
    #[serde(default = "default_max_fds")]
    pub max_file_descriptors: u32,

    /// Max threads
    #[serde(default = "default_max_threads")]
    pub max_threads: u32,

    /// Max network bandwidth in Mbps
    #[serde(default = "default_max_network")]
    pub max_network_mbps: u64,
}

fn default_cpu_percent() -> f64 {
    100.0
}
fn default_memory_bytes() -> u64 {
    4 * 1024 * 1024 * 1024
}
fn default_disk_bytes() -> u64 {
    100 * 1024 * 1024 * 1024
}
fn default_max_fds() -> u32 {
    65536
}
fn default_max_threads() -> u32 {
    1024
}
fn default_max_network() -> u64 {
    1000
}

impl ResourceLimitsYaml {
    fn to_core_limits(&self) -> srwsts_core::ResourceLimits {
        srwsts_core::ResourceLimits {
            max_cpu_percent: self.max_cpu_percent,
            max_memory_bytes: self.max_memory_bytes,
            max_disk_bytes: self.max_disk_bytes,
            max_file_descriptors: self.max_file_descriptors,
            max_threads: self.max_threads,
            max_network_mbps: self.max_network_mbps,
        }
    }
}

impl Default for ResourceLimitsYaml {
    fn default() -> Self {
        Self {
            max_cpu_percent: default_cpu_percent(),
            max_memory_bytes: default_memory_bytes(),
            max_disk_bytes: default_disk_bytes(),
            max_file_descriptors: default_max_fds(),
            max_threads: default_max_threads(),
            max_network_mbps: default_max_network(),
        }
    }
}

/// Workload YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadYaml {
    /// Unique identifier
    pub id: String,

    /// Workload type
    #[serde(rename = "type")]
    pub workload_type: String,

    /// Number of concurrent tasks
    pub concurrency: u32,

    /// Duration in seconds
    pub duration_secs: u64,

    /// Operations per second
    #[serde(default)]
    pub ops_per_sec: u64,

    /// Custom parameters
    #[serde(default)]
    pub params: Option<HashMap<String, String>>,
}

impl WorkloadYaml {
    fn to_core_workload(&self) -> srwsts_core::Workload {
        let mut workload =
            srwsts_core::Workload::new(&self.id, &self.workload_type, self.concurrency, self.duration_secs);
        workload.ops_per_sec = self.ops_per_sec;
        if let Some(params) = &self.params {
            workload.parameters = params.clone();
        }
        workload
    }
}

/// Fault definition YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultDefinitionYaml {
    /// Unique identifier
    pub id: String,

    /// Fault type
    #[serde(rename = "type")]
    pub fault_type: String,

    /// When to inject (seconds)
    pub inject_at_secs: u64,

    /// Duration (seconds)
    pub duration_secs: u64,

    /// Whether enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Custom parameters
    #[serde(default)]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

fn default_true() -> bool {
    true
}

impl FaultDefinitionYaml {
    fn to_core_fault(&self) -> srwsts_core::FaultDefinition {
        let fault_type = match self.fault_type.as_str() {
            "cpu_stress" => srwsts_core::FaultType::CpuStress,
            "memory_exhaustion" => srwsts_core::FaultType::MemoryExhaustion,
            "disk_io_stress" => srwsts_core::FaultType::DiskIoStress,
            "network_packet_loss" => srwsts_core::FaultType::NetworkPacketLoss,
            "network_latency" => srwsts_core::FaultType::NetworkLatency,
            "kernel_panic" => srwsts_core::FaultType::KernelPanic,
            "task_scheduling_stress" => srwsts_core::FaultType::TaskSchedulingStress,
            "file_descriptor_exhaustion" => srwsts_core::FaultType::FileDescriptorExhaustion,
            "process_termination" => srwsts_core::FaultType::ProcessTermination,
            "signal_injection" => srwsts_core::FaultType::SignalInjection,
            other => srwsts_core::FaultType::Custom(other.to_string()),
        };

        let mut fault = srwsts_core::FaultDefinition::new(&self.id, fault_type, self.inject_at_secs, self.duration_secs);
        fault.enabled = self.enabled;

        if let Some(params) = &self.params {
            for (key, value) in params {
                fault = fault.with_parameter(key, value.clone());
            }
        }

        fault
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimitsYaml::default();
        assert_eq!(limits.max_cpu_percent, 100.0);
        assert_eq!(limits.max_threads, 1024);
    }

    #[test]
    fn test_workload_yaml_to_core() {
        let yaml = WorkloadYaml {
            id: "w1".to_string(),
            workload_type: "cpu_stress".to_string(),
            concurrency: 4,
            duration_secs: 300,
            ops_per_sec: 1000,
            params: None,
        };
        let core = yaml.to_core_workload();
        assert_eq!(core.id, "w1");
        assert_eq!(core.concurrency, 4);
    }

    #[test]
    fn test_fault_definition_yaml_cpu_stress() {
        let yaml = FaultDefinitionYaml {
            id: "f1".to_string(),
            fault_type: "cpu_stress".to_string(),
            inject_at_secs: 60,
            duration_secs: 120,
            enabled: true,
            params: None,
        };
        let core = yaml.to_core_fault();
        assert_eq!(core.id, "f1");
        assert!(matches!(core.fault_type, srwsts_core::FaultType::CpuStress));
    }

    #[test]
    fn test_fault_definition_yaml_custom() {
        let yaml = FaultDefinitionYaml {
            id: "f2".to_string(),
            fault_type: "custom_fault".to_string(),
            inject_at_secs: 10,
            duration_secs: 30,
            enabled: true,
            params: None,
        };
        let core = yaml.to_core_fault();
        assert!(matches!(core.fault_type, srwsts_core::FaultType::Custom(_)));
    }
}
