//! WebSocket connection registry.
//!
//! `WsRouter` tracks every authenticated WebSocket client. It provides:
//!   - `register()` → allocates a client slot and returns an `mpsc` receiver
//!     the handler task drains to send frames to the client.
//!   - `unregister(id)` → removes the client when the connection drops.
//!   - `broadcast(msg)` → fan-out a message to every connected client.
//!   - `send_to(id, msg)` → unicast to one client (VSCode extension, Android, etc.)

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use axum::extract::ws::Message;
use tokio::sync::mpsc;

pub type ClientId = u64;

#[derive(Clone, Debug, Default)]
pub struct WsRouter {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    next_id: ClientId,
    clients: HashMap<ClientId, mpsc::UnboundedSender<Message>>,
}

impl WsRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new client; returns `(id, rx)`.  The handler task should
    /// forward every `Message` it receives from `rx` to the WebSocket sink.
    pub fn register(&self) -> (ClientId, mpsc::UnboundedReceiver<Message>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut g = self.inner.lock().unwrap();
        g.next_id += 1;
        let id = g.next_id;
        g.clients.insert(id, tx);
        (id, rx)
    }

    /// Remove a client when its connection closes.
    pub fn unregister(&self, id: ClientId) {
        self.inner.lock().unwrap().clients.remove(&id);
    }

    /// Send `msg` to every connected client (fire-and-forget; dead senders are pruned).
    pub fn broadcast(&self, msg: Message) {
        let mut g = self.inner.lock().unwrap();
        let dead: Vec<ClientId> = g
            .clients
            .iter()
            .filter_map(|(id, tx)| {
                if tx.send(msg.clone()).is_err() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in dead {
            g.clients.remove(&id);
        }
    }

    /// Send `msg` to a specific client.  Returns `false` if the client is gone.
    pub fn send_to(&self, id: ClientId, msg: Message) -> bool {
        let g = self.inner.lock().unwrap();
        g.clients.get(&id).map(|tx| tx.send(msg).is_ok()).unwrap_or(false)
    }

    pub fn client_count(&self) -> usize {
        self.inner.lock().unwrap().clients.len()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ws::Message;

    #[test]
    fn register_increments_client_count() {
        let router = WsRouter::new();
        assert_eq!(router.client_count(), 0);

        let (id1, _rx1) = router.register();
        assert_eq!(router.client_count(), 1);

        let (id2, _rx2) = router.register();
        assert_eq!(router.client_count(), 2);

        // IDs are unique and monotonically increasing.
        assert_ne!(id1, id2);
        assert!(id2 > id1);
    }

    #[test]
    fn unregister_decrements_client_count() {
        let router = WsRouter::new();
        let (id, _rx) = router.register();
        assert_eq!(router.client_count(), 1);

        router.unregister(id);
        assert_eq!(router.client_count(), 0);
    }

    #[test]
    fn unregister_unknown_id_is_noop() {
        let router = WsRouter::new();
        router.unregister(9999); // must not panic
        assert_eq!(router.client_count(), 0);
    }

    #[tokio::test]
    async fn broadcast_delivers_to_all_clients() {
        let router = WsRouter::new();
        let (_id1, mut rx1) = router.register();
        let (_id2, mut rx2) = router.register();

        router.broadcast(Message::Text("hello".into()));

        let m1 = rx1.recv().await.expect("rx1 should receive");
        let m2 = rx2.recv().await.expect("rx2 should receive");

        assert_eq!(m1, Message::Text("hello".into()));
        assert_eq!(m2, Message::Text("hello".into()));
    }

    #[tokio::test]
    async fn send_to_delivers_only_to_target() {
        let router = WsRouter::new();
        let (id1, mut rx1) = router.register();
        let (_id2, mut rx2) = router.register();

        let sent = router.send_to(id1, Message::Text("targeted".into()));
        assert!(sent, "send_to should return true for a live client");

        let m1 = rx1.recv().await.expect("rx1 should receive");
        assert_eq!(m1, Message::Text("targeted".into()));

        // rx2 must NOT receive anything.
        assert!(rx2.try_recv().is_err(), "rx2 should not receive the targeted message");
    }

    #[test]
    fn send_to_missing_id_returns_false() {
        let router = WsRouter::new();
        let sent = router.send_to(42, Message::Text("nope".into()));
        assert!(!sent);
    }

    #[tokio::test]
    async fn broadcast_prunes_dead_senders() {
        let router = WsRouter::new();
        let (id, rx) = router.register();
        drop(rx); // close the receiver — sender becomes dead

        assert_eq!(router.client_count(), 1);
        // After broadcast, the dead client should be pruned.
        router.broadcast(Message::Text("prune me".into()));
        assert_eq!(router.client_count(), 0);

        // Verify the id is no longer present.
        let still_there = router.send_to(id, Message::Text("nope".into()));
        assert!(!still_there);
    }

    #[test]
    fn multiple_register_unregister_cycles_stay_consistent() {
        let router = WsRouter::new();
        let mut ids = vec![];

        for _ in 0..5 {
            let (id, _rx) = router.register();
            ids.push(id);
        }
        assert_eq!(router.client_count(), 5);

        for id in &ids[..3] {
            router.unregister(*id);
        }
        assert_eq!(router.client_count(), 2);

        router.unregister(ids[3]);
        router.unregister(ids[4]);
        assert_eq!(router.client_count(), 0);
    }
}
