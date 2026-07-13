//! Core trait definitions for SRWSTS extensibility
//!
//! Provides trait interfaces that can be implemented by different components
//! to extend SRWSTS functionality for test execution, fault injection, and result collection.

use crate::{SrwstsError, SrwstsResult, TestMetrics, TestResult};
use async_trait::async_trait;

/// Trait for components that execute tests
///
/// Implementers of this trait can run test plans and return results.
/// This allows for different test execution strategies (local, distributed, etc.)
#[async_trait]
pub trait TestExecutor: Send + Sync {
    /// Execute a test and return results
    ///
    /// # Arguments
    /// * `plan` - The test plan to execute
    /// * `timeout_secs` - Maximum execution time in seconds
    ///
    /// # Returns
    /// A TestResult containing execution outcomes
    async fn execute(&self, plan: &crate::TestPlan, timeout_secs: u64) -> SrwstsResult<TestResult>;

    /// Validate that a test plan can be executed
    ///
    /// # Arguments
    /// * `plan` - The test plan to validate
    ///
    /// # Returns
    /// Ok if the plan can be executed, Err otherwise
    async fn validate_plan(&self, plan: &crate::TestPlan) -> SrwstsResult<()> {
        plan.validate()
    }

    /// Setup any resources needed for test execution
    async fn setup(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Cleanup resources after test execution
    async fn cleanup(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Get the name of this executor
    fn name(&self) -> &str {
        "Unknown"
    }
}

/// Trait for components that inject faults during testing
///
/// Implementers can inject various types of faults to test system resilience.
#[async_trait]
pub trait FaultInjector: Send + Sync {
    /// Inject a fault into the system
    ///
    /// # Arguments
    /// * `fault` - The fault definition to inject
    ///
    /// # Returns
    /// Ok if fault was successfully injected, Err otherwise
    async fn inject_fault(&mut self, fault: &crate::FaultDefinition) -> SrwstsResult<()>;

    /// Remove/recover from a previously injected fault
    ///
    /// # Arguments
    /// * `fault_id` - ID of the fault to recover from
    ///
    /// # Returns
    /// Ok if recovery was successful, Err otherwise
    async fn recover_fault(&mut self, fault_id: &str) -> SrwstsResult<()>;

    /// Check if the system has recovered from a fault
    ///
    /// # Arguments
    /// * `fault_id` - ID of the fault to check
    ///
    /// # Returns
    /// Ok(true) if recovered, Ok(false) if not, Err on error
    async fn is_recovered(&self, fault_id: &str) -> SrwstsResult<bool>;

    /// Get the supported fault types
    fn supported_faults(&self) -> Vec<crate::FaultType>;

    /// Validate a fault definition
    fn validate_fault(&self, fault: &crate::FaultDefinition) -> SrwstsResult<()> {
        // Default implementation: check if fault type is supported
        if !self
            .supported_faults()
            .contains(&fault.fault_type)
        {
            return Err(SrwstsError::UnsupportedFaultType {
                fault_type: fault.fault_type.to_string(),
            });
        }
        fault.validate()
    }

    /// Initialize the fault injector
    async fn initialize(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Shutdown the fault injector
    async fn shutdown(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Get the name of this injector
    fn name(&self) -> &str {
        "Unknown"
    }
}

/// Trait for components that collect and store test results
#[async_trait]
pub trait ResultCollector: Send + Sync {
    /// Record a test result
    ///
    /// # Arguments
    /// * `result` - The test result to record
    ///
    /// # Returns
    /// Ok if result was stored successfully, Err otherwise
    async fn collect(&mut self, result: TestResult) -> SrwstsResult<()>;

    /// Retrieve a previously collected result
    ///
    /// # Arguments
    /// * `result_id` - UUID of the result to retrieve
    ///
    /// # Returns
    /// The retrieved result, or error if not found
    async fn get_result(&self, result_id: &uuid::Uuid) -> SrwstsResult<TestResult>;

    /// List all results for a test ID
    ///
    /// # Arguments
    /// * `test_id` - The test ID to search for
    ///
    /// # Returns
    /// Vec of result IDs matching the test
    async fn list_results_for_test(&self, test_id: &crate::TestId) -> SrwstsResult<Vec<uuid::Uuid>>;

    /// Get metrics summary for a specific result
    ///
    /// # Arguments
    /// * `result_id` - UUID of the result
    ///
    /// # Returns
    /// Metrics from that result
    async fn get_metrics(&self, result_id: &uuid::Uuid) -> SrwstsResult<TestMetrics> {
        let result = self.get_result(result_id).await?;
        Ok(result.metrics)
    }

    /// Delete a result
    ///
    /// # Arguments
    /// * `result_id` - UUID of the result to delete
    async fn delete_result(&mut self, result_id: &uuid::Uuid) -> SrwstsResult<()>;

    /// Clear all results
    async fn clear_all(&mut self) -> SrwstsResult<()>;

    /// Initialize the result collector
    async fn initialize(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Shutdown the result collector
    async fn shutdown(&mut self) -> SrwstsResult<()> {
        Ok(())
    }

    /// Get the name of this collector
    fn name(&self) -> &str {
        "Unknown"
    }

    /// Get the total number of results stored
    async fn count(&self) -> SrwstsResult<usize>;
}

/// Trait for custom test hooks/callbacks
pub trait TestHook: Send + Sync {
    /// Called before test execution starts
    async fn before_test(&self, test_id: &crate::TestId) -> SrwstsResult<()> {
        Ok(())
    }

    /// Called after test execution completes
    async fn after_test(&self, result: &TestResult) -> SrwstsResult<()> {
        Ok(())
    }

    /// Called before a fault is injected
    async fn before_fault_injection(&self, fault: &crate::FaultDefinition) -> SrwstsResult<()> {
        Ok(())
    }

    /// Called after a fault is injected
    async fn after_fault_injection(
        &self,
        fault: &crate::FaultDefinition,
        success: bool,
    ) -> SrwstsResult<()> {
        Ok(())
    }

    /// Called when a fault recovery is detected
    async fn on_fault_recovery(&self, fault_id: &str) -> SrwstsResult<()> {
        Ok(())
    }

    /// Called when an assertion fails
    async fn on_assertion_failure(
        &self,
        test_id: &crate::TestId,
        assertion: &crate::result::AssertionResult,
    ) -> SrwstsResult<()> {
        Ok(())
    }
}

/// Default no-op implementation of TestHook
pub struct DefaultTestHook;

#[async_trait]
impl TestHook for DefaultTestHook {}

/// Trait for monitoring system behavior during tests
#[async_trait]
pub trait SystemMonitor: Send + Sync {
    /// Sample current system metrics
    async fn sample_metrics(&self) -> SrwstsResult<TestMetrics>;

    /// Get metrics over a time window
    async fn get_metrics_window(&self, start_secs: u64, end_secs: u64) -> SrwstsResult<TestMetrics>;

    /// Start continuous monitoring
    async fn start_monitoring(&mut self) -> SrwstsResult<()>;

    /// Stop continuous monitoring
    async fn stop_monitoring(&mut self) -> SrwstsResult<()>;

    /// Get the name of this monitor
    fn name(&self) -> &str {
        "Unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_hook_default_implementations() {
        let hook = DefaultTestHook;
        assert!(hook.before_test(&crate::TestId::new("test1")).await.is_ok());
        assert!(hook.after_test(&TestResult::new(
            crate::TestId::new("test1"),
            crate::RunId::new(),
            crate::types::Timestamp::now()
        ))
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn test_fault_injector_default_validate() {
        struct TestInjector;

        #[async_trait]
        impl FaultInjector for TestInjector {
            async fn inject_fault(&mut self, _: &crate::FaultDefinition) -> SrwstsResult<()> {
                Ok(())
            }
            async fn recover_fault(&mut self, _: &str) -> SrwstsResult<()> {
                Ok(())
            }
            async fn is_recovered(&self, _: &str) -> SrwstsResult<bool> {
                Ok(true)
            }
            fn supported_faults(&self) -> Vec<crate::FaultType> {
                vec![crate::FaultType::CpuStress]
            }
        }

        let injector = TestInjector;
        let fault = crate::FaultDefinition::new("f1", crate::FaultType::CpuStress, 0, 10);
        assert!(injector.validate_fault(&fault).is_ok());

        let unsupported = crate::FaultDefinition::new("f2", crate::FaultType::MemoryExhaustion, 0, 10);
        assert!(injector.validate_fault(&unsupported).is_err());
    }
}
