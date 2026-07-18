/// Phase 12: Compliance & Audit Testing
///
/// RBAC, audit logging, encryption at rest, SOC2/HIPAA/GDPR compliance

use omnisystem_cluster::*;

#[test]
fn test_rbac_basic() {
    let mut mgr = rbac::RBACManager::new().unwrap();

    mgr.add_user("alice".to_string(), rbac::Role::Admin)
        .unwrap();
    mgr.add_user("bob".to_string(), rbac::Role::Auditor)
        .unwrap();

    // Alice (admin) can write
    assert!(mgr
        .has_permission("alice", rbac::Permission::Write)
        .unwrap());

    // Bob (auditor) cannot write
    assert!(!mgr
        .has_permission("bob", rbac::Permission::Write)
        .unwrap());
}

#[test]
fn test_rbac_role_hierarchy() {
    let admin_perms = rbac::Role::Admin.permissions();
    let replica_perms = rbac::Role::Replica.permissions();

    // Admin has more permissions than replica
    assert!(admin_perms.len() > replica_perms.len());

    // Replica has read and replicate
    assert!(replica_perms.contains(&rbac::Permission::Read));
    assert!(replica_perms.contains(&rbac::Permission::Replicate));

    // But not write or delete
    assert!(!replica_perms.contains(&rbac::Permission::Write));
    assert!(!replica_perms.contains(&rbac::Permission::Delete));
}

#[test]
fn test_audit_log_basic() {
    let mut logger = audit::AuditLogger::new(90).unwrap();

    logger
        .log_event(
            audit::AuditEventType::UserCreated,
            "admin".to_string(),
            "user:alice".to_string(),
            "create".to_string(),
            audit::AuditResult::Success,
            "Created new admin user".to_string(),
        )
        .unwrap();

    let entries = logger.get_entries(10);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].event_type,
        audit::AuditEventType::UserCreated
    );
}

#[test]
fn test_audit_log_security_events() {
    let mut logger = audit::AuditLogger::new(90).unwrap();

    // Log successful access
    logger
        .log_event(
            audit::AuditEventType::DataRead,
            "user1".to_string(),
            "data:123".to_string(),
            "read".to_string(),
            audit::AuditResult::Success,
            "User read data".to_string(),
        )
        .unwrap();

    // Log failed access attempt
    logger
        .log_event(
            audit::AuditEventType::AccessDenied,
            "user2".to_string(),
            "admin_panel".to_string(),
            "access".to_string(),
            audit::AuditResult::Failure,
            "User lacks admin permission".to_string(),
        )
        .unwrap();

    let failed = logger.get_failed_attempts();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].user_id, "user2");
}

#[test]
fn test_audit_log_query() {
    let mut logger = audit::AuditLogger::new(90).unwrap();

    // Log events from different users
    for i in 0..5 {
        logger
            .log_event(
                audit::AuditEventType::DataWrite,
                format!("user{}", i),
                "data".to_string(),
                "write".to_string(),
                audit::AuditResult::Success,
                "".to_string(),
            )
            .unwrap();
    }

    // Query by user
    let user1_events = logger.query_by_user("user1");
    assert_eq!(user1_events.len(), 1);

    // Query by type
    let write_events = logger.query_by_type(audit::AuditEventType::DataWrite);
    assert_eq!(write_events.len(), 5);
}

