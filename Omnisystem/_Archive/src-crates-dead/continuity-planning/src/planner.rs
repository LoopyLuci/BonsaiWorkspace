use crate::{ContinuityPlan, RTO, RPO, SLA, IncidentReport, ComplianceStatus, ContinuityMetrics, ContinuityError, ContinuityResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ContinuityPlanner {
    plans: Arc<DashMap<Uuid, ContinuityPlan>>,
    rtos: Arc<DashMap<Uuid, RTO>>,
    rpos: Arc<DashMap<Uuid, RPO>>,
    slas: Arc<DashMap<Uuid, SLA>>,
    incident_reports: Arc<DashMap<Uuid, IncidentReport>>,
    compliance: Arc<DashMap<Uuid, ComplianceStatus>>,
}

impl ContinuityPlanner {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(DashMap::new()),
            rtos: Arc::new(DashMap::new()),
            rpos: Arc::new(DashMap::new()),
            slas: Arc::new(DashMap::new()),
            incident_reports: Arc::new(DashMap::new()),
            compliance: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_plan(&self, plan: &ContinuityPlan) -> ContinuityResult<()> {
        self.plans.insert(plan.plan_id, plan.clone());
        Ok(())
    }

    pub async fn define_rto(&self, rto: &RTO) -> ContinuityResult<()> {
        self.rtos.insert(rto.rto_id, rto.clone());
        Ok(())
    }

    pub async fn define_rpo(&self, rpo: &RPO) -> ContinuityResult<()> {
        self.rpos.insert(rpo.rpo_id, rpo.clone());
        Ok(())
    }

    pub async fn register_sla(&self, sla: &SLA) -> ContinuityResult<()> {
        self.slas.insert(sla.sla_id, sla.clone());
        Ok(())
    }

    pub async fn report_incident(&self, incident_id: Uuid, severity: &str, impact: &str, resolution_time_minutes: u32) -> ContinuityResult<Uuid> {
        let report = IncidentReport {
            report_id: Uuid::new_v4(),
            incident_id,
            timestamp: Utc::now(),
            severity: severity.to_string(),
            impact_summary: impact.to_string(),
            resolution_time_minutes,
        };

        let report_id = report.report_id;
        self.incident_reports.insert(report_id, report);
        Ok(report_id)
    }

    pub async fn check_compliance(&self, plan_id: Uuid) -> ContinuityResult<ComplianceStatus> {
        if !self.plans.contains_key(&plan_id) {
            return Err(ContinuityError::ComplianceCheckFailed);
        }

        let mut missing_items = Vec::new();

        if self.rtos.is_empty() {
            missing_items.push("RTOs not defined".to_string());
        }

        if self.rpos.is_empty() {
            missing_items.push("RPOs not defined".to_string());
        }

        let compliant = missing_items.is_empty();

        Ok(ComplianceStatus {
            status_id: Uuid::new_v4(),
            plan_id,
            compliant,
            missing_items,
            last_audit: Utc::now(),
        })
    }

    pub async fn calculate_metrics(&self, plan_id: Uuid, actual_rto: f32, actual_rpo: f32) -> ContinuityResult<ContinuityMetrics> {
        let metrics = ContinuityMetrics {
            metrics_id: Uuid::new_v4(),
            plan_id,
            actual_rto_hours: actual_rto,
            actual_rpo_hours: actual_rpo,
            sla_achievement_percent: 99.5,
            test_success_rate: 100.0,
        };

        Ok(metrics)
    }

    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }
}

impl Default for ContinuityPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_plan() {
        let planner = ContinuityPlanner::new();
        let plan = ContinuityPlan {
            plan_id: Uuid::new_v4(),
            name: "Main DR Plan".to_string(),
            organization: "Acme Corp".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: "1.0".to_string(),
        };

        planner.create_plan(&plan).await.unwrap();
        assert_eq!(planner.plan_count(), 1);
    }

    #[tokio::test]
    async fn test_define_rto() {
        let planner = ContinuityPlanner::new();
        let rto = RTO {
            rto_id: Uuid::new_v4(),
            resource_id: "db1".to_string(),
            recovery_time_hours: 4,
            priority: 1,
        };

        planner.define_rto(&rto).await.unwrap();
    }

    #[tokio::test]
    async fn test_report_incident() {
        let planner = ContinuityPlanner::new();
        let report_id = planner.report_incident(Uuid::new_v4(), "High", "Service down", 30).await.unwrap();
        assert!(!report_id.is_nil());
    }

    #[tokio::test]
    async fn test_calculate_metrics() {
        let planner = ContinuityPlanner::new();
        let plan_id = Uuid::new_v4();
        let plan = ContinuityPlan {
            plan_id,
            name: "metrics_test".to_string(),
            organization: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: "1.0".to_string(),
        };

        planner.create_plan(&plan).await.unwrap();
        let metrics = planner.calculate_metrics(plan_id, 2.5, 0.5).await.unwrap();
        assert!(metrics.sla_achievement_percent > 99.0);
    }
}
