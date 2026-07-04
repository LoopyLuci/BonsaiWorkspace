/// Compliance Framework
///
/// SOC2, HIPAA, GDPR compliance tracking and reporting

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Compliance framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceFramework {
    SOC2,     // Security, Availability, Integrity, Confidentiality
    HIPAA,    // Health Insurance Portability and Accountability Act
    GDPR,     // General Data Protection Regulation
    PciDss,  // Payment Card Industry Data Security Standard
}

/// Compliance requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequirement {
    pub framework: ComplianceFramework,
    pub requirement_id: String,
    pub description: String,
    pub implemented: bool,
    pub verified: bool,
}

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub framework: ComplianceFramework,
    pub total_requirements: u32,
    pub implemented_count: u32,
    pub verified_count: u32,
    pub last_audit: u64,
}

impl ComplianceStatus {
    /// Get compliance percentage
    pub fn compliance_percentage(&self) -> f64 {
        if self.total_requirements == 0 {
            return 0.0;
        }
        ((self.verified_count as f64) / (self.total_requirements as f64)) * 100.0
    }

    /// Is fully compliant
    pub fn is_fully_compliant(&self) -> bool {
        self.verified_count == self.total_requirements
    }
}

/// Compliance manager
pub struct ComplianceManager {
    requirements: HashMap<ComplianceFramework, Vec<ComplianceRequirement>>,
}

impl ComplianceManager {
    /// Create compliance manager
    pub fn new() -> Result<Self> {
        info!("Initializing Compliance Manager");
        Ok(Self {
            requirements: HashMap::new(),
        })
    }

    /// Add compliance requirement
    pub fn add_requirement(
        &mut self,
        framework: ComplianceFramework,
        requirement_id: String,
        description: String,
    ) -> Result<()> {
        let req = ComplianceRequirement {
            framework,
            requirement_id,
            description,
            implemented: false,
            verified: false,
        };

        self.requirements
            .entry(framework)
            .or_insert_with(Vec::new)
            .push(req);

        Ok(())
    }

    /// Mark requirement as implemented
    pub fn mark_implemented(
        &mut self,
        framework: ComplianceFramework,
        requirement_id: &str,
    ) -> Result<()> {
        if let Some(reqs) = self.requirements.get_mut(&framework) {
            for req in reqs {
                if req.requirement_id == requirement_id {
                    req.implemented = true;
                    info!(
                        "Marked requirement as implemented: {} - {}",
                        framework as u32, requirement_id
                    );
                    return Ok(());
                }
            }
        }

        Err(crate::ClusterError::Network(format!(
            "Requirement not found: {}",
            requirement_id
        )))
    }

    /// Mark requirement as verified (audited)
    pub fn mark_verified(
        &mut self,
        framework: ComplianceFramework,
        requirement_id: &str,
    ) -> Result<()> {
        if let Some(reqs) = self.requirements.get_mut(&framework) {
            for req in reqs {
                if req.requirement_id == requirement_id {
                    req.verified = true;
                    info!(
                        "Marked requirement as verified: {} - {}",
                        framework as u32, requirement_id
                    );
                    return Ok(());
                }
            }
        }

        Err(crate::ClusterError::Network(format!(
            "Requirement not found: {}",
            requirement_id
        )))
    }

    /// Get compliance status for framework
    pub fn get_status(&self, framework: ComplianceFramework) -> ComplianceStatus {
        let reqs = self.requirements.get(&framework).map(|r| r.clone()).unwrap_or_default();

        let total = reqs.len() as u32;
        let implemented = reqs.iter().filter(|r| r.implemented).count() as u32;
        let verified = reqs.iter().filter(|r| r.verified).count() as u32;

        ComplianceStatus {
            framework,
            total_requirements: total,
            implemented_count: implemented,
            verified_count: verified,
            last_audit: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Get all compliance statuses
    pub fn get_all_statuses(&self) -> Vec<ComplianceStatus> {
        vec![
            self.get_status(ComplianceFramework::SOC2),
            self.get_status(ComplianceFramework::HIPAA),
            self.get_status(ComplianceFramework::GDPR),
            self.get_status(ComplianceFramework::PciDss),
        ]
    }

    /// Generate compliance report
    pub fn generate_report(&self) -> String {
        let statuses = self.get_all_statuses();

        let mut report = "COMPLIANCE REPORT\n".to_string();
        report.push_str("================\n\n");

        for status in statuses {
            if status.total_requirements > 0 {
                let framework_name = match status.framework {
                    ComplianceFramework::SOC2 => "SOC 2",
                    ComplianceFramework::HIPAA => "HIPAA",
                    ComplianceFramework::GDPR => "GDPR",
                    ComplianceFramework::PciDss => "PCI-DSS",
                };

                report.push_str(&format!(
                    "{}: {:.1}% compliant ({}/{})\n",
                    framework_name,
                    status.compliance_percentage(),
                    status.verified_count,
                    status.total_requirements
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_manager() {
        let mut mgr = ComplianceManager::new().unwrap();

        mgr.add_requirement(
            ComplianceFramework::SOC2,
            "SC-1".to_string(),
            "Encryption in transit".to_string(),
        )
        .unwrap();

        let status = mgr.get_status(ComplianceFramework::SOC2);
        assert_eq!(status.total_requirements, 1);
        assert!(!status.is_fully_compliant());
    }

    #[test]
    fn test_mark_implemented_and_verified() {
        let mut mgr = ComplianceManager::new().unwrap();

        mgr.add_requirement(
            ComplianceFramework::HIPAA,
            "IA-2".to_string(),
            "User authentication".to_string(),
        )
        .unwrap();

        mgr.mark_implemented(ComplianceFramework::HIPAA, "IA-2")
            .unwrap();
        mgr.mark_verified(ComplianceFramework::HIPAA, "IA-2")
            .unwrap();

        let status = mgr.get_status(ComplianceFramework::HIPAA);
        assert!(status.is_fully_compliant());
        assert_eq!(status.compliance_percentage(), 100.0);
    }
}
