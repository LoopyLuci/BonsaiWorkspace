//! Validation logic for SRWSTS test plans and schemas
//!
//! Provides comprehensive validation of test plans, including resource limits,
//! fault parameters, and workload consistency.

use serde::{Deserialize, Serialize};
use srwsts_core::{SrwstsError, SrwstsResult, TestPlan};
use thiserror::Error;

/// Schema validation error
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Resource limit violation: {message}")]
    ResourceLimitViolation { message: String },

    #[error("Invalid workload configuration: {message}")]
    InvalidWorkload { message: String },

    #[error("Invalid fault configuration: {message}")]
    InvalidFault { message: String },

    #[error("Plan constraint violation: {message}")]
    ConstraintViolation { message: String },

    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },
}

/// Schema validator for test plans
pub struct SchemaValidator {
    max_total_duration_secs: u64,
    max_workload_concurrency: u32,
    max_memory_bytes: u64,
}

impl SchemaValidator {
    /// Create a new schema validator with default limits
    pub fn new() -> Self {
        Self {
            max_total_duration_secs: 86400, // 24 hours
            max_workload_concurrency: 100000,
            max_memory_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
        }
    }

    /// Set the maximum total test duration
    pub fn with_max_duration(mut self, secs: u64) -> Self {
        self.max_total_duration_secs = secs;
        self
    }

    /// Set the maximum workload concurrency
    pub fn with_max_concurrency(mut self, concurrency: u32) -> Self {
        self.max_workload_concurrency = concurrency;
        self
    }

