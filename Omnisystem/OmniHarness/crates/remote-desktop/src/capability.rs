//! Capability tokens for zero-trust remote desktop access.
//!
//! This module implements Ed25519-signed capability tokens with expiry,
//! revocation, and fine-grained permission checking. Each token is cryptographically
//! signed and includes temporal and capability-specific constraints.

use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use thiserror::Error;
use zeroize::Zeroize;

/// Errors that can occur when working with capability tokens.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Token signature verification failed")]
    VerificationFailed,

    #[error("Token has expired (expired at {expired_at})")]
    Expired { expired_at: DateTime<Utc> },

    #[error("Token is not yet valid (valid from {valid_from})")]
    NotYetValid { valid_from: DateTime<Utc> },

    #[error("Token does not grant capability: {capability}")]
    MissingCapability { capability: String },

    #[error("Invalid token format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Signature error: {0}")]
    SignatureError(String),

    #[error("Tampered token detected")]
    TamperedToken,
}

/// Capability types that can be granted to a token.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Connect to a remote peer.
    Connect,
    /// Capture screen and audio.
    Capture,
    /// Inject input (keyboard, mouse).
    InjectInput,
    /// Transfer files.
    TransferFiles,
    /// Forward TCP ports.
    PortForward,
    /// Full administrative access.
    Admin,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Connect => "connect",
            Capability::Capture => "capture",
            Capability::InjectInput => "inject_input",
            Capability::TransferFiles => "transfer_files",
            Capability::PortForward => "port_forward",
            Capability::Admin => "admin",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Revocation status for a capability token.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum RevocationStatus {
    /// Token is active.
    Active,
    /// Token has been revoked.
    Revoked,
}

/// An Ed25519-signed capability token for remote desktop access.
///
/// Tokens are immutable once created and must be cryptographically verified
/// before use. Each token includes:
/// - Issuer public key
/// - Subject (target peer)
/// - Capabilities granted
/// - Temporal constraints (not before, not after)
/// - Ed25519 signature (8 bytes)
#[derive(Clone, Serialize, Deserialize)]
pub struct RemoteDesktopToken {
    /// Subject (target peer this token applies to).
    pub subject: String,

    /// Capabilities granted by this token.
    pub capabilities: Vec<Capability>,

    /// Not valid before this timestamp.
    pub not_before: DateTime<Utc>,

    /// Not valid after this timestamp.
    pub not_after: DateTime<Utc>,

    /// Issuer public key (Ed25519).
    pub issuer_public_key: Vec<u8>,

    /// Ed25519 signature over the token data.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,

    /// Revocation status.
    pub revocation_status: RevocationStatus,

    /// Optional session ID this token is bound to.
    pub session_id: Option<String>,
}

impl RemoteDesktopToken {
    /// Create a new unsigned capability token.
    pub fn new(
        subject: String,
        capabilities: Vec<Capability>,
        duration: Duration,
    ) -> Self {
        let now = Utc::now();
        RemoteDesktopToken {
            subject,
            capabilities,
            not_before: now,
            not_after: now + duration,
            issuer_public_key: vec![],
            signature: vec![],
            revocation_status: RevocationStatus::Active,
            session_id: None,
        }
    }

    /// Sign this token with the provided Ed25519 private key.
    pub fn sign(&mut self, private_key: &SigningKey) -> Result<(), TokenError> {
        // Serialize token data (excluding signature).
        let data_to_sign = self.data_to_sign()?;

        // Sign the data.
        let signature = private_key.sign(&data_to_sign);

        // Store signature and issuer public key.
        self.signature = signature.to_bytes().to_vec();
        self.issuer_public_key = private_key.verifying_key().to_bytes().to_vec();

        Ok(())
    }

