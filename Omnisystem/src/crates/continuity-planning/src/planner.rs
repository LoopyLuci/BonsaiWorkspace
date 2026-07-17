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
            return Err(ContinuityError::PlanNotFound(plan_id));
        }

        let mut missing_items = Vec::new();

        if self.rtos.is_empty() {
            missing_items.push("RTOs not defined".to_string());
        }

        if self.rpos.is_empty() {
            missing_items.push("RPOs not defined".to_string());
        }

        if self.slas.is_empty() {
            missing_items.push("SLAs not defined".to_string());
        }

        let compliant = missing_items.is_empty();

        let status = ComplianceStatus {
            status_id: Uuid::new_v4(),
            plan_id,
            compliant,
            missing_items,
            last_audit: Utc::now(),
        };

        // Persist the audit trail so compliance history can be queried later.
        self.compliance.insert(status.status_id, status.clone());

        Ok(status)
    }

    /// Look up the most recent compliance check recorded for `plan_id`, if any.
    pub fn last_compliance_check(&self, plan_id: Uuid) -> Option<ComplianceStatus> {
        self.compliance
            .iter()
            .filter(|entry| entry.value().plan_id == plan_id)
            .max_by_key(|entry| entry.value().last_audit)
            .map(|entry| entry.value().clone())
    }

    /// Calculate real continuity metrics for a plan by comparing the actual
    /// recovery time/point achieved during a drill or incident against the
    /// targets registered via `define_rto`/`define_rpo` for that plan's
    /// resources, and comparing incident resolution times against the SLA's
    /// promised response time.
    pub async fn calculate_metrics(&self, plan_id: Uuid, actual_rto: f32, actual_rpo: f32) -> ContinuityResult<ContinuityMetrics> {
        if !self.plans.contains_key(&plan_id) {
            return Err(ContinuityError::PlanNotFound(plan_id));
        }

        // Target RTO/RPO: the tightest (minimum) target across all resources
        // registered for this planner, since the plan must satisfy all of them.
        let target_rto_hours = self
            .rtos
            .iter()
            .map(|e| e.value().recovery_time_hours as f32)
            .fold(f32::INFINITY, f32::min);
        let target_rpo_hours = self
            .rpos
            .iter()
            .map(|e| e.value().recovery_point_hours as f32)
            .fold(f32::INFINITY, f32::min);

        // SLA achievement: how close the actual recovery was to the defined
        // targets. 100% means we recovered at or faster than the target;
        // it degrades the further actual exceeds target. When no targets are
        // defined yet, we cannot claim any achievement.
        let sla_achievement_percent = match (target_rto_hours.is_finite(), target_rpo_hours.is_finite()) {
            (false, false) => 0.0,
            _ => {
                let rto_score = if target_rto_hours.is_finite() && target_rto_hours > 0.0 {
                    (target_rto_hours / actual_rto.max(target_rto_hours)).min(1.0)
                } else {
                    1.0
                };
                let rpo_score = if target_rpo_hours.is_finite() && target_rpo_hours > 0.0 {
                    (target_rpo_hours / actual_rpo.max(target_rpo_hours)).min(1.0)
                } else {
                    1.0
                };
                ((rto_score + rpo_score) / 2.0) * 100.0
            }
        };

        // Test success rate: fraction of recorded incidents that were resolved
        // within the fastest registered SLA's promised response time.
        let test_success_rate = if self.incident_reports.is_empty() || self.slas.is_empty() {
            0.0
        } else {
            let sla_target_minutes = self
                .slas
                .iter()
                .map(|e| e.value().incident_response_minutes)
                .min()
                .unwrap_or(u32::MAX);

            let total = self.incident_reports.len();
            let met = self
                .incident_reports
                .iter()
                .filter(|e| e.value().resolution_time_minutes <= sla_target_minutes)
                .count();
            (met as f32 / total as f32) * 100.0
        };

        Ok(ContinuityMetrics {
            metrics_id: Uuid::new_v4(),
            plan_id,
            actual_rto_hours: actual_rto,
            actual_rpo_hours: actual_rpo,
            sla_achievement_percent,
            test_success_rate,
        })
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
        planner
            .define_rto(&RTO {
                rto_id: Uuid::new_v4(),
                resource_id: "db1".to_string(),
                recovery_time_hours: 4,
                priority: 1,
            })
            .await
            .unwrap();
        planner
            .define_rpo(&RPO {
                rpo_id: Uuid::new_v4(),
                resource_id: "db1".to_string(),
                recovery_point_hours: 1,
                acceptable_data_loss: "1h".to_string(),
            })
            .await
            .unwrap();

        // Actual recovery beat both targets, so achievement should be 100%.
        let metrics = planner.calculate_metrics(plan_id, 2.5, 0.5).await.unwrap();
        assert!((metrics.sla_achievement_percent - 100.0).abs() < f32::EPSILON);

        // Actual recovery blew past the target, so achievement should degrade.
        let bad_metrics = planner.calculate_metrics(plan_id, 8.0, 0.5).await.unwrap();
        assert!(bad_metrics.sla_achievement_percent < 100.0);
    }

    #[tokio::test]
    async fn test_calculate_metrics_unknown_plan_fails() {
        let planner = ContinuityPlanner::new();
        let result = planner.calculate_metrics(Uuid::new_v4(), 1.0, 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_compliance_and_history() {
        let planner = ContinuityPlanner::new();
        let plan_id = Uuid::new_v4();
        let plan = ContinuityPlan {
            plan_id,
            name: "compliance_test".to_string(),
            organization: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: "1.0".to_string(),
        };
        planner.create_plan(&plan).await.unwrap();

        let status = planner.check_compliance(plan_id).await.unwrap();
        assert!(!status.compliant);
        assert!(!status.missing_items.is_empty());

        let recorded = planner.last_compliance_check(plan_id).unwrap();
        assert_eq!(recorded.status_id, status.status_id);
    }
}
