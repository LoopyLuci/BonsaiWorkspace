use crate::SecurityFinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub timestamp: u64,
    pub findings_count: u32,
    pub critical_count: u32,
    pub remediation_rate: f32,
}

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn generate_report(findings: &[SecurityFinding]) -> AuditReport {
        let critical_count = findings
            .iter()
            .filter(|f| f.severity == crate::Severity::Critical)
            .count() as u32;
        
        let resolved_count = findings
            .iter()
            .filter(|f| f.status == crate::FindingStatus::Resolved)
            .count() as u32;
        
        AuditReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            findings_count: findings.len() as u32,
            critical_count,
            remediation_rate: if findings.is_empty() {
                100.0
            } else {
                (resolved_count as f32 / findings.len() as f32) * 100.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let finding = SecurityFinding {
            id: "f1".to_string(),
            severity: crate::Severity::High,
            description: "test".to_string(),
            remediation: "fix".to_string(),
            status: crate::FindingStatus::Open,
        };
        let report = ReportGenerator::generate_report(&[finding]);
        assert_eq!(report.findings_count, 1);
    }
}