    /// Verify the signature and temporal constraints of this token.
    pub fn verify(&self) -> Result<(), TokenError> {
        // Check if token has been revoked.
        if self.revocation_status == RevocationStatus::Revoked {
            return Err(TokenError::TamperedToken);
        }

        // Check temporal validity.
        let now = Utc::now();
        if now < self.not_before {
            return Err(TokenError::NotYetValid {
                valid_from: self.not_before,
            });
        }
        if now > self.not_after {
            return Err(TokenError::Expired {
                expired_at: self.not_after,
            });
        }

        // Verify signature.
        if self.signature.is_empty() || self.issuer_public_key.is_empty() {
            return Err(TokenError::InvalidFormat {
                reason: "Missing signature or public key".to_string(),
            });
        }

        let verifying_key = VerifyingKey::from_bytes(
            self.issuer_public_key
                .as_slice()
                .try_into()
                .map_err(|_| TokenError::InvalidFormat {
                    reason: "Invalid public key length".to_string(),
                })?,
        )
        .map_err(|_| TokenError::InvalidFormat {
            reason: "Invalid public key format".to_string(),
        })?;

        let signature = Signature::from_slice(&self.signature).map_err(|_| {
            TokenError::InvalidFormat {
                reason: "Invalid signature format".to_string(),
            }
        })?;

        let data_to_verify = self.data_to_sign()?;
        verifying_key
            .verify(&data_to_verify, &signature)
            .map_err(|_| TokenError::VerificationFailed)?;

        Ok(())
    }

    /// Check if this token grants a specific capability.
    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Check if all required capabilities are granted.
    pub fn has_all_capabilities(&self, required: &[Capability]) -> Result<(), TokenError> {
        for cap in required {
            if !self.has_capability(*cap) {
                return Err(TokenError::MissingCapability {
                    capability: cap.as_str().to_string(),
                });
            }
        }
        Ok(())
    }

    /// Revoke this token (marks it as no longer valid).
    pub fn revoke(&mut self) {
        self.revocation_status = RevocationStatus::Revoked;
    }

    /// Bind this token to a specific session.
    pub fn bind_to_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    /// Get the data that was signed (for verification purposes).
    fn data_to_sign(&self) -> Result<Vec<u8>, TokenError> {
        let mut hasher = Sha256::new();
        hasher.update(self.subject.as_bytes());

        for cap in &self.capabilities {
            hasher.update(cap.as_str().as_bytes());
        }

        hasher.update(self.not_before.to_rfc3339().as_bytes());
        hasher.update(self.not_after.to_rfc3339().as_bytes());

        Ok(hasher.finalize().to_vec())
    }
}

impl fmt::Debug for RemoteDesktopToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteDesktopToken")
            .field("subject", &self.subject)
            .field("capabilities", &self.capabilities)
            .field("not_before", &self.not_before)
            .field("not_after", &self.not_after)
            .field("issuer_public_key", &format!("{}...", hex::encode(&self.issuer_public_key[..8.min(self.issuer_public_key.len())])))
            .field("signature_len", &self.signature.len())
            .field("revocation_status", &self.revocation_status)
            .field("session_id", &self.session_id)
            .finish()
    }
}

impl Drop for RemoteDesktopToken {
    fn drop(&mut self) {
        self.issuer_public_key.zeroize();
        self.signature.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::thread_rng;

    #[test]
    fn test_create_token() {
        let token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect, Capability::Capture],
            Duration::hours(1),
        );

        assert_eq!(token.subject, "peer-123");
        assert_eq!(token.capabilities.len(), 2);
    }

    #[test]
    fn test_sign_and_verify() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);

        let mut token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect],
            Duration::hours(1),
        );

        token.sign(&signing_key).unwrap();
        token.verify().unwrap();
    }

    #[test]
    fn test_tampered_signature_fails() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);

        let mut token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect],
            Duration::hours(1),
        );

        token.sign(&signing_key).unwrap();

        // Tamper with the token.
        token.subject = "peer-456".to_string();

        let result = token.verify();
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_fails() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);

        let mut token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect],
            Duration::seconds(-1), // Already expired
        );

        token.sign(&signing_key).unwrap();
        let result = token.verify();
        assert!(result.is_err());
    }

    #[test]
    fn test_has_capability() {
        let token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect, Capability::Capture],
            Duration::hours(1),
        );

        assert!(token.has_capability(Capability::Connect));
        assert!(token.has_capability(Capability::Capture));
        assert!(!token.has_capability(Capability::InjectInput));
    }

    #[test]
    fn test_revoke_token() {
        let mut token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect],
            Duration::hours(1),
        );

        token.revoke();
        let result = token.verify();
        assert!(result.is_err());
    }

    #[test]
    fn test_bind_to_session() {
        let mut token = RemoteDesktopToken::new(
            "peer-123".to_string(),
            vec![Capability::Connect],
            Duration::hours(1),
        );

        token.bind_to_session("session-456".to_string());
        assert_eq!(token.session_id, Some("session-456".to_string()));
    }
}
