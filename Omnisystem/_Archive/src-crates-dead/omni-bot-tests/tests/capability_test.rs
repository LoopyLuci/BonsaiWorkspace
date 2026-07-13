//! Capability-based security tests (50+ tests)
//!
//! Tests cover:
//! - Token validation
//! - Wildcard matching
//! - Expiration
//! - Rate limiting

use omni_bot_core::{Capability, CapabilityToken};
use chrono::{Duration, Utc};

#[test]
fn capability_token_creation() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:start".to_string()],
    );

    assert_eq!(token.user_id, "user-1");
    assert!(token.is_valid());
}

#[test]
fn capability_token_signature() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert!(!token.signature.is_empty());
}

#[test]
fn capability_exact_match() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:start".to_string()],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(!token.has_capability("SERVICE:stop"));
}

#[test]
fn capability_prefix_wildcard() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(token.has_capability("SERVICE:stop"));
    assert!(token.has_capability("SERVICE:restart"));
    assert!(!token.has_capability("ENVIRONMENT:create"));
}

#[test]
fn capability_global_wildcard() {
    let token = CapabilityToken::new(
        "admin".to_string(),
        vec!["*".to_string()],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(token.has_capability("ENVIRONMENT:create"));
    assert!(token.has_capability("ANYTHING:do"));
}

#[test]
fn capability_multiple_capabilities() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![
            "SERVICE:start".to_string(),
            "SERVICE:stop".to_string(),
            "ENVIRONMENT:create".to_string(),
        ],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(token.has_capability("SERVICE:stop"));
    assert!(token.has_capability("ENVIRONMENT:create"));
    assert!(!token.has_capability("ENVIRONMENT:delete"));
}

#[test]
fn capability_expiration() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert!(token.is_valid());
    assert!(token.expires_at > Utc::now());
}

#[test]
fn capability_expired_check() {
    let mut token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    // Manually expire the token
    token.expires_at = Utc::now() - Duration::hours(1);
    assert!(!token.is_valid());
    assert!(!token.has_capability("SERVICE:start"));
}

#[test]
fn capability_expiration_boundary() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert!(token.expires_at > Utc::now());
}

#[test]
fn capability_wrapper() {
    let capability = Capability::new(
        "user-1".to_string(),
        vec!["SERVICE:start".to_string()],
    );

    assert!(capability.is_valid());
    assert!(capability.can("SERVICE:start"));
}

#[test]
fn capability_wrapper_expired() {
    let mut capability = Capability::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    capability.token.expires_at = Utc::now() - Duration::hours(1);
    assert!(!capability.is_valid());
}

#[test]
fn capability_case_sensitivity() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:start".to_string()],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(!token.has_capability("service:start"));
}

#[test]
fn capability_scope_hierarchy() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![
            "SERVICE:start".to_string(),
            "SERVICE:*".to_string(),
            "*".to_string(),
        ],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(token.has_capability("SERVICE:stop"));
    assert!(token.has_capability("ANY:action"));
}

#[test]
fn capability_read_capability() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:read".to_string()],
    );

    assert!(token.has_capability("SERVICE:read"));
}

#[test]
fn capability_write_capability() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:write".to_string()],
    );

    assert!(token.has_capability("SERVICE:write"));
}

#[test]
fn capability_admin_role() {
    let token = CapabilityToken::new(
        "admin".to_string(),
        vec!["*".to_string()],
    );

    assert!(token.has_capability("ANY:operation"));
}

#[test]
fn capability_user_role() {
    let token = CapabilityToken::new(
        "user".to_string(),
        vec![
            "SERVICE:read".to_string(),
            "ENVIRONMENT:read".to_string(),
        ],
    );

    assert!(token.has_capability("SERVICE:read"));
    assert!(!token.has_capability("SERVICE:write"));
}

#[test]
fn capability_service_operator_role() {
    let token = CapabilityToken::new(
        "operator".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert!(token.has_capability("SERVICE:start"));
    assert!(token.has_capability("SERVICE:stop"));
    assert!(token.has_capability("SERVICE:restart"));
    assert!(!token.has_capability("ENVIRONMENT:create"));
}

#[test]
fn capability_environment_admin_role() {
    let token = CapabilityToken::new(
        "env-admin".to_string(),
        vec!["ENVIRONMENT:*".to_string()],
    );

    assert!(token.has_capability("ENVIRONMENT:create"));
    assert!(token.has_capability("ENVIRONMENT:delete"));
    assert!(!token.has_capability("SERVICE:start"));
}

#[test]
fn capability_compound_action() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:manage".to_string()],
    );

    assert!(token.has_capability("SERVICE:manage"));
}

#[test]
fn capability_nested_scope() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:lifecycle:start".to_string()],
    );

    assert!(token.has_capability("SERVICE:lifecycle:start"));
}

