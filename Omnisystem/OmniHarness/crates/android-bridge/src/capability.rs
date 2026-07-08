use crate::error::Result;
use chrono::{DateTime, Utc};
use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Capability types for zero-trust access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CapabilityType {
    /// Screen streaming capability
    ScreenStream,
    /// Input injection (touch, keyboard, mouse)
    InputInjection,
    /// File read access
    FileRead,
    /// File write access
    FileWrite,
    /// App installation/deployment
    AppDeploy,
    /// App hot reload
    AppHotReload,
    /// Sensor access (GPS, accelerometer, etc.)
    SensorAccess,
    /// Device configuration
    DeviceConfig,
    /// System shell commands
    ShellExecution,
    /// Custom capability
    Custom(String),
}

/// Signed capability token with revocation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Unique token ID
    pub id: Uuid,
    /// Capability type
    pub capability: CapabilityType,
    /// Device ID this token is bound to
    pub device_id: String,
    /// Subject (agent/user) this token is issued to
    pub subject: String,
    /// Issue timestamp
    pub issued_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Revocation status
    pub revoked: bool,
    /// Scope/context for the capability
    pub scope: Option<String>,
    /// Ed25519 signature of the token
    #[serde(skip)]
    pub signature: Vec<u8>,
    /// Signing key (only present on issuer side)
    #[serde(skip)]
    pub signing_key: Option<SigningKey>,
}

impl CapabilityToken {
    /// Create a new unsigned capability token
    pub fn new(
        capability: CapabilityType,
        device_id: String,
        subject: String,
        expires_at: DateTime<Utc>,
        scope: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            capability,
            device_id,
            subject,
            issued_at: Utc::now(),
            expires_at,
            revoked: false,
            scope,
            signature: Vec::new(),
            signing_key: None,
        }
    }

    /// Check if token is still valid
    pub fn is_valid(&self) -> bool {
        !self.revoked && Utc::now() < self.expires_at
    }

    /// Sign the token with a private key
    pub fn sign(&mut self, signing_key: SigningKey) -> Result<()> {
        let payload = self.signing_payload()?;
        self.signature = signing_key.sign(&payload).to_bytes().to_vec();
        self.signing_key = Some(signing_key);
        Ok(())
    }

    /// Verify token signature
    pub fn verify(&self, verifying_key: VerifyingKey) -> Result<()> {
        if self.signature.is_empty() {
            return Err(crate::error::Error::CapabilityError(
                "Token not signed".to_string(),
            ));
        }

        let payload = self.signing_payload()?;
        use ed25519_dalek::Signature;
        let sig = Signature::from_bytes(
            self.signature
                .as_slice()
                .try_into()
                .map_err(|_| {
                    crate::error::Error::CapabilityError("Invalid signature length".to_string())
                })?
        );

        verifying_key
            .verify_strict(&payload, &sig)
            .map_err(|_| {
                crate::error::Error::CapabilityError("Signature verification failed".to_string())
            })
    }

    /// Get the payload to sign (excluding signature itself)
    fn signing_payload(&self) -> Result<Vec<u8>> {
        let token_without_sig = TokenForSigning {
            id: self.id,
            capability: self.capability.clone(),
            device_id: self.device_id.clone(),
            subject: self.subject.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            revoked: self.revoked,
            scope: self.scope.clone(),
        };

        serde_json::to_vec(&token_without_sig)
            .map_err(|e| crate::error::Error::SerializationError(e))
    }

    /// Revoke the token
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenForSigning {
    id: Uuid,
    capability: CapabilityType,
    device_id: String,
    subject: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked: bool,
    scope: Option<String>,
}

/// Capability registry for managing device-specific capabilities
pub struct CapabilityRegistry {
    tokens: parking_lot::RwLock<std::collections::HashMap<Uuid, CapabilityToken>>,
    revoked_ids: parking_lot::RwLock<std::collections::HashSet<Uuid>>,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            tokens: parking_lot::RwLock::new(std::collections::HashMap::new()),
            revoked_ids: parking_lot::RwLock::new(std::collections::HashSet::new()),
        }
    }

    /// Issue a new capability token
    pub fn issue_token(&self, token: CapabilityToken) -> Result<()> {
        if !token.is_valid() {
            return Err(crate::error::Error::CapabilityError(
                "Token not valid".to_string(),
            ));
        }
        self.tokens.write().insert(token.id, token);
        Ok(())
    }

    /// Check if a capability is available
    pub fn has_capability(
        &self,
        device_id: &str,
        subject: &str,
        capability: &CapabilityType,
    ) -> bool {
        self.tokens
            .read()
            .values()
            .any(|token| {
                token.device_id == device_id
                    && token.subject == subject
                    && &token.capability == capability
                    && token.is_valid()
            })
    }

    /// Revoke a capability token by ID
    pub fn revoke_token(&self, token_id: Uuid) -> Result<()> {
        let mut tokens = self.tokens.write();
        if let Some(token) = tokens.get_mut(&token_id) {
            token.revoke();
            self.revoked_ids.write().insert(token_id);
            Ok(())
        } else {
            Err(crate::error::Error::CapabilityError(
                "Token not found".to_string(),
            ))
        }
    }

    /// Revoke all tokens for a device
    pub fn revoke_device_tokens(&self, device_id: &str) {
        let mut tokens = self.tokens.write();
        for token in tokens.values_mut() {
            if token.device_id == device_id {
                token.revoke();
            }
        }
    }

    /// Get all tokens for a device
    pub fn get_device_tokens(&self, device_id: &str) -> Vec<CapabilityToken> {
        self.tokens
            .read()
            .values()
            .filter(|t| t.device_id == device_id)
            .cloned()
            .collect()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_validity() {
        let token = CapabilityToken::new(
            CapabilityType::ScreenStream,
            "device123".to_string(),
            "agent1".to_string(),
            Utc::now() + chrono::Duration::hours(1),
            None,
        );

        assert!(token.is_valid());
    }

    #[test]
    fn test_token_expiration() {
        let token = CapabilityToken::new(
            CapabilityType::ScreenStream,
            "device123".to_string(),
            "agent1".to_string(),
            Utc::now() - chrono::Duration::seconds(1),
            None,
        );

        assert!(!token.is_valid());
    }

    #[test]
    fn test_capability_registry() {
        let registry = CapabilityRegistry::new();
        let token = CapabilityToken::new(
            CapabilityType::ScreenStream,
            "device123".to_string(),
            "agent1".to_string(),
            Utc::now() + chrono::Duration::hours(1),
            None,
        );

        assert!(registry.issue_token(token).is_ok());
        assert!(registry.has_capability("device123", "agent1", &CapabilityType::ScreenStream));
    }
}
