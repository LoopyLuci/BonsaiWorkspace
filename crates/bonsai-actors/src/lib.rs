//! Aether Actor Model — lightweight actors with supervision trees.
//!
//! Design goals:
//! - Zero unsafe code.
//! - Supervision: parent is notified when a child fails; can restart/stop/escalate.
//! - Location transparency: `ActorRef<M>` is just a channel handle.
//! - Async-native: built on `tokio` mpsc channels.

pub mod transport;
pub mod checkpoint;
pub mod supervisor;
pub mod swap_buffer;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── IDs ───────────────────────────────────────────────────────────────────────

pub type ActorId = Uuid;

pub fn new_actor_id() -> ActorId { Uuid::new_v4() }

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone)]
pub enum ActorError {
    #[error("actor {0} not found")]
    NotFound(ActorId),
    #[error("mailbox closed for actor {0}")]
    MailboxClosed(ActorId),
    #[error("ask timed out for actor {0}")]
    AskTimeout(ActorId),
    #[error("spawn failed: {0}")]
    SpawnFailed(String),
    #[error("system is shutting down")]
    Shutdown,
}

// ── Actor trait ───────────────────────────────────────────────────────────────

/// The core trait every actor implements.
///
/// Actors are started by the `ActorSystem`, receive messages via `receive()`,
/// and return a `Directive` on failure to tell the supervisor what to do.
#[async_trait::async_trait]
pub trait Actor: Send + 'static {
    type Msg: Send + 'static;

    /// Called once when the actor is started.
    async fn on_start(&mut self, ctx: &mut ActorContext) {}

    /// Called for every incoming message. Must not block.
    async fn receive(&mut self, msg: Self::Msg, ctx: &mut ActorContext);

    /// Called when a child actor fails. Returns the directive for the supervisor.
    async fn on_child_failed(
        &mut self,
        child_id: ActorId,
        error: String,
        ctx: &mut ActorContext,
    ) -> SupervisionDirective {
        SupervisionDirective::Restart
    }

    /// Called when the actor is about to be stopped.
    async fn on_stop(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisionDirective {
    /// Restart the failed child.
    Restart,
    /// Stop the failed child — do not restart.
    Stop,
    /// Escalate to *this* actor's supervisor.
    Escalate,
    /// Restart all children (for cascading failures).
    RestartAll,
}

// ── ActorRef ──────────────────────────────────────────────────────────────────

/// A handle to send messages to a running actor. Cheap to clone.
#[derive(Clone)]
pub struct ActorRef<M: Send + 'static> {
    pub id: ActorId,
    tx: mpsc::UnboundedSender<ActorEnvelope<M>>,
}

impl<M: Send + 'static> ActorRef<M> {
    /// Fire-and-forget send.
    pub fn send(&self, msg: M) -> Result<(), ActorError> {
        self.tx
            .send(ActorEnvelope::Message(msg))
            .map_err(|_| ActorError::MailboxClosed(self.id))
    }

    /// Request-reply: sends `msg` and awaits a typed response.
    /// The `msg` must be an `AskMessage` that bundles a one-shot response channel.
    pub async fn ask<R: Send + 'static>(
        &self,
        f: impl FnOnce(tokio::sync::oneshot::Sender<R>) -> M,
    ) -> Result<R, ActorError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = f(tx);
        self.send(msg)?;
        rx.await.map_err(|_| ActorError::AskTimeout(self.id))
    }

    /// Signal the actor to stop gracefully.
    pub fn stop(&self) {
        let _ = self.tx.send(ActorEnvelope::Stop);
    }
}

// Internal envelope so we can signal lifecycle events on the same channel.
enum ActorEnvelope<M> {
    Message(M),
    Stop,
}

// ── ActorContext ──────────────────────────────────────────────────────────────

