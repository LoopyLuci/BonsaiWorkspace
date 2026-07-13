use omnisystem_security::*;

#[test]
fn test_rbac_full() {
    let rbac = RoleBasedAccessControl::new();
    let role = Role {
        name: "user".to_string(),
        permissions: vec![Permission::Read],
    };
    rbac.add_role(role);
    assert!(rbac.has_permission("user", &Permission::Read));
    assert!(!rbac.has_permission("user", &Permission::Admin));
}

#[test]
fn test_audit_logging() {
    let audit = AuditLog::new();
    audit.record("user1".to_string(), "delete".to_string());
    assert_eq!(audit.entry_count(), 1);
}

#[test]
fn test_encryption() {
    let data = b"test";
    let key = b"key";
    let encrypted = Encryptor::encrypt(data, key).unwrap();
    let decrypted = Encryptor::decrypt(&encrypted, key).unwrap();
    assert_eq!(decrypted, data);
}
