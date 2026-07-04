use crate::event::UniverseEvent;
use crate::store::UniverseStore;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Non-blocking event emitter. Buffers events in a channel and persists
/// them asynchronously so callers never block on disk I/O.
pub struct UniverseEmitter {
    tx: mpsc::Sender<UniverseEvent>,
}

impl UniverseEmitter {
    /// Spawn the background persistence worker and return the emitter handle.
    pub fn spawn(store: Arc<UniverseStore>, buffer: usize) -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel::<UniverseEvent>(buffer);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let id = event.event_id.clone();
                if let Err(e) = store.insert_event(&event).await {
                    warn!("UniverseEmitter: failed to persist event {}: {}", id, e);
                } else {
                    debug!("UniverseEmitter: persisted {}", id);
                }
            }
        });
        Arc::new(Self { tx })
    }

    /// Emit an event. Non-blocking; returns false if the buffer is full.
    pub fn emit(&self, event: UniverseEvent) -> bool {
        self.tx.try_send(event).is_ok()
    }

    /// Emit an event, waiting if the buffer is full.
    pub async fn emit_async(&self, event: UniverseEvent) {
        if self.tx.send(event).await.is_err() {
            warn!("UniverseEmitter: channel closed, event dropped");
        }
    }
}