#[test]
fn test_encryption_at_rest_aes256() {
    let mgr = encryption_at_rest::EncryptionAtRestManager::new(
        encryption_at_rest::EncryptionAlgorithm::AES256GCM,
    )
    .unwrap();

    let plaintext = b"sensitive credential";
    let ciphertext = mgr.encrypt_at_rest(plaintext).unwrap();

    // Ciphertext should be different from plaintext
    assert_ne!(ciphertext, plaintext);

    // Should be able to decrypt
    let decrypted = mgr.decrypt_at_rest(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encryption_at_rest_chacha20() {
    let mgr = encryption_at_rest::EncryptionAtRestManager::new(
        encryption_at_rest::EncryptionAlgorithm::ChaCha20,
    )
    .unwrap();

    let plaintext = b"database password";
    let ciphertext = mgr.encrypt_at_rest(plaintext).unwrap();

    let decrypted = mgr.decrypt_at_rest(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[tokio::test]
async fn test_key_rotation() {
    let mut mgr =
        encryption_at_rest::EncryptionAtRestManager::new(
            encryption_at_rest::EncryptionAlgorithm::AES256GCM,
        )
        .unwrap();

    assert_eq!(mgr.rotation_count(), 0);

    mgr.rotate_key().await.unwrap();
    assert_eq!(mgr.rotation_count(), 1);

    // Key should still decrypt data from before rotation (in real scenario)
    let plaintext = b"test data";
    let encrypted = mgr.encrypt_at_rest(plaintext).unwrap();
    let decrypted = mgr.decrypt_at_rest(&encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_compliance_soc2() {
    let mut mgr = compliance::ComplianceManager::new().unwrap();

    // Add SOC2 requirements
    mgr.add_requirement(
        compliance::ComplianceFramework::SOC2,
        "SC-1".to_string(),
        "Encryption in transit".to_string(),
    )
    .unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::SOC2,
        "SC-2".to_string(),
        "Encryption at rest".to_string(),
    )
    .unwrap();

    let status = mgr.get_status(compliance::ComplianceFramework::SOC2);
    assert_eq!(status.total_requirements, 2);
    assert!(!status.is_fully_compliant());

    // Mark as implemented and verified
    mgr.mark_implemented(compliance::ComplianceFramework::SOC2, "SC-1")
        .unwrap();
    mgr.mark_verified(compliance::ComplianceFramework::SOC2, "SC-1")
        .unwrap();

    mgr.mark_implemented(compliance::ComplianceFramework::SOC2, "SC-2")
        .unwrap();
    mgr.mark_verified(compliance::ComplianceFramework::SOC2, "SC-2")
        .unwrap();

    let updated_status = mgr.get_status(compliance::ComplianceFramework::SOC2);
    assert!(updated_status.is_fully_compliant());
    assert_eq!(updated_status.compliance_percentage(), 100.0);
}

#[test]
fn test_compliance_hipaa() {
    let mut mgr = compliance::ComplianceManager::new().unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::HIPAA,
        "IA-2".to_string(),
        "User authentication".to_string(),
    )
    .unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::HIPAA,
        "AU-2".to_string(),
        "Audit and accountability".to_string(),
    )
    .unwrap();

    // Mark first requirement as verified
    mgr.mark_implemented(compliance::ComplianceFramework::HIPAA, "IA-2")
        .unwrap();
    mgr.mark_verified(compliance::ComplianceFramework::HIPAA, "IA-2")
        .unwrap();

    let status = mgr.get_status(compliance::ComplianceFramework::HIPAA);
    assert_eq!(status.compliance_percentage(), 50.0);
}

#[test]
fn test_compliance_gdpr() {
    let mut mgr = compliance::ComplianceManager::new().unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::GDPR,
        "Art-32".to_string(),
        "Data protection".to_string(),
    )
    .unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::GDPR,
        "Art-35".to_string(),
        "Data impact assessment".to_string(),
    )
    .unwrap();

    let status = mgr.get_status(compliance::ComplianceFramework::GDPR);
    assert_eq!(status.total_requirements, 2);
}

#[test]
fn test_compliance_report() {
    let mut mgr = compliance::ComplianceManager::new().unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::SOC2,
        "SC-1".to_string(),
        "Encryption in transit".to_string(),
    )
    .unwrap();

    let report = mgr.generate_report();
    assert!(report.contains("SOC 2"));
    assert!(report.contains("0.0% compliant"));
}

