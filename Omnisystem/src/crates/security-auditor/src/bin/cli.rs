//! CLI that exercises the security auditor end to end.

use security_auditor::{
    ComplianceFramework, ComplianceManager, FindingStatus, ReportGenerator, RuleEngine,
    SecurityAuditor, SecurityFinding, SecurityPolicy, Severity,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auditor = SecurityAuditor::new();
    auditor.register_policy(SecurityPolicy {
        name: "encryption_policy".to_string(),
        requirements: vec!["AES-256".to_string()],
        enforced: true,
    })?;

    let algorithm = "AES-128";
    if !RuleEngine::check_encryption_strength(algorithm)? {
        auditor.create_finding(SecurityFinding {
            id: "finding-1".to_string(),
            severity: Severity::Critical,
            description: format!("Weak encryption algorithm in use: {algorithm}"),
            remediation: "Upgrade to AES-256 or ChaCha20".to_string(),
            status: FindingStatus::Open,
        })?;
    }

    let findings = auditor.get_critical_findings();
    println!("Critical findings: {}", findings.len());

    let report = ReportGenerator::generate_report(&findings);
    println!(
        "Report: {} finding(s), {} critical, {:.1}% remediated",
        report.findings_count, report.critical_count, report.remediation_rate
    );

    let compliance = ComplianceManager::new();
    compliance.register_framework(ComplianceFramework {
        name: "SOC2".to_string(),
        requirements: vec!["encryption".to_string()],
        compliance_score: 0.85,
    })?;
    println!("Compliance frameworks tracked: {}", compliance.framework_count());

    Ok(())
}