    /// Set the maximum memory limit
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = bytes;
        self
    }

    /// Validate a test plan
    pub fn validate(&self, plan: &TestPlan) -> SrwstsResult<()> {
        // First, run the plan's own validation
        plan.validate()?;

        // Validate resource limits
        self.validate_resource_limits(plan)?;

        // Validate workloads
        self.validate_workloads(plan)?;

        // Validate faults
        self.validate_faults(plan)?;

        // Validate plan constraints
        self.validate_plan_constraints(plan)?;

        Ok(())
    }

    fn validate_resource_limits(&self, plan: &TestPlan) -> SrwstsResult<()> {
        let limits = &plan.resource_limits;

        if limits.max_cpu_percent <= 0.0 || limits.max_cpu_percent > 100.0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: format!(
                    "invalid CPU limit: {}% (must be 0-100)",
                    limits.max_cpu_percent
                ),
            });
        }

        if limits.max_memory_bytes > self.max_memory_bytes {
            return Err(SrwstsError::ResourceLimitExceeded {
                resource_type: "memory".to_string(),
                limit: self.max_memory_bytes,
                actual: limits.max_memory_bytes,
            });
        }

        if limits.max_threads == 0 || limits.max_threads > 1000000 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: format!("invalid thread limit: {} (must be 1-1000000)", limits.max_threads),
            });
        }

        if limits.max_file_descriptors == 0 {
            return Err(SrwstsError::InvalidConfiguration {
                reason: "max_file_descriptors must be > 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_workloads(&self, plan: &TestPlan) -> SrwstsResult<()> {
        if plan.workloads.is_empty() {
            return Err(SrwstsError::InvalidTestPlan {
                reason: "test plan must contain at least one workload".to_string(),
            });
        }

        for workload in &plan.workloads {
            if workload.concurrency > self.max_workload_concurrency {
                return Err(SrwstsError::ResourceLimitExceeded {
                    resource_type: "concurrency".to_string(),
                    limit: self.max_workload_concurrency as u64,
                    actual: workload.concurrency as u64,
                });
            }

            if workload.duration_secs == 0 {
                return Err(SrwstsError::InvalidConfiguration {
                    reason: format!("workload {} has zero duration", workload.id),
                });
            }

            if workload.duration_secs > self.max_total_duration_secs {
                return Err(SrwstsError::ResourceLimitExceeded {
                    resource_type: "workload_duration".to_string(),
                    limit: self.max_total_duration_secs,
                    actual: workload.duration_secs,
                });
            }
        }

        Ok(())
    }

    fn validate_faults(&self, plan: &TestPlan) -> SrwstsResult<()> {
        for fault in &plan.faults {
            if !fault.enabled {
                continue; // Skip disabled faults
            }

            // Check fault timing vs test duration
            let fault_end = fault.inject_at_secs + fault.duration_secs;
            if fault_end > plan.max_duration_secs {
                return Err(SrwstsError::InvalidFaultParameters {
                    reason: format!(
                        "fault {} ends at {}s, but test max duration is {}s",
                        fault.id, fault_end, plan.max_duration_secs
                    ),
                });
            }

            // Validate fault-specific parameters based on type
            self.validate_fault_parameters(fault)?;
        }

        Ok(())
    }

    fn validate_fault_parameters(&self, fault: &srwsts_core::FaultDefinition) -> SrwstsResult<()> {
        use srwsts_core::FaultType;

        match &fault.fault_type {
            FaultType::CpuStress => {
                if let Some(cpu_count) = fault.parameters.get("cpu_count") {
                    if let Some(count) = cpu_count.as_u64() {
                        if count == 0 || count > 256 {
                            return Err(SrwstsError::InvalidFaultParameters {
                                reason: format!("invalid cpu_count: {} (must be 1-256)", count),
                            });
                        }
                    }
                }
            }
            FaultType::MemoryExhaustion => {
                if let Some(percent) = fault.parameters.get("memory_percent") {
                    if let Some(p) = percent.as_f64() {
                        if p < 0.0 || p > 100.0 {
                            return Err(SrwstsError::InvalidFaultParameters {
                                reason: format!("invalid memory_percent: {} (must be 0-100)", p),
                            });
                        }
                    }
                }
            }
            FaultType::NetworkPacketLoss => {
                if let Some(loss) = fault.parameters.get("packet_loss_percent") {
                    if let Some(p) = loss.as_f64() {
                        if p < 0.0 || p > 100.0 {
                            return Err(SrwstsError::InvalidFaultParameters {
                                reason: format!(
                                    "invalid packet_loss_percent: {} (must be 0-100)",
                                    p
                                ),
                            });
                        }
                    }
                }
            }
            FaultType::NetworkLatency => {
                if let Some(latency) = fault.parameters.get("latency_ms") {
                    if let Some(ms) = latency.as_u64() {
                        if ms > 10000 {
                            return Err(SrwstsError::InvalidFaultParameters {
                                reason: format!("latency_ms: {}ms seems unreasonably high", ms),
                            });
                        }
                    }
                }
            }
            _ => {
                // Other fault types don't have specific validation
            }
        }

        Ok(())
    }

    fn validate_plan_constraints(&self, plan: &TestPlan) -> SrwstsResult<()> {
        if plan.max_duration_secs > self.max_total_duration_secs {
            return Err(SrwstsError::ResourceLimitExceeded {
                resource_type: "test_duration".to_string(),
                limit: self.max_total_duration_secs,
                actual: plan.max_duration_secs,
            });
        }

        // Check that max_duration is >= longest workload
        let longest_workload = plan.max_workload_duration();
        if plan.max_duration_secs < longest_workload {
            return Err(SrwstsError::InvalidTestPlan {
                reason: format!(
                    "max_duration_secs ({}) must be >= longest workload ({})",
                    plan.max_duration_secs, longest_workload
                ),
            });
        }

        Ok(())
    }

    /// Suggest fixes for common validation errors
    pub fn suggest_fix(&self, error: &SrwstsError) -> Option<String> {
        match error {
            SrwstsError::ResourceLimitExceeded { resource_type, limit, actual } => {
                Some(format!(
                    "Resource limit exceeded for {}: {} > limit {}. Reduce the {} or increase limits.",
                    resource_type, actual, limit, resource_type
                ))
            }
            SrwstsError::InvalidConfiguration { reason } => {
                Some(format!("Configuration error: {}. Check your test plan YAML.", reason))
            }
            SrwstsError::InvalidTestPlan { reason } => {
                Some(format!("Test plan error: {}. Verify your plan definition.", reason))
            }
            _ => None,
        }
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_plan() {
        let plan = TestPlan::new("test1", "Test", "Description")
            .with_workload(srwsts_core::Workload::new("w1", "cpu_stress", 4, 300));

        let validator = SchemaValidator::new();
        assert!(validator.validate(&plan).is_ok());
    }

    #[test]
    fn test_validate_cpu_limit() {
        let mut plan = TestPlan::new("test1", "Test", "Description")
            .with_workload(srwsts_core::Workload::new("w1", "cpu_stress", 4, 300));
        plan.resource_limits.max_cpu_percent = 150.0; // Invalid

        let validator = SchemaValidator::new();
        assert!(validator.validate(&plan).is_err());
    }

    #[test]
    fn test_validate_workload_concurrency() {
        let plan = TestPlan::new("test1", "Test", "Description")
            .with_workload(srwsts_core::Workload::new("w1", "cpu_stress", 1000000, 300));

        let validator = SchemaValidator::new().with_max_concurrency(100000);
        assert!(validator.validate(&plan).is_err());
    }

    #[test]
    fn test_validate_fault_timing() {
        let mut plan = TestPlan::new("test1", "Test", "Description")
            .with_workload(srwsts_core::Workload::new("w1", "cpu_stress", 4, 100))
            .with_fault(srwsts_core::FaultDefinition::new(
                "f1",
                srwsts_core::FaultType::CpuStress,
                50,
                100, // Ends at 150s
            ));
        plan.max_duration_secs = 120; // But we set max_duration to 120s, which is < 150s

        let validator = SchemaValidator::new();
        assert!(validator.validate(&plan).is_err());
    }

    #[test]
    fn test_validate_network_packet_loss_parameter() {
        let mut fault = srwsts_core::FaultDefinition::new(
            "f1",
            srwsts_core::FaultType::NetworkPacketLoss,
            10,
            60,
        );
        fault = fault.with_parameter("packet_loss_percent", serde_json::json!(150.0)); // Invalid

        let plan = TestPlan::new("test1", "Test", "Description")
            .with_workload(srwsts_core::Workload::new("w1", "cpu_stress", 4, 300))
            .with_fault(fault);

        let validator = SchemaValidator::new();
        assert!(validator.validate(&plan).is_err());
    }
}