#[test]
fn test_multi_framework_compliance() {
    let mut mgr = compliance::ComplianceManager::new().unwrap();

    // Add requirements for multiple frameworks
    mgr.add_requirement(
        compliance::ComplianceFramework::SOC2,
        "SC-1".to_string(),
        "Encryption in transit".to_string(),
    )
    .unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::HIPAA,
        "IA-2".to_string(),
        "User authentication".to_string(),
    )
    .unwrap();

    mgr.add_requirement(
        compliance::ComplianceFramework::GDPR,
        "Art-32".to_string(),
        "Data protection".to_string(),
    )
    .unwrap();

    let statuses = mgr.get_all_statuses();

    // Verify all frameworks are represented
    let soc2_status = statuses
        .iter()
        .find(|s| s.framework == compliance::ComplianceFramework::SOC2);
    assert!(soc2_status.is_some());

    let hipaa_status = statuses
        .iter()
        .find(|s| s.framework == compliance::ComplianceFramework::HIPAA);
    assert!(hipaa_status.is_some());

    let gdpr_status = statuses
        .iter()
        .find(|s| s.framework == compliance::ComplianceFramework::GDPR);
    assert!(gdpr_status.is_some());
}

#[test]
fn test_end_to_end_compliance_scenario() {
    // Scenario: Track compliance for healthcare provider
    let mut rbac_mgr = rbac::RBACManager::new().unwrap();
    let mut audit_logger = audit::AuditLogger::new(365).unwrap(); // 1 year retention
    let encryption_mgr = encryption_at_rest::EncryptionAtRestManager::new(
        encryption_at_rest::EncryptionAlgorithm::AES256GCM,
    )
    .unwrap();
    let mut compliance_mgr = compliance::ComplianceManager::new().unwrap();

    // Setup RBAC
    rbac_mgr
        .add_user("dr_alice".to_string(), rbac::Role::Leader)
        .unwrap();
    rbac_mgr
        .add_user("nurse_bob".to_string(), rbac::Role::Replica)
        .unwrap();
    rbac_mgr
        .add_user("auditor_carol".to_string(), rbac::Role::Auditor)
        .unwrap();

    // Log patient data access
    audit_logger
        .log_event(
            audit::AuditEventType::DataRead,
            "dr_alice".to_string(),
            "patient:12345".to_string(),
            "read".to_string(),
            audit::AuditResult::Success,
            "Doctor viewed patient record".to_string(),
        )
        .unwrap();

    // Encrypt patient data
    let patient_ssn = "123-45-6789";
    let _encrypted = encryption_mgr.encrypt_at_rest(patient_ssn.as_bytes()).unwrap();

    // Track HIPAA compliance
    compliance_mgr
        .add_requirement(
            compliance::ComplianceFramework::HIPAA,
            "IA-2".to_string(),
            "User authentication (MFA)".to_string(),
        )
        .unwrap();

    compliance_mgr
        .add_requirement(
            compliance::ComplianceFramework::HIPAA,
            "SC-2".to_string(),
            "Encryption at rest".to_string(),
        )
        .unwrap();

    compliance_mgr
        .add_requirement(
            compliance::ComplianceFramework::HIPAA,
            "AU-2".to_string(),
            "Audit and accountability".to_string(),
        )
        .unwrap();

    // Verify requirements
    compliance_mgr
        .mark_implemented(compliance::ComplianceFramework::HIPAA, "IA-2")
        .unwrap();
    compliance_mgr
        .mark_verified(compliance::ComplianceFramework::HIPAA, "IA-2")
        .unwrap();

    compliance_mgr
        .mark_implemented(compliance::ComplianceFramework::HIPAA, "SC-2")
        .unwrap();
    compliance_mgr
        .mark_verified(compliance::ComplianceFramework::HIPAA, "SC-2")
        .unwrap();

    // Compliance: 66% (2 out of 3)
    let status = compliance_mgr.get_status(compliance::ComplianceFramework::HIPAA);
    assert_eq!(status.implemented_count, 2);
    assert_eq!(status.verified_count, 2);
}
