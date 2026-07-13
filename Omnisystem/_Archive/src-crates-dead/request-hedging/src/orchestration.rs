use crate::{HedgingError, HedgingResult, OrchestrationPlan, OrchestrationStatus};
use dashmap::DashMap;
use std::sync::Arc;

pub struct OrchestrationEngine {
    plans: Arc<DashMap<String, OrchestrationPlan>>,
    statuses: Arc<DashMap<String, OrchestrationStatus>>,
}

impl OrchestrationEngine {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(DashMap::new()),
            statuses: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_plan(
        &self,
        plan_id: &str,
        service_order: Vec<String>,
        dependencies: Vec<(String, String)>,
    ) -> HedgingResult<OrchestrationPlan> {
        if service_order.is_empty() {
            return Err(HedgingError::InvalidConfiguration(
                "Service order cannot be empty".to_string(),
            ));
        }

        let parallel_stages = Self::compute_parallel_stages(&service_order, &dependencies);
        let estimated_duration = (service_order.len() as u64) * 100;

        let plan = OrchestrationPlan {
            plan_id: plan_id.to_string(),
            service_order: service_order.clone(),
            parallel_stages,
            dependencies,
            estimated_duration_ms: estimated_duration,
        };

        self.plans.insert(plan_id.to_string(), plan.clone());

        let status = OrchestrationStatus {
            plan_id: plan_id.to_string(),
            completed_services: Vec::new(),
            failed_services: Vec::new(),
            in_progress_services: vec![service_order[0].clone()],
            overall_progress_percent: 0,
        };

        self.statuses.insert(plan_id.to_string(), status);

        Ok(plan)
    }

    pub async fn mark_service_completed(
        &self,
        plan_id: &str,
        service_id: &str,
    ) -> HedgingResult<()> {
        if let Some(mut status) = self.statuses.get_mut(plan_id) {
            status.in_progress_services.retain(|s| s != service_id);
            status.completed_services.push(service_id.to_string());

            let total = status.completed_services.len() + status.failed_services.len();
            if let Some(plan) = self.plans.get(plan_id) {
                let total_services = plan.service_order.len();
                status.overall_progress_percent = ((total as f64 / total_services as f64) * 100.0) as u32;
            }
        }

        Ok(())
    }

    pub async fn mark_service_failed(
        &self,
        plan_id: &str,
        service_id: &str,
    ) -> HedgingResult<()> {
        if let Some(mut status) = self.statuses.get_mut(plan_id) {
            status.in_progress_services.retain(|s| s != service_id);
            status.failed_services.push(service_id.to_string());

            let total = status.completed_services.len() + status.failed_services.len();
            if let Some(plan) = self.plans.get(plan_id) {
                let total_services = plan.service_order.len();
                status.overall_progress_percent = ((total as f64 / total_services as f64) * 100.0) as u32;
            }
        }

        Ok(())
    }

    pub async fn get_status(&self, plan_id: &str) -> HedgingResult<OrchestrationStatus> {
        self.statuses
            .get(plan_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| HedgingError::Internal("Status not found".to_string()))
    }

    fn compute_parallel_stages(
        service_order: &[String],
        dependencies: &[(String, String)],
    ) -> Vec<Vec<String>> {
        let mut stages = Vec::new();
        let mut current_stage = vec![service_order[0].clone()];

        for i in 1..service_order.len() {
            let service = &service_order[i];
            let has_dependency = dependencies.iter().any(|(_, target)| target == service);

            if has_dependency {
                stages.push(current_stage);
                current_stage = vec![service.clone()];
            } else {
                current_stage.push(service.clone());
            }
        }

        if !current_stage.is_empty() {
            stages.push(current_stage);
        }

        if stages.is_empty() {
            stages.push(service_order.to_vec());
        }

        stages
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }
}

impl Default for OrchestrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_plan() {
        let engine = OrchestrationEngine::new();
        let services = vec!["service-1".to_string(), "service-2".to_string()];

        let plan = engine
            .create_plan("plan-1", services, vec![])
            .await
            .unwrap();

        assert_eq!(plan.plan_id, "plan-1");
        assert_eq!(plan.service_order.len(), 2);
    }

    #[tokio::test]
    async fn test_mark_service_completed() {
        let engine = OrchestrationEngine::new();
        let services = vec!["service-1".to_string(), "service-2".to_string()];

        engine
            .create_plan("plan-1", services, vec![])
            .await
            .unwrap();

        engine
            .mark_service_completed("plan-1", "service-1")
            .await
            .unwrap();

        let status = engine.get_status("plan-1").await.unwrap();
        assert_eq!(status.completed_services.len(), 1);
        assert!(status.overall_progress_percent > 0);
    }

    #[tokio::test]
    async fn test_mark_service_failed() {
        let engine = OrchestrationEngine::new();
        let services = vec!["service-1".to_string(), "service-2".to_string()];

        engine
            .create_plan("plan-1", services, vec![])
            .await
            .unwrap();

        engine
            .mark_service_failed("plan-1", "service-1")
            .await
            .unwrap();

        let status = engine.get_status("plan-1").await.unwrap();
        assert_eq!(status.failed_services.len(), 1);
    }

    #[tokio::test]
    async fn test_compute_parallel_stages() {
        let services = vec![
            "service-1".to_string(),
            "service-2".to_string(),
            "service-3".to_string(),
        ];
        let dependencies = vec![("service-1".to_string(), "service-2".to_string())];

        let stages = OrchestrationEngine::compute_parallel_stages(&services, &dependencies);
        assert!(stages.len() > 0);
    }

    #[tokio::test]
    async fn test_empty_service_order() {
        let engine = OrchestrationEngine::new();
        let result = engine.create_plan("plan-1", vec![], vec![]).await;

        assert!(result.is_err());
    }
}