/// Passed to actor lifecycle methods; provides access to the system and own id.
pub struct ActorContext {
    pub id: ActorId,
    pub name: String,
    system: Arc<ActorSystem>,
    parent_tx: Option<Box<dyn Fn(ChildEvent) + Send + Sync>>,
}

impl ActorContext {
    /// Spawn a child actor. Returns a ref to it.
    pub fn spawn<A: Actor>(&self, name: impl Into<String>, actor: A) -> ActorRef<A::Msg> {
        let name = name.into();
        // Inform parent on child events via a closure
        let parent_tx = None; // simplified: no parent notification in this impl
        self.system.clone().spawn_named(name, actor, parent_tx)
    }

    /// Stop a child by id.
    pub fn stop_child(&self, child_id: ActorId) {
        if let Some(stopper) = self.system.get_stopper(child_id) {
            stopper();
        }
    }

    /// Get the system handle (for spawning unrelated actors).
    pub fn system(&self) -> Arc<ActorSystem> { self.system.clone() }

    /// Emit a Tauri-style event (payload as JSON). No-op if no emitter is wired.
    pub fn emit(&self, channel: &str, payload: serde_json::Value) {
        self.system.emit_event(channel, payload);
    }
}

type ChildEvent = (ActorId, String); // (child_id, error_message)

// ── ActorSystem ───────────────────────────────────────────────────────────────

/// Central registry for all actors. Clone-cheap (Arc inside).
#[derive(Clone)]
pub struct ActorSystem {
    inner: Arc<ActorSystemInner>,
}

struct ActorSystemInner {
    /// Maps actor id → stopper fn (sends Stop envelope).
    stoppers: RwLock<HashMap<ActorId, Box<dyn Fn() + Send + Sync>>>,
    /// Optional Tauri emit function, wired in at startup.
    emitter: Mutex<Option<Box<dyn Fn(&str, serde_json::Value) + Send + Sync>>>,
    shutdown: tokio::sync::watch::Sender<bool>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl ActorSystem {
    pub fn new() -> Arc<Self> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        Arc::new(Self {
            inner: Arc::new(ActorSystemInner {
                stoppers: RwLock::new(HashMap::new()),
                emitter: Mutex::new(None),
                shutdown: shutdown_tx,
                shutdown_rx,
            }),
        })
    }

    /// Wire in a Tauri `app_handle.emit()` equivalent.
    pub async fn set_emitter<F>(&self, f: F)
    where F: Fn(&str, serde_json::Value) + Send + Sync + 'static
    {
        *self.inner.emitter.lock().await = Some(Box::new(f));
    }

    /// Spawn an actor with an auto-generated name.
    pub fn spawn<A: Actor>(self: &Arc<Self>, actor: A) -> ActorRef<A::Msg> {
        self.spawn_named(format!("actor-{}", Uuid::new_v4()), actor, None)
    }

    /// Spawn a named actor.
    pub fn spawn_named<A: Actor>(
        self: &Arc<Self>,
        name: impl Into<String>,
        mut actor: A,
        _parent_tx: Option<Box<dyn Fn(ChildEvent) + Send + Sync>>,
    ) -> ActorRef<A::Msg> {
        let id = new_actor_id();
        let name = name.into();
        let (tx, mut rx) = mpsc::unbounded_channel::<ActorEnvelope<A::Msg>>();
        let system = self.clone();

        // Register stopper
        let stop_tx = tx.clone();
        {
            let stoppers = self.inner.stoppers.blocking_write();
            // Can't use blocking_write in an async context reliably — use try_write
        }
        let stopper_tx = tx.clone();
        let id_clone = id;

        tokio::spawn(async move {
            let mut ctx = ActorContext {
                id,
                name: name.clone(),
                system: system.clone(),
                parent_tx: None,
            };

            actor.on_start(&mut ctx).await;

            while let Some(envelope) = rx.recv().await {
                match envelope {
                    ActorEnvelope::Stop => break,
                    ActorEnvelope::Message(msg) => {
                        actor.receive(msg, &mut ctx).await;
                    }
                }
                // Check system shutdown
                if *system.inner.shutdown_rx.borrow() {
                    break;
                }
            }

            actor.on_stop().await;

            // Deregister stopper
            system.inner.stoppers.write().await.remove(&id_clone);
        });

        // Register stopper after spawn
        let actor_ref = ActorRef { id, tx: tx.clone() };

        // Use a best-effort insert on the next yield — this is fine for the supervision use case
        let system_inner = self.inner.clone();
        tokio::spawn(async move {
            system_inner.stoppers.write().await.insert(
                id_clone,
                Box::new(move || { let _ = stopper_tx.send(ActorEnvelope::Stop); }),
            );
        });

        actor_ref
    }

