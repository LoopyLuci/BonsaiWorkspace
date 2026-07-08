use crate::error::Result;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

/// Noise protocol-based session establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseHandshake {
    /// Ephemeral public key
    pub ephemeral_pk: [u8; 32],
    /// Static public key
    pub static_pk: [u8; 32],
    /// Payload (optional)
    pub payload: Option<Vec<u8>>,
}

/// Secure session key derived from Noise protocol
#[derive(Clone)]
pub struct SessionKey {
    /// Encryption key
    key: [u8; 32],
    /// Nonce counter
    nonce_counter: Arc<parking_lot::Mutex<u64>>,
}

impl SessionKey {
    /// Create a new session key from raw bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Self {
            key,
            nonce_counter: Arc::new(parking_lot::Mutex::new(0)),
        }
    }

    /// Derive session key from Noise DH result
    pub fn from_dh_result(dh_result: &[u8]) -> Result<Self> {
        let hash = blake3::hash(dh_result);
        let key_bytes = hash.as_bytes()[..32].to_vec();
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self {
            key,
            nonce_counter: Arc::new(parking_lot::Mutex::new(0)),
        })
    }

    /// Encrypt data with AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut counter = self.nonce_counter.lock();
        let nonce_bytes = counter.to_le_bytes();
        *counter += 1;

        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(&nonce_bytes);

        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce_slice = aes_gcm::Nonce::from_slice(&nonce);

        cipher
            .encrypt(nonce_slice, plaintext)
            .map_err(|e| crate::error::Error::CryptoError(e.to_string()))
    }

    /// Decrypt data with AES-256-GCM
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 20 {
            return Err(crate::error::Error::CryptoError(
                "Ciphertext too short".to_string(),
            ));
        }

        let (nonce_bytes, encrypted_payload) = ciphertext.split_at(8);
        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce_slice = aes_gcm::Nonce::from_slice(&nonce);

        cipher
            .decrypt(nonce_slice, encrypted_payload)
            .map_err(|e| crate::error::Error::CryptoError(e.to_string()))
    }
}

impl Drop for SessionKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

/// Device identity with public/private key pair
#[derive(Clone)]
pub struct DeviceIdentity {
    /// Device fingerprint (public key hash)
    pub fingerprint: String,
    /// Static secret for Noise protocol
    pub secret_key: Arc<parking_lot::Mutex<StaticSecret>>,
    /// Public key
    pub public_key: PublicKey,
}

impl fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceIdentity")
            .field("fingerprint", &self.fingerprint)
            .field("secret_key", &"<secret>")
            .field("public_key", &"<public>")
            .finish()
    }
}

impl DeviceIdentity {
    /// Generate new device identity
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let secret_bytes: [u8; 32] = rng.gen();
        let secret_key = StaticSecret::from(secret_bytes);
        let public_key = PublicKey::from(&secret_key);

        let fingerprint = blake3::hash(public_key.as_bytes())
            .to_hex()
            .to_string()[..16]
            .to_string();

        Self {
            fingerprint,
            secret_key: Arc::new(parking_lot::Mutex::new(secret_key)),
            public_key,
        }
    }

    /// Create device identity from existing secret
    pub fn from_secret(secret: &[u8; 32]) -> Self {
        let secret_key = StaticSecret::from(*secret);
        let public_key = PublicKey::from(&secret_key);

        let fingerprint = blake3::hash(public_key.as_bytes())
            .to_hex()
            .to_string()[..16]
            .to_string();

        Self {
            fingerprint,
            secret_key: Arc::new(parking_lot::Mutex::new(secret_key)),
            public_key,
        }
    }

    /// Perform X25519 DH with remote public key
    pub fn dh(&self, remote_pk: &PublicKey) -> Vec<u8> {
        let secret = self.secret_key.lock();
        let shared_secret = secret.diffie_hellman(remote_pk);
        shared_secret.as_bytes().to_vec()
    }
}

/// Secure channel for encrypted communication
pub struct SecureChannel {
    session_key: SessionKey,
    tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    rx: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl SecureChannel {
    /// Create new secure channel
    pub fn new(
        session_key: SessionKey,
        tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        Self {
            session_key,
            tx,
            rx: tokio::sync::Mutex::new(rx),
        }
    }

    /// Send encrypted message
    pub async fn send(&self, message: &[u8]) -> Result<()> {
        let encrypted = self.session_key.encrypt(message)?;
        self.tx.send(encrypted).map_err(|e| {
            crate::error::Error::CommunicationError(e.to_string())
        })
    }

    /// Receive and decrypt message
    pub async fn recv(&self) -> Result<Option<Vec<u8>>> {
        let mut rx = self.rx.lock().await;
        if let Some(encrypted) = rx.recv().await {
            let decrypted = self.session_key.decrypt(&encrypted)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_identity_generation() {
        let identity = DeviceIdentity::generate();
        assert!(!identity.fingerprint.is_empty());
        assert_eq!(identity.public_key.as_bytes().len(), 32);
    }

    #[test]
    fn test_dh_exchange() {
        let alice = DeviceIdentity::generate();
        let bob = DeviceIdentity::generate();

        let alice_shared = alice.dh(&bob.public_key);
        let bob_shared = bob.dh(&alice.public_key);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_session_key_encryption() {
        let key = SessionKey::from_bytes(&[1u8; 32]);
        let plaintext = b"hello world";

        let encrypted = key.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = key.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
