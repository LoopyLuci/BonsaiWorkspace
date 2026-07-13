use crate::{RecoveryPlan, RecoveryPoint, RecoveryExecution, RecoveryTest, HealthCheckResult, RecoveryError, RecoveryResult, ExecutionStatus};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct RecoveryOrchestrator {
    plans: Arc<DashMap<Uuid, RecoveryPlan>>,
    points: Arc<DashMap<Uuid, RecoveryPoint>>,
    executions: Arc<DashMap<Uuid, RecoveryExecution>>,
    tests: Arc<DashMap<Uuid, RecoveryTest>>,
}

impl RecoveryOrchestrator {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(DashMap::new()),
            points: Arc::new(DashMap::new()),
            executions: Arc::new(DashMap::new()),
            tests: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_recovery_plan(&self, plan: &RecoveryPlan) -> RecoveryResult<()> {
        self.plans.insert(plan.plan_id, plan.clone());
        Ok(())
    }

    pub async fn register_recovery_point(&self, point: &RecoveryPoint) -> RecoveryResult<()> {
        self.points.insert(point.point_id, point.clone());
        Ok(())
    }

    pub async fn execute_recovery(&self, plan_id: Uuid, recovery_point_id: Uuid) -> RecoveryResult<Uuid> {
        if !self.plans.contains_key(&plan_id) {
            return Err(RecoveryError::PlanNotFound);
        }

        if !self.points.contains_key(&recovery_point_id) {
            return Err(RecoveryError::InvalidRecoveryPoint);
        }

        let plan = self.plans.get(&plan_id).unwrap();
        let execution = RecoveryExecution {
            execution_id: Uuid::new_v4(),
            plan_id,
            status: ExecutionStatus::Executing,
            start_time: Utc::now(),
            end_time: None,
            rto_seconds: (plan.steps.len() * 5) as u32,
        };

        let execution_id = execution.execution_id;
        self.executions.insert(execution_id, execution);
        Ok(execution_id)
    }

    pub async fn complete_recovery(&self, execution_id: Uuid) -> RecoveryResult<()> {
        if let Some(mut execution) = self.executions.get_mut(&execution_id) {
            execution.status = ExecutionStatus::Completed;
            execution.end_time = Some(Utc::now());
            Ok(())
        } else {
            Err(RecoveryError::ExecutionFailed)
        }
    }

    pub async fn create_recovery_test(&self, test: &RecoveryTest) -> RecoveryResult<()> {
        self.tests.insert(test.test_id, test.clone());
        Ok(())
    }

    pub async fn run_recovery_test(&self, test_id: Uuid) -> RecoveryResult<bool> {
        if let Some(mut test) = self.tests.get_mut(&test_id) {
            test.last_tested = Utc::now();
            test.success = true;
            Ok(true)
        } else {
            Err(RecoveryError::ExecutionFailed)
        }
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }

    pub fn execution_count(&self) -> usize {
        self.executions.len()
    }
}

impl Default for RecoveryOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_recovery_plan() {
        let orchestrator = RecoveryOrchestrator::new();
        let plan = RecoveryPlan {
            plan_id: Uuid::new_v4(),
            name: "db_recovery".to_string(),
            resource_id: "db1".to_string(),
            steps: vec![],
            created_at: Utc::now(),
        };

        orchestrator.create_recovery_plan(&plan).await.unwrap();
        assert_eq!(orchestrator.plan_count(), 1);
    }

    #[tokio::test]
    async fn test_register_recovery_point() {
        let orchestrator = RecoveryOrchestrator::new();
        let point = RecoveryPoint {
            point_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            resource_id: "db1".to_string(),
            backup_id: Uuid::new_v4(),
            rpo_seconds: 3600,
        };

        orchestrator.register_recovery_point(&point).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_recovery() {
        let orchestrator = RecoveryOrchestrator::new();
        let plan_id = Uuid::new_v4();
        let point_id = Uuid::new_v4();

        let plan = RecoveryPlan {
            plan_id,
            name: "recovery".to_string(),
            resource_id: "db1".to_string(),
            steps: vec![],
            created_at: Utc::now(),
        };

        let point = RecoveryPoint {
            point_id,
            timestamp: Utc::now(),
            resource_id: "db1".to_string(),
            backup_id: Uuid::new_v4(),
            rpo_seconds: 3600,
        };

        orchestrator.create_recovery_plan(&plan).await.unwrap();
        orchestrator.register_recovery_point(&point).await.unwrap();

        let execution_id = orchestrator.execute_recovery(plan_id, point_id).await.unwrap();
        assert!(!execution_id.is_nil());
    }

    #[tokio::test]
    async fn test_run_recovery_test() {
        let orchestrator = RecoveryOrchestrator::new();
        let test = RecoveryTest {
            test_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            test_name: "quarterly_test".to_string(),
            last_tested: Utc::now(),
            success: false,
        };

        orchestrator.create_recovery_test(&test).await.unwrap();
        let success = orchestrator.run_recovery_test(test.test_id).await.unwrap();
        assert!(success);
    }
}
