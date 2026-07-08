use async_trait::async_trait;

/// Actors that implement this trait can be snapshotted and restored.
/// The snapshot is a plain JSON value so it can be stored in any backing store.
#[async_trait]
pub trait Checkpointable: Send + Sync {
    /// Returns a serialisable snapshot of the actor's durable state.
    async fn snapshot(&self) -> serde_json::Value;

    /// Restores state from a snapshot produced by `snapshot()`.
    /// Should be called once, before the actor starts receiving messages.
    async fn restore(&mut self, snapshot: serde_json::Value) -> Result<(), String>;
}

/// A typed snapshot envelope that bundles the actor name with its payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointEnvelope {
    pub actor_name: String,
    pub version: u64,
    pub payload: serde_json::Value,
}

impl CheckpointEnvelope {
    pub fn new(actor_name: impl Into<String>, version: u64, payload: serde_json::Value) -> Self {
        Self {
            actor_name: actor_name.into(),
            version,
            payload,
        }
    }
}
