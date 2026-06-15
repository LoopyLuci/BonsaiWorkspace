use serde::{Serialize, Deserialize};
use std::sync::Arc;
use uuid::Uuid;

/// Capability token for fine-grained access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub id: String,
    pub scope: String,
    pub permissions: Vec<String>,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
    pub issued_to: String,
    pub signature: String,
}

impl CapabilityToken {
    pub fn new(
        scope: impl Into<String>,
        permissions: Vec<String>,
        issued_to: impl Into<String>,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            scope: scope.into(),
            permissions,
            issued_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            expires_at,
            issued_to: issued_to.into(),
            signature: String::new(), // Would be set by Sentinel Core in production
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission || p == "*")
    }

    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            now > exp
        } else {
            false
        }
    }
}

/// Request context carrying metadata and capabilities
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: String,
    pub capability_token: Option<CapabilityToken>,
    pub user_id: Option<String>,
    pub service_name: String,
    pub metadata: std::collections::HashMap<String, String>,
}

impl RequestContext {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
            capability_token: None,
            user_id: None,
            service_name: service_name.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_token(mut self, token: CapabilityToken) -> Self {
        self.capability_token = Some(token);
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn check_permission(&self, permission: &str) -> Result<(), String> {
        match &self.capability_token {
            Some(token) => {
                if token.is_expired() {
                    Err("Capability token expired".to_string())
                } else if token.has_permission(permission) {
                    Ok(())
                } else {
                    Err(format!("Permission denied: {}", permission))
                }
            }
            None => Err("No capability token provided".to_string()),
        }
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}
