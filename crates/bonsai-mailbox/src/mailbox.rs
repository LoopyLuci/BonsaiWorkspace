//! AgentMailbox — per-agent inbox/outbox with local and remote delivery.

use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use bonsai_transfer_crypto::identity::BonsaiIdentity;
use crate::envelope::{AgentId, MailEnvelope};
use crate::error::{MailboxError, MailboxResult};

const INBOX_CAPACITY: usize = 1024;

/// Registry entry for a locally-registered agent.
struct AgentEntry {
    #[allow(dead_code)]
    identity: Arc<BonsaiIdentity>,
    /// Channel into the agent's inbox.
    inbox_tx: mpsc::Sender<MailEnvelope>,
}

/// Shared mailbox hub. Clone cheaply; all clones share the same registry.
#[derive(Clone)]
pub struct AgentMailbox {
    agents: Arc<DashMap<AgentId, AgentEntry>>,
}

impl AgentMailbox {
    pub fn new() -> Self {
        Self { agents: Arc::new(DashMap::new()) }
    }

    /// Register an agent, returning its inbox receiver.
    pub fn register(
        &self,
        identity: Arc<BonsaiIdentity>,
    ) -> mpsc::Receiver<MailEnvelope> {
        let agent_id = identity.fingerprint().to_string();
        let (tx, rx) = mpsc::channel(INBOX_CAPACITY);
        self.agents.insert(agent_id, AgentEntry { identity, inbox_tx: tx });
        rx
    }

    /// Unregister an agent by fingerprint.
    pub fn unregister(&self, agent_id: &AgentId) {
        self.agents.remove(agent_id);
    }

    /// Send a pre-built envelope. Local delivery is synchronous (channel push).
    pub async fn deliver(&self, envelope: MailEnvelope) -> MailboxResult<()> {
        if let Some(entry) = self.agents.get(&envelope.to) {
            entry.inbox_tx.send(envelope).await
                .map_err(|_| MailboxError::Closed)?;
            Ok(())
        } else {
            Err(MailboxError::UnknownRecipient(envelope.to.clone()))
        }
    }

    /// Build + sign + deliver a message from `sender` to `recipient_id`.
    pub async fn send_to(
        &self,
        sender: &Arc<BonsaiIdentity>,
        recipient_id: &AgentId,
        topic: &str,
        payload: Vec<u8>,
    ) -> MailboxResult<()> {
        let from = sender.fingerprint().to_string();
        let signed_bytes = {
            let id = uuid::Uuid::new_v4();
            let mut v = Vec::new();
            v.extend_from_slice(id.as_bytes());
            v.extend_from_slice(from.as_bytes());
            v.extend_from_slice(recipient_id.as_bytes());
            v.extend_from_slice(topic.as_bytes());
            v.extend_from_slice(&payload);
            v
        };
        let signature = sender.sign(&signed_bytes);
        let envelope = MailEnvelope::new(from, recipient_id.clone(), topic, payload, signature);
        self.deliver(envelope).await
    }

    /// Verify the signature on an envelope against the sender's registered identity.
    pub fn verify_signature(&self, envelope: &MailEnvelope) -> MailboxResult<bool> {
        if let Some(entry) = self.agents.get(&envelope.from) {
            let signed = envelope.signed_bytes();
            Ok(entry.identity.public_key.verify(&signed, &envelope.signature).is_ok())
        } else {
            Err(MailboxError::UnknownRecipient(envelope.from.clone()))
        }
    }

    /// Returns the number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for AgentMailbox {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bonsai_transfer_crypto::identity::BonsaiIdentity;

    #[tokio::test]
    async fn local_delivery() {
        let mailbox = AgentMailbox::new();
        let alice = Arc::new(BonsaiIdentity::generate());
        let bob   = Arc::new(BonsaiIdentity::generate());

        let bob_id = bob.fingerprint().to_string();
        let mut bob_rx = mailbox.register(bob.clone());
        mailbox.register(alice.clone());

        mailbox.send_to(&alice, &bob_id, "ping", b"hello bob".to_vec()).await.unwrap();

        let env = bob_rx.recv().await.unwrap();
        assert_eq!(env.topic, "ping");
        assert_eq!(env.payload, b"hello bob");
    }

    #[tokio::test]
    async fn unknown_recipient_error() {
        let mailbox = AgentMailbox::new();
        let alice = Arc::new(BonsaiIdentity::generate());
        mailbox.register(alice.clone());

        let result = mailbox.send_to(&alice, &"nonexistent".to_string(), "x", vec![]).await;
        assert!(matches!(result, Err(MailboxError::UnknownRecipient(_))));
    }
}
