use security_auditor::*;

#[test]
fn test_full_audit_workflow() {
    let auditor = auditor::SecurityAuditor::new();
    
    let policy = auditor::SecurityPolicy {
        name: "encryption_policy".to_string(),
        requirements: vec!["AES-256".to_string()],
        enforced: true,
    };
    auditor.register_policy(policy).unwrap();
    
    let finding = auditor::SecurityFinding {
        id: "finding1".to_string(),
        severity: auditor::Severity::Critical,
        description: "Weak encryption detected".to_string(),
        remediation: "Upgrade to AES-256".to_string(),
        status: auditor::FindingStatus::Open,
    };
    auditor.create_finding(finding).unwrap();
    
    let critical = auditor.get_critical_findings();
    assert_eq!(critical.len(), 1);
}