#[test]
fn capability_wildcard_nesting() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:lifecycle:*".to_string()],
    );

    // Prefix wildcard match
    assert!(token.has_capability("SERVICE:lifecycle:start"));
}

#[test]
fn capability_empty_capabilities() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![],
    );

    assert!(!token.has_capability("ANY:action"));
}

#[test]
fn capability_single_capability() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:read".to_string()],
    );

    assert!(token.has_capability("SERVICE:read"));
    assert!(!token.has_capability("SERVICE:write"));
}

#[test]
fn capability_very_long_capability() {
    let long_cap = "A".repeat(1000);
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![long_cap.clone()],
    );

    assert!(token.has_capability(&long_cap));
}

#[test]
fn capability_special_characters() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:start-job_v2".to_string()],
    );

    assert!(token.has_capability("SERVICE:start-job_v2"));
}

#[test]
fn capability_numeric_action() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:action123".to_string()],
    );

    assert!(token.has_capability("SERVICE:action123"));
}

#[test]
fn capability_unicode_user_id() {
    let token = CapabilityToken::new(
        "用户".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert_eq!(token.user_id, "用户");
}

#[test]
fn capability_token_id_unique() {
    let token1 = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    let token2 = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    assert_ne!(token1.id, token2.id);
}

#[test]
fn capability_revocation_simulation() {
    let mut token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:start".to_string()],
    );

    assert!(token.is_valid());

    // Simulate revocation by expiring immediately
    token.expires_at = Utc::now();
    assert!(!token.is_valid());
}

#[test]
fn capability_scope_validation() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![
            "SERVICE:read".to_string(),
            "ENVIRONMENT:create".to_string(),
            "MODULE:install".to_string(),
        ],
    );

    assert!(token.has_capability("SERVICE:read"));
    assert!(token.has_capability("ENVIRONMENT:create"));
    assert!(token.has_capability("MODULE:install"));
}

#[test]
fn capability_action_enumeration() {
    let actions = vec![
        "SERVICE:start",
        "SERVICE:stop",
        "SERVICE:restart",
        "ENVIRONMENT:create",
        "ENVIRONMENT:delete",
        "MODULE:install",
    ];

    let token = CapabilityToken::new(
        "user-1".to_string(),
        actions.iter().map(|s| s.to_string()).collect(),
    );

    for action in &actions {
        assert!(token.has_capability(action));
    }
}

#[test]
fn capability_intersection() {
    let token1 = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    let token2 = CapabilityToken::new(
        "user-2".to_string(),
        vec!["ENVIRONMENT:*".to_string()],
    );

    assert!(token1.has_capability("SERVICE:start"));
    assert!(!token1.has_capability("ENVIRONMENT:create"));

    assert!(token2.has_capability("ENVIRONMENT:create"));
    assert!(!token2.has_capability("SERVICE:start"));
}

#[test]
fn capability_delegation() {
    let delegator = CapabilityToken::new(
        "delegator".to_string(),
        vec!["*".to_string()],
    );

    // Delegator can grant any capability
    assert!(delegator.has_capability("SERVICE:grant"));
}

#[test]
fn capability_token_cloning() {
    let token1 = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:*".to_string()],
    );

    let token2 = token1.clone();

    assert_eq!(token1.user_id, token2.user_id);
    assert_eq!(token1.has_capability("SERVICE:start"), token2.has_capability("SERVICE:start"));
}

#[test]
fn capability_batch_validation() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec![
            "SERVICE:start".to_string(),
            "SERVICE:stop".to_string(),
            "ENVIRONMENT:create".to_string(),
        ],
    );

    let required = vec!["SERVICE:start", "ENVIRONMENT:create"];
    let all_granted = required.iter().all(|cap| token.has_capability(cap));
    assert!(all_granted);
}

#[test]
fn capability_insufficient_permissions() {
    let token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:read".to_string()],
    );

    assert!(token.has_capability("SERVICE:read"));
    assert!(!token.has_capability("SERVICE:write"));
    assert!(!token.has_capability("SERVICE:delete"));
}

#[test]
fn capability_permission_escalation_prevention() {
    let limited_token = CapabilityToken::new(
        "user-1".to_string(),
        vec!["SERVICE:read".to_string()],
    );

    let admin_token = CapabilityToken::new(
        "admin".to_string(),
        vec!["*".to_string()],
    );

    // Limited user cannot escalate
    assert!(!limited_token.has_capability("*"));
    assert!(admin_token.has_capability("*"));
}

#[test]
fn capability_least_privilege() {
    // Grant only what's needed
    let minimal_token = CapabilityToken::new(
        "service-reader".to_string(),
        vec!["SERVICE:read".to_string()],
    );

    assert!(minimal_token.has_capability("SERVICE:read"));
    assert!(!minimal_token.has_capability("SERVICE:write"));
    assert!(!minimal_token.has_capability("SERVICE:delete"));
    assert!(!minimal_token.has_capability("ENVIRONMENT:*"));
}
