//! Test Plan Generation and Management
//!
//! Generates and stores test plans in YAML format for reproducible testing.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::{SrwstsResult, registry::SuiteId};

/// Test plan definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlan {
    pub name: String,
    pub version: String,
    pub suite_id: String,
    pub tests: Vec<TestDefinition>,
    pub execution_order: ExecutionOrder,
    pub parallel_execution: bool,
    pub max_concurrent: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Test definition within a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub timeout_seconds: u64,
    pub retry_count: u32,
    pub priority: u32,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Test execution order strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionOrder {
    /// Execute by priority, then FIFO
    ByPriority,
    /// Execute as registered
    Sequential,
    /// Execute in parallel (limited by max_concurrent)
    Parallel,
}

impl std::fmt::Display for ExecutionOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByPriority => write!(f, "by-priority"),
            Self::Sequential => write!(f, "sequential"),
            Self::Parallel => write!(f, "parallel"),
        }
    }
}

/// Test plan generator
pub struct TestPlanGenerator;

impl TestPlanGenerator {
    /// Generate a test plan from a suite
    pub fn generate(
        suite_id: SuiteId,
        test_ids: Vec<String>,
    ) -> SrwstsResult<TestPlan> {
        let suite_name = format!("{}", suite_id);

        let tests = test_ids
            .into_iter()
            .enumerate()
            .map(|(idx, id)| TestDefinition {
                id: id.clone(),
                name: format!("test_{}", idx),
                description: format!("Test: {}", id),
                timeout_seconds: 60,
                retry_count: 1,
                priority: 50,
                tags: vec!["integration".to_string()],
                dependencies: vec![],
            })
            .collect();

        Ok(TestPlan {
            name: format!("{} Plan", suite_name),
            version: "0.1.0".to_string(),
            suite_id: suite_name,
            tests,
            execution_order: ExecutionOrder::Parallel,
            parallel_execution: true,
            max_concurrent: 8,
            created_at: chrono::Utc::now(),
        })
    }

    /// Generate from kernel tests
    pub fn kernel_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::Kernel,
            vec![
                "scheduler_fairness".to_string(),
                "scheduler_priority".to_string(),
                "scheduler_preemption".to_string(),
                "memory_allocation".to_string(),
                "memory_fragmentation".to_string(),
                "memory_oom_handling".to_string(),
                "ipc_message_passing".to_string(),
                "ipc_synchronization".to_string(),
                "ipc_deadlock_detection".to_string(),
                "driver_device_abstraction".to_string(),
                "driver_interrupt_handling".to_string(),
                "driver_dma_transfers".to_string(),
            ],
        )
    }

    /// Generate from service tests
    pub fn service_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::Service,
            vec![
                "p2p_node_discovery".to_string(),
                "p2p_routing".to_string(),
                "p2p_consensus".to_string(),
                "storage_persistence".to_string(),
                "storage_consistency".to_string(),
                "storage_recovery".to_string(),
                "network_bandwidth".to_string(),
                "network_latency".to_string(),
                "network_resilience".to_string(),
                "compositor_rendering".to_string(),
                "compositor_buffering".to_string(),
                "compositor_vsync".to_string(),
            ],
        )
    }

    /// Generate from language tests
    pub fn language_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::Language,
            vec![
                "titan_type_safety".to_string(),
                "titan_performance".to_string(),
                "titan_metaprogramming".to_string(),
                "sylva_dynamic_types".to_string(),
                "sylva_macros".to_string(),
                "sylva_interop".to_string(),
                "aether_actors".to_string(),
                "aether_distribution".to_string(),
                "aether_fault_tolerance".to_string(),
                "axiom_proofs".to_string(),
                "axiom_invariants".to_string(),
                "axiom_model_checking".to_string(),
            ],
        )
    }

    /// Generate from application tests
    pub fn application_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::Application,
            vec![
                "workspace_multi_user".to_string(),
                "workspace_sync".to_string(),
                "workspace_conflict_resolution".to_string(),
                "buddy_offline_sync".to_string(),
                "buddy_ai_queries".to_string(),
                "buddy_model_accuracy".to_string(),
                "omnibot_task_execution".to_string(),
                "omnibot_goal_planning".to_string(),
                "omnibot_learning".to_string(),
            ],
        )
    }

    /// Generate from hardware tests
    pub fn hardware_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::HardwareEquivalence,
            vec![
                "instruction_throughput".to_string(),
                "cache_behavior".to_string(),
                "memory_bandwidth".to_string(),
                "simd_vectorization".to_string(),
                "atomic_operations".to_string(),
            ],
        )
    }

    /// Generate from fullstack tests
    pub fn fullstack_plan() -> SrwstsResult<TestPlan> {
        Self::generate(
            SuiteId::FullStack,
            vec![
                "nominal_throughput".to_string(),
                "nominal_latency".to_string(),
                "nominal_consistency".to_string(),
                "peak_throughput".to_string(),
                "peak_stability".to_string(),
                "cascading_recovery".to_string(),
                "fault_isolation".to_string(),
                "partition_tolerance".to_string(),
                "partition_healing".to_string(),
                "byzantine_consensus".to_string(),
                "malicious_node_detection".to_string(),
                "corruption_detection".to_string(),
                "corruption_recovery".to_string(),
            ],
        )
    }

    /// Save test plan to YAML file
    pub fn save_to_file(plan: &TestPlan, path: &PathBuf) -> SrwstsResult<()> {
        let yaml = serde_yaml::to_string(plan)
            .map_err(|e| crate::SrwstsError::InvalidTestPlan {
                reason: format!("YAML serialization failed: {}", e),
            })?;

        std::fs::write(path, yaml)
            .map_err(|e| crate::SrwstsError::FileWriteError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Load test plan from YAML file
    pub fn load_from_file(path: &PathBuf) -> SrwstsResult<TestPlan> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::SrwstsError::FileReadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        let plan = serde_yaml::from_str(&content)
            .map_err(|e| crate::SrwstsError::InvalidTestPlan {
                reason: format!("YAML deserialization failed: {}", e),
            })?;

        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_plan_generation() {
        let plan = TestPlanGenerator::kernel_plan();
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.suite_id, "kernel");
        assert_eq!(plan.tests.len(), 12);
    }

    #[test]
    fn test_service_plan_generation() {
        let plan = TestPlanGenerator::service_plan();
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.suite_id, "service");
        assert_eq!(plan.tests.len(), 12);
    }

    #[test]
    fn test_language_plan_generation() {
        let plan = TestPlanGenerator::language_plan();
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.suite_id, "language");
        assert_eq!(plan.tests.len(), 12);
    }

    #[test]
    fn test_fullstack_plan_generation() {
        let plan = TestPlanGenerator::fullstack_plan();
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.tests.len(), 13);
    }

    #[test]
    fn test_test_definition_creation() {
        let def = TestDefinition {
            id: "test_1".to_string(),
            name: "Test One".to_string(),
            description: "First test".to_string(),
            timeout_seconds: 30,
            retry_count: 1,
            priority: 50,
            tags: vec!["unit".to_string()],
            dependencies: vec![],
        };

        assert_eq!(def.id, "test_1");
        assert_eq!(def.priority, 50);
    }
}
