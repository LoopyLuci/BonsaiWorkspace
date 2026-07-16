use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SecurityAuditor {
    findings: Arc<DashMap<String, SecurityFinding>>,
    policies: Arc<DashMap<String, SecurityPolicy>>,
}

#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingStatus {
    Open,
    InProgress,
    Resolved,
    Accepted,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub name: String,
    pub requirements: Vec<String>,
    pub enforced: bool,
}

impl SecurityAuditor {
    pub fn new() -> Self {
        Self {
            findings: Arc::new(DashMap::new()),
            policies: Arc::new(DashMap::new()),
        }
    }

    pub fn register_policy(&self, policy: SecurityPolicy) -> Result<()> {
        self.policies.insert(policy.name.clone(), policy);
        tracing::info!("Policy registered");
        Ok(())
    }

    pub fn create_finding(&self, finding: SecurityFinding) -> Result<()> {
        self.findings.insert(finding.id.clone(), finding);
        tracing::info!("Finding created");
        Ok(())
    }

    pub fn get_critical_findings(&self) -> Vec<SecurityFinding> {
        self.findings
            .iter()
            .filter(|f| f.value().severity == Severity::Critical)
            .map(|f| f.value().clone())
            .collect()
    }

    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for SecurityAuditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auditor() {
        let auditor = SecurityAuditor::new();
        let policy = SecurityPolicy {
            name: "encryption".to_string(),
            requirements: vec!["AES-256".to_string()],
            enforced: true,
        };
        assert!(auditor.register_policy(policy).is_ok());
        assert_eq!(auditor.policy_count(), 1);
    }
}
