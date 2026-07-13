use crate::{AuditError, AuditResult, ComplianceReport};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ComplianceChecker {
    reports: Arc<DashMap<String, ComplianceReport>>,
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(DashMap::new()),
        }
    }

    pub async fn check_compliance(&self, entity_id: &str) -> AuditResult<ComplianceReport> {
        let report = ComplianceReport {
            report_id: uuid::Uuid::new_v4().to_string(),
            violations: 0,
            last_audit: Utc::now(),
            status: "compliant".to_string(),
        };

        self.reports.insert(entity_id.to_string(), report.clone());
        Ok(report)
    }

    pub async fn get_report(&self, entity_id: &str) -> AuditResult<ComplianceReport> {
        self.reports
            .get(entity_id)
            .map(|entry| entry.clone())
            .ok_or(AuditError::QueryFailed)
    }

    pub fn report_count(&self) -> usize {
        self.reports.len()
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_compliance() {
        let checker = ComplianceChecker::new();
        let report = checker.check_compliance("entity1").await.unwrap();
        assert_eq!(report.status, "compliant");
    }

    #[tokio::test]
    async fn test_get_report() {
        let checker = ComplianceChecker::new();
        checker.check_compliance("entity1").await.unwrap();
        let report = checker.get_report("entity1").await.unwrap();
        assert_eq!(report.violations, 0);
    }
}