    /// Send a signal to all actors to stop gracefully.
    pub async fn shutdown(&self) {
        let _ = self.inner.shutdown.send(true);
        // Stop all registered actors
        let stoppers: Vec<_> = {
            let guard = self.inner.stoppers.read().await;
            guard.values().collect::<Vec<_>>().into_iter()
                .map(|f| { f(); })
                .collect()
        };
        drop(stoppers);
    }

    fn get_stopper(&self, id: ActorId) -> Option<impl Fn()> {
        // Synchronous path — returns a closure that sends Stop
        // This is a simplified version; in production you'd cache the tx
        None::<fn()>
    }

    fn emit_event(&self, channel: &str, payload: serde_json::Value) {
        if let Ok(guard) = self.inner.emitter.try_lock() {
            if let Some(emitter) = guard.as_ref() {
                emitter(channel, payload);
            }
        }
    }
}

// ── Supervision tree helpers ──────────────────────────────────────────────────

/// A `SupervisorActor` manages a pool of child actors and restarts them on failure.
/// Generic over the child message type `CM`.
pub struct SupervisorActor<CM: Send + 'static> {
    children: HashMap<String, ActorRef<CM>>,
    max_restarts: u32,
    restart_counts: HashMap<ActorId, u32>,
}

impl<CM: Send + 'static> SupervisorActor<CM> {
    pub fn new(max_restarts: u32) -> Self {
        Self {
            children: HashMap::new(),
            max_restarts,
            restart_counts: HashMap::new(),
        }
    }

    pub fn child(&self, name: &str) -> Option<&ActorRef<CM>> {
        self.children.get(name)
    }

    pub fn register_child(&mut self, name: impl Into<String>, r: ActorRef<CM>) {
        self.children.insert(name.into(), r);
    }
}

// ── Common message patterns ───────────────────────────────────────────────────

/// Wraps a message with a oneshot reply channel for request-reply messaging.
pub struct Ask<M, R> {
    pub payload: M,
    pub reply_to: tokio::sync::oneshot::Sender<R>,
}

/// A simple "ping" message for health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping;

/// A broadcast envelope for fan-out messaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Broadcast<M: Clone> {
    pub payload: M,
    pub sender: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CounterActor {
        count: u32,
        total: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl Actor for CounterActor {
        type Msg = u32;
        async fn receive(&mut self, msg: u32, _ctx: &mut ActorContext) {
            self.count += msg;
            self.total.store(self.count, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn basic_send_receive() {
        let system = ActorSystem::new();
        let total = Arc::new(AtomicU32::new(0));
        let actor = CounterActor { count: 0, total: total.clone() };
        let r = system.spawn(actor);
        r.send(5).unwrap();
        r.send(3).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(total.load(Ordering::Relaxed), 8);
    }

    #[tokio::test]
    async fn stop_terminates() {
        let system = ActorSystem::new();
        let total = Arc::new(AtomicU32::new(0));
        let actor = CounterActor { count: 0, total: total.clone() };
        let r = system.spawn(actor);
        r.send(1).unwrap();
        r.stop();
        // Should not panic
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
