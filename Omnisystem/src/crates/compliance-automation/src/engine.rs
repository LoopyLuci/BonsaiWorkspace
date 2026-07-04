use crate::{CompliancePolicy, ComplianceFramework, PolicyEvaluation, ComplianceViolation, ViolationSeverity, ComplianceReport, RegulatoryRequirement, ComplianceError, ComplianceResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ComplianceEngine {
    policies: Arc<DashMap<Uuid, CompliancePolicy>>,
    evaluations: Arc<DashMap<Uuid, PolicyEvaluation>>,
    violations: Arc<DashMap<Uuid, ComplianceViolation>>,
    reports: Arc<DashMap<Uuid, ComplianceReport>>,
    requirements: Arc<DashMap<Uuid, RegulatoryRequirement>>,
}

impl ComplianceEngine {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            evaluations: Arc::new(DashMap::new()),
            violations: Arc::new(DashMap::new()),
            reports: Arc::new(DashMap::new()),
            requirements: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_policy(&self, framework: ComplianceFramework, name: &str, requirement: &str) -> ComplianceResult<CompliancePolicy> {
        let policy = CompliancePolicy {
            policy_id: Uuid::new_v4(),
            framework,
            policy_name: name.to_string(),
            requirement: requirement.to_string(),
            created_at: Utc::now(),
            enabled: true,
        };

        self.policies.insert(policy.policy_id, policy.clone());
        Ok(policy)
    }

    pub async fn evaluate_policy(&self, policy_id: Uuid, compliant: bool, score: f64) -> ComplianceResult<PolicyEvaluation> {
        if self.policies.get(&policy_id).is_none() {
            return Err(ComplianceError::PolicyNotFound);
        }

        let evaluation = PolicyEvaluation {
            evaluation_id: Uuid::new_v4(),
            policy_id,
            evaluated_at: Utc::now(),
            compliant,
            score,
            findings: vec![],
        };

        self.evaluations.insert(evaluation.evaluation_id, evaluation.clone());
        Ok(evaluation)
    }

    pub async fn record_violation(&self, policy_id: Uuid, violation_type: &str, severity: ViolationSeverity) -> ComplianceResult<ComplianceViolation> {
        let violation = ComplianceViolation {
            violation_id: Uuid::new_v4(),
            policy_id,
            violation_type: violation_type.to_string(),
            severity,
            detected_at: Utc::now(),
            remediation_due: Utc::now() + chrono::Duration::days(30),
        };

        self.violations.insert(violation.violation_id, violation.clone());
        Ok(violation)
    }

    pub async fn generate_report(&self, framework: ComplianceFramework) -> ComplianceResult<ComplianceReport> {
        let mut total_policies = 0;
        let mut compliant_count = 0;
        let mut total_score = 0.0;

        for entry in self.evaluations.iter() {
            total_policies += 1;
            total_score += entry.value().score;
            if entry.value().compliant {
                compliant_count += 1;
            }
        }

        let violation_count = self.violations.len() as u32;
        let overall_score = if total_policies > 0 { total_score / total_policies as f64 } else { 0.0 };

        let report = ComplianceReport {
            report_id: Uuid::new_v4(),
            framework,
            generated_at: Utc::now(),
            total_policies: total_policies as u32,
            compliant_count: compliant_count as u32,
            violation_count,
            overall_score,
        };

        self.reports.insert(report.report_id, report.clone());
        Ok(report)
    }

    pub async fn track_regulatory_requirement(&self, framework: ComplianceFramework, code: &str, description: &str) -> ComplianceResult<RegulatoryRequirement> {
        let req = RegulatoryRequirement {
            requirement_id: Uuid::new_v4(),
            framework,
            requirement_code: code.to_string(),
            description: description.to_string(),
            last_audited: None,
        };

        self.requirements.insert(req.requirement_id, req.clone());
        Ok(req)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_policy() {
        let engine = ComplianceEngine::new();
        let policy = engine
            .create_policy(ComplianceFramework::HIPAA, "patient_data_encryption", "All patient data must be AES-256 encrypted")
            .await
            .unwrap();

        assert_eq!(policy.framework, ComplianceFramework::HIPAA);
        assert_eq!(engine.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_evaluate_policy() {
        let engine = ComplianceEngine::new();
        let policy = engine
            .create_policy(ComplianceFramework::SOC2, "access_control", "Role-based access control required")
            .await
            .unwrap();

        let eval = engine.evaluate_policy(policy.policy_id, true, 0.95).await.unwrap();
        assert!(eval.compliant);
    }

    #[tokio::test]
    async fn test_record_violation() {
        let engine = ComplianceEngine::new();
        let policy = engine
            .create_policy(ComplianceFramework::GDPR, "data_retention", "Delete data after 3 years")
            .await
            .unwrap();

        let violation = engine
            .record_violation(policy.policy_id, "data_not_deleted", ViolationSeverity::High)
            .await
            .unwrap();

        assert_eq!(violation.severity, ViolationSeverity::High);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let engine = ComplianceEngine::new();
        let policy = engine
            .create_policy(ComplianceFramework::PCI_DSS, "encryption", "Payment data encryption")
            .await
            .unwrap();

        engine.evaluate_policy(policy.policy_id, true, 0.98).await.unwrap();

        let report = engine.generate_report(ComplianceFramework::PCI_DSS).await.unwrap();
        assert_eq!(report.total_policies, 1);
        assert_eq!(report.compliant_count, 1);
    }
}
