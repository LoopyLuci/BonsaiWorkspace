//! Message envelope — the unit of agent-to-agent communication.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identifier for an agent (its public key fingerprint).
pub type AgentId = String;

/// A sealed message from one agent to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailEnvelope {
    /// Unique message ID.
    pub id: Uuid,
    /// Sender's agent ID (fingerprint).
    pub from: AgentId,
    /// Recipient's agent ID.
    pub to: AgentId,
    /// Message topic / intent (e.g. "inference-request", "model-shard", "ping").
    pub topic: String,
    /// Encrypted payload bytes. Decrypted by the recipient's session key.
    pub payload: Vec<u8>,
    /// Ed25519 signature of `id || from || to || topic || payload` by the sender.
    pub signature: Vec<u8>,
    /// Unix timestamp (ms) when the envelope was created.
    pub created_at_ms: u64,
}

impl MailEnvelope {
    pub fn new(from: AgentId, to: AgentId, topic: &str, payload: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            topic: topic.to_string(),
            payload,
            signature,
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Bytes that were signed: `id || from || to || topic || payload`.
    pub fn signed_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(self.id.as_bytes());
        v.extend_from_slice(self.from.as_bytes());
        v.extend_from_slice(self.to.as_bytes());
        v.extend_from_slice(self.topic.as_bytes());
        v.extend_from_slice(&self.payload);
        v
    }
}
