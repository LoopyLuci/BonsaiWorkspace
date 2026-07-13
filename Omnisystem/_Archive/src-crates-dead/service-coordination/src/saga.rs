use crate::{CoordinationError, CoordinationResult, SagaExecution, SagaPhase, SagaStep};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SagaExecutor {
    sagas: Arc<DashMap<String, SagaExecution>>,
}

impl SagaExecutor {
    pub fn new() -> Self {
        Self {
            sagas: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_saga(&self, saga_id: &str, steps: Vec<SagaStep>) -> CoordinationResult<SagaExecution> {
        if steps.is_empty() {
            return Err(CoordinationError::InvalidStep);
        }

        let execution = SagaExecution {
            saga_id: saga_id.to_string(),
            steps,
            phase: SagaPhase::Pending,
            completed_steps: Vec::new(),
            failed_step: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.sagas.insert(saga_id.to_string(), execution.clone());
        Ok(execution)
    }

    pub async fn start_saga(&self, saga_id: &str) -> CoordinationResult<()> {
        if let Some(mut saga) = self.sagas.get_mut(saga_id) {
            if saga.phase != SagaPhase::Pending {
                return Err(CoordinationError::InvalidPhase);
            }

            saga.phase = SagaPhase::Executing;
            saga.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub async fn mark_step_completed(&self, saga_id: &str, step_id: &str) -> CoordinationResult<()> {
        if let Some(mut saga) = self.sagas.get_mut(saga_id) {
            if saga.phase != SagaPhase::Executing {
                return Err(CoordinationError::InvalidPhase);
            }

            if saga.steps.iter().any(|s| s.step_id == step_id) {
                saga.completed_steps.push(step_id.to_string());
                saga.updated_at = Utc::now();
                Ok(())
            } else {
                Err(CoordinationError::InvalidStep)
            }
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub async fn mark_step_failed(&self, saga_id: &str, step_id: &str) -> CoordinationResult<()> {
        if let Some(mut saga) = self.sagas.get_mut(saga_id) {
            if saga.phase != SagaPhase::Executing {
                return Err(CoordinationError::InvalidPhase);
            }

            if saga.steps.iter().any(|s| s.step_id == step_id) {
                saga.failed_step = Some(step_id.to_string());
                saga.phase = SagaPhase::Compensating;
                saga.updated_at = Utc::now();
                Ok(())
            } else {
                Err(CoordinationError::InvalidStep)
            }
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub async fn complete_compensation(&self, saga_id: &str) -> CoordinationResult<()> {
        if let Some(mut saga) = self.sagas.get_mut(saga_id) {
            if saga.phase != SagaPhase::Compensating {
                return Err(CoordinationError::InvalidPhase);
            }

            saga.phase = SagaPhase::Failed;
            saga.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub async fn complete_saga(&self, saga_id: &str) -> CoordinationResult<()> {
        if let Some(mut saga) = self.sagas.get_mut(saga_id) {
            if saga.phase != SagaPhase::Executing {
                return Err(CoordinationError::InvalidPhase);
            }

            saga.phase = SagaPhase::Complete;
            saga.updated_at = Utc::now();
            Ok(())
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub async fn get_saga(&self, saga_id: &str) -> CoordinationResult<SagaExecution> {
        self.sagas
            .get(saga_id)
            .map(|entry| entry.clone())
            .ok_or(CoordinationError::SagaNotFound)
    }

    pub async fn get_pending_compensations(&self, saga_id: &str) -> CoordinationResult<Vec<SagaStep>> {
        if let Some(saga) = self.sagas.get(saga_id) {
            if saga.failed_step.is_none() {
                return Ok(Vec::new());
            }

            let failed_order = saga
                .steps
                .iter()
                .find(|s| saga.failed_step.as_ref().map_or(false, |f| f == &s.step_id))
                .map(|s| s.order)
                .unwrap_or(0);

            let pending: Vec<SagaStep> = saga
                .steps
                .iter()
                .filter(|s| s.order < failed_order && saga.completed_steps.contains(&s.step_id))
                .cloned()
                .collect();

            Ok(pending)
        } else {
            Err(CoordinationError::SagaNotFound)
        }
    }

    pub fn saga_count(&self) -> usize {
        self.sagas.len()
    }
}

impl Default for SagaExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_saga() {
        let executor = SagaExecutor::new();
        let steps = vec![SagaStep {
            step_id: "step-1".to_string(),
            service_id: "service-1".to_string(),
            action: "debit".to_string(),
            compensation: "credit".to_string(),
            order: 1,
        }];

        let saga = executor.create_saga("saga-1", steps).await.unwrap();
        assert_eq!(saga.saga_id, "saga-1");
        assert_eq!(saga.phase, SagaPhase::Pending);
    }

    #[tokio::test]
    async fn test_start_saga() {
        let executor = SagaExecutor::new();
        let steps = vec![SagaStep {
            step_id: "step-1".to_string(),
            service_id: "service-1".to_string(),
            action: "debit".to_string(),
            compensation: "credit".to_string(),
            order: 1,
        }];

        executor.create_saga("saga-1", steps).await.unwrap();
        executor.start_saga("saga-1").await.unwrap();

        let saga = executor.get_saga("saga-1").await.unwrap();
        assert_eq!(saga.phase, SagaPhase::Executing);
    }

    #[tokio::test]
    async fn test_mark_step_completed() {
        let executor = SagaExecutor::new();
        let steps = vec![SagaStep {
            step_id: "step-1".to_string(),
            service_id: "service-1".to_string(),
            action: "debit".to_string(),
            compensation: "credit".to_string(),
            order: 1,
        }];

        executor.create_saga("saga-1", steps).await.unwrap();
        executor.start_saga("saga-1").await.unwrap();
        executor.mark_step_completed("saga-1", "step-1").await.unwrap();

        let saga = executor.get_saga("saga-1").await.unwrap();
        assert_eq!(saga.completed_steps.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_step_failed() {
        let executor = SagaExecutor::new();
        let steps = vec![SagaStep {
            step_id: "step-1".to_string(),
            service_id: "service-1".to_string(),
            action: "debit".to_string(),
            compensation: "credit".to_string(),
            order: 1,
        }];

        executor.create_saga("saga-1", steps).await.unwrap();
        executor.start_saga("saga-1").await.unwrap();
        executor.mark_step_failed("saga-1", "step-1").await.unwrap();

        let saga = executor.get_saga("saga-1").await.unwrap();
        assert_eq!(saga.phase, SagaPhase::Compensating);
        assert_eq!(saga.failed_step, Some("step-1".to_string()));
    }

    #[tokio::test]
    async fn test_complete_saga() {
        let executor = SagaExecutor::new();
        let steps = vec![SagaStep {
            step_id: "step-1".to_string(),
            service_id: "service-1".to_string(),
            action: "debit".to_string(),
            compensation: "credit".to_string(),
            order: 1,
        }];

        executor.create_saga("saga-1", steps).await.unwrap();
        executor.start_saga("saga-1").await.unwrap();
        executor.complete_saga("saga-1").await.unwrap();

        let saga = executor.get_saga("saga-1").await.unwrap();
        assert_eq!(saga.phase, SagaPhase::Complete);
    }

    #[tokio::test]
    async fn test_get_pending_compensations() {
        let executor = SagaExecutor::new();
        let steps = vec![
            SagaStep {
                step_id: "step-1".to_string(),
                service_id: "service-1".to_string(),
                action: "debit".to_string(),
                compensation: "credit".to_string(),
                order: 1,
            },
            SagaStep {
                step_id: "step-2".to_string(),
                service_id: "service-2".to_string(),
                action: "transfer".to_string(),
                compensation: "reverse".to_string(),
                order: 2,
            },
        ];

        executor.create_saga("saga-1", steps).await.unwrap();
        executor.start_saga("saga-1").await.unwrap();
        executor.mark_step_completed("saga-1", "step-1").await.unwrap();
        executor.mark_step_failed("saga-1", "step-2").await.unwrap();

        let compensations = executor.get_pending_compensations("saga-1").await.unwrap();
        assert_eq!(compensations.len(), 1);
    }

    #[tokio::test]
    async fn test_saga_not_found() {
        let executor = SagaExecutor::new();
        let result = executor.get_saga("nonexistent").await;

        assert!(result.is_err());
    }
}
