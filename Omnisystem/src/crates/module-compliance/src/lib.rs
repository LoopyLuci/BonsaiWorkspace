use dashmap::DashMap;
use module_interfaces::ModuleError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceFramework {
    HIPAA,
    SOC2,
    GDPR,
    CCPA,
    PCIDSS,
    ISO27001,
    FedRAMP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub id: String,
    pub framework: ComplianceFramework,
    pub requirement: String,
    pub control: String,
    pub status: ComplianceStatus,
    pub evidence: Option<String>,
    pub last_verified: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceStatus {
    NotStarted,
    InProgress,
    Compliant,
    NonCompliant,
    Exempt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: String,
    pub module_id: String,
    pub frameworks: Vec<ComplianceFramework>,
    pub requirements: Vec<ComplianceRequirement>,
    pub overall_status: ComplianceStatus,
    pub generated_at: u64,
    pub next_review: u64,
}

pub struct ComplianceManager {
    requirements: Arc<DashMap<String, Vec<ComplianceRequirement>>>,
    reports: Arc<DashMap<String, ComplianceReport>>,
    framework_configs: Arc<DashMap<String, FrameworkConfig>>,
}

#[derive(Clone, Debug)]
pub struct FrameworkConfig {
    pub framework: ComplianceFramework,
    pub enabled: bool,
    pub review_interval_days: u32,
    pub auto_remediate: bool,
    pub notification_email: Option<String>,
}

impl ComplianceManager {
    pub fn new() -> Self {
        info!("Creating ComplianceManager");
        Self {
            requirements: Arc::new(DashMap::new()),
            reports: Arc::new(DashMap::new()),
            framework_configs: Arc::new(DashMap::new()),
        }
    }

    pub fn enable_framework(&self, framework: ComplianceFramework, config: FrameworkConfig) -> Result<(), ModuleError> {
        debug!("Enabling compliance framework: {:?}", framework);
        let key = format!("{:?}", framework);
        self.framework_configs.insert(key, config);
        Ok(())
    }

    pub fn add_requirement(&self, module_id: String, req: ComplianceRequirement) -> Result<(), ModuleError> {
        debug!("Adding compliance requirement for module: {}", module_id);
        self.requirements
            .entry(module_id)
            .or_insert_with(Vec::new)
            .push(req);
        Ok(())
    }

    pub fn check_module_compliance(&self, module_id: &str, framework: ComplianceFramework) -> Result<ComplianceStatus, ModuleError> {
        debug!("Checking compliance for module: {} - {:?}", module_id, framework);

        match self.requirements.get(module_id) {
            Some(reqs) => {
                let all_compliant = reqs
                    .iter()
                    .filter(|r| r.framework == framework)
                    .all(|r| r.status == ComplianceStatus::Compliant || r.status == ComplianceStatus::Exempt);

                if all_compliant {
                    Ok(ComplianceStatus::Compliant)
                } else {
                    Ok(ComplianceStatus::NonCompliant)
                }
            }
            None => Ok(ComplianceStatus::NotStarted),
        }
    }

    pub fn generate_report(&self, module_id: String) -> Result<ComplianceReport, ModuleError> {
        debug!("Generating compliance report for module: {}", module_id);

        let requirements = self
            .requirements
            .get(&module_id)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        let frameworks = requirements.iter().map(|r| r.framework.clone()).collect::<Vec<_>>();

        let overall_status = if requirements.iter().all(|r| r.status == ComplianceStatus::Compliant) {
            ComplianceStatus::Compliant
        } else {
            ComplianceStatus::NonCompliant
        };

        let report = ComplianceReport {
            id: uuid::Uuid::new_v4().to_string(),
            module_id: module_id.clone(),
            frameworks,
            requirements,
            overall_status,
            generated_at: chrono::Utc::now().timestamp() as u64,
            next_review: (chrono::Utc::now().timestamp() + 86400 * 90) as u64, // 90 days
        };

        self.reports.insert(module_id, report.clone());
        info!("Compliance report generated");
        Ok(report)
    }

    pub fn get_report(&self, module_id: &str) -> Result<ComplianceReport, ModuleError> {
        self.reports
            .get(module_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }

    pub fn list_non_compliant_modules(&self) -> Vec<String> {
        self.requirements
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .iter()
                    .any(|r| r.status == ComplianceStatus::NonCompliant)
            })
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for ComplianceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ComplianceManager {
    fn clone(&self) -> Self {
        Self {
            requirements: Arc::clone(&self.requirements),
            reports: Arc::clone(&self.reports),
            framework_configs: Arc::clone(&self.framework_configs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_manager_creation() {
        let manager = ComplianceManager::new();
        assert_eq!(manager.list_non_compliant_modules().len(), 0);
    }

    #[test]
    fn test_enable_framework() {
        let manager = ComplianceManager::new();
        let config = FrameworkConfig {
            framework: ComplianceFramework::HIPAA,
            enabled: true,
            review_interval_days: 90,
            auto_remediate: true,
            notification_email: Some("admin@example.com".to_string()),
        };
        assert!(manager.enable_framework(ComplianceFramework::HIPAA, config).is_ok());
    }

    #[test]
    fn test_add_requirement() {
        let manager = ComplianceManager::new();
        let req = ComplianceRequirement {
            id: "req1".to_string(),
            framework: ComplianceFramework::HIPAA,
            requirement: "Encryption at rest".to_string(),
            control: "SC-28".to_string(),
            status: ComplianceStatus::Compliant,
            evidence: None,
            last_verified: 0,
        };
        assert!(manager.add_requirement("module1".to_string(), req).is_ok());
    }

    #[test]
    fn test_check_module_compliance() {
        let manager = ComplianceManager::new();
        let req = ComplianceRequirement {
            id: "req1".to_string(),
            framework: ComplianceFramework::HIPAA,
            requirement: "Encryption at rest".to_string(),
            control: "SC-28".to_string(),
            status: ComplianceStatus::Compliant,
            evidence: None,
            last_verified: 0,
        };
        manager.add_requirement("module1".to_string(), req).unwrap();
        assert_eq!(
            manager.check_module_compliance("module1", ComplianceFramework::HIPAA).unwrap(),
            ComplianceStatus::Compliant
        );
    }
}
