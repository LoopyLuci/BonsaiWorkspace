//! Core actor trait defining the Aether actor model
//!
//! The actor model is based on the Erlang/Akka pattern with async/await and message passing.
//! Each actor:
//! - Runs in its own task
//! - Receives messages through a channel
//! - Processes messages sequentially
//! - Can spawn new actors
//! - Can maintain mutable state

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Unique actor identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub Uuid);

impl ActorId {
    pub fn new() -> Self {
        ActorId(Uuid::new_v4())
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Actor address for sending messages (thread-safe sender)
pub struct ActorRef<M> {
    tx: mpsc::UnboundedSender<M>,
    id: ActorId,
}

impl<M> Clone for ActorRef<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            id: self.id,
        }
    }
}

impl<M> ActorRef<M> {
    pub fn new(tx: mpsc::UnboundedSender<M>, id: ActorId) -> Self {
        Self { tx, id }
    }

    pub fn id(&self) -> ActorId {
        self.id
    }

    /// Send a message to this actor
    pub fn send(&self, msg: M) -> Result<(), M> {
        self.tx.send(msg).map_err(|e| e.0)
    }

    /// Try to send a message (non-blocking, returns error if channel is closed)
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// State snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub actor_id: ActorId,
    pub actor_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: serde_json::Value,
}

impl Snapshot {
    pub fn new(
        actor_id: ActorId,
        actor_type: String,
        state: serde_json::Value,
    ) -> Self {
        Self {
            actor_id,
            actor_type,
            timestamp: chrono::Utc::now(),
            state,
        }
    }

    /// Compute content-addressable hash for CAS storage
    pub fn cas_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        blake3::hash(&bytes).to_hex().to_string()
    }
}

/// Supervision event for monitoring actor health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisionEvent {
    ActorStarted { id: ActorId, actor_type: String },
    ActorStopped { id: ActorId, reason: String },
    ActorError { id: ActorId, error: String },
    ActorRecovered { id: ActorId, reason: String },
    SnapshotCreated { id: ActorId, hash: String },
}

/// Core actor trait
#[async_trait]
pub trait Actor: Send + Sync {
    /// Message type for this actor
    type Message: Send;

    /// Get this actor's ID
    fn id(&self) -> ActorId;

    /// Handle a single message
    /// Return true to continue processing, false to stop
    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String>;

    /// Create a snapshot of the current state
    async fn snapshot(&self) -> Result<Snapshot, String>;

    /// Restore from a snapshot
    async fn restore(&mut self, snapshot: Snapshot) -> Result<(), String>;

    /// Called when the actor is about to be stopped
    async fn on_stop(&mut self) {}

    /// Get actor type name
    fn actor_type(&self) -> &'static str;
}

/// Actor handle for spawning and supervising actors
pub struct ActorHandle<M> {
    pub task: tokio::task::JoinHandle<()>,
    pub ref_: ActorRef<M>,
}

impl<M: Send + 'static> ActorHandle<M> {
    pub fn new(task: tokio::task::JoinHandle<()>, ref_: ActorRef<M>) -> Self {
        Self { task, ref_ }
    }

    /// Wait for actor to finish
    pub async fn join(self) {
        let _ = self.task.await;
    }

    /// Get reference to actor
    pub fn actor_ref(&self) -> ActorRef<M> {
        self.ref_.clone()
    }
}

/// Spawn an actor and return its reference
pub fn spawn_actor<A>(mut actor: A) -> ActorHandle<A::Message>
where
    A: Actor + 'static,
{
    let actor_id = actor.id();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let ref_ = ActorRef::new(tx, actor_id);

    let task = tokio::spawn(async move {
        log::info!("[Actor {}] Started {}", actor_id.0, actor.actor_type());

        while let Some(msg) = rx.recv().await {
            match actor.handle(msg).await {
                Ok(false) => {
                    log::info!("[Actor {}] Stopping gracefully", actor_id.0);
                    break;
                }
                Ok(true) => {
                    // Continue processing
                }
                Err(e) => {
                    log::error!("[Actor {}] Error handling message: {}", actor_id.0, e);
                    // Continue processing to allow recovery
                }
            }
        }

        actor.on_stop().await;
        log::info!("[Actor {}] Stopped", actor_id.0);
    });

    ActorHandle::new(task, ref_)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestActor {
        id: ActorId,
        count: i32,
    }

    #[derive(Debug)]
    enum TestMessage {
        Increment,
        Stop,
    }

    impl TestActor {
        fn new() -> Self {
            Self {
                id: ActorId::new(),
                count: 0,
            }
        }
    }

    #[async_trait]
    impl Actor for TestActor {
        type Message = TestMessage;

        fn id(&self) -> ActorId {
            self.id
        }

        async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
            match msg {
                TestMessage::Increment => {
                    self.count += 1;
                    Ok(true)
                }
                TestMessage::Stop => Ok(false),
            }
        }

        async fn snapshot(&self) -> Result<Snapshot, String> {
            Ok(Snapshot::new(
                self.id,
                "TestActor".to_string(),
                serde_json::json!({"count": self.count}),
            ))
        }

        async fn restore(&mut self, snapshot: Snapshot) -> Result<(), String> {
            if let Some(count) = snapshot.state.get("count").and_then(|v| v.as_i64()) {
                self.count = count as i32;
                Ok(())
            } else {
                Err("Invalid snapshot format".to_string())
            }
        }

        fn actor_type(&self) -> &'static str {
            "TestActor"
        }
    }

    #[tokio::test]
    async fn test_actor_message_handling() {
        let actor = TestActor::new();
        let handle = spawn_actor(actor);
        let ref_ = handle.actor_ref();

        ref_.send(TestMessage::Increment).unwrap();
        ref_.send(TestMessage::Increment).unwrap();
        ref_.send(TestMessage::Stop).unwrap();

        handle.join().await;
    }

    #[tokio::test]
    async fn test_snapshot_cas_hash() {
        let snapshot = Snapshot::new(
            ActorId::new(),
            "TestActor".to_string(),
            serde_json::json!({"data": "test"}),
        );

        let hash = snapshot.cas_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // BLAKE3 hex output is 64 chars
    }
}
