use automerge::transaction::Transactable;
use automerge::{sync, AutoCommit, Change};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CollaborativeDoc {
    doc: Arc<Mutex<AutoCommit>>,
    sync_state: Arc<Mutex<sync::State>>,
}

impl CollaborativeDoc {
    pub fn new() -> Self {
        let mut doc = AutoCommit::new();
        let _ = doc.put_object(automerge::ROOT, "content", automerge::ObjType::Text);
        Self {
            doc: Arc::new(Mutex::new(doc)),
            sync_state: Arc::new(Mutex::new(sync::State::new())),
        }
    }

    pub async fn apply_change(&self, change: Change) -> Result<(), String> {
        let mut doc = self.doc.lock().await;
        doc.apply_changes(vec![change]).map_err(|e| e.to_string())
    }

    pub async fn generate_sync_message(&self) -> Vec<u8> {
        let mut doc = self.doc.lock().await;
        let mut state = self.sync_state.lock().await;
        // Placeholder: automerge sync API varies between versions. Return an
        // empty message for now so the crate builds; proper sync will be
        // implemented when the correct automerge API is integrated.
        vec![]
    }
}
