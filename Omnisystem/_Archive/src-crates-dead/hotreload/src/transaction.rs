use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;

pub trait Snapshot: Send + Sync {
    fn rollback(&self);
}

pub struct AtomicTransaction {
    snapshots: Vec<Arc<dyn Snapshot>>,
    committed: bool,
}

impl AtomicTransaction {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            committed: false,
        }
    }

    pub fn add_snapshot(&mut self, snapshot: Arc<dyn Snapshot>) {
        self.snapshots.push(snapshot);
    }

    pub fn commit(&mut self) -> Result<()> {
        self.committed = true;
        Ok(())
    }

    pub fn rollback(&self) {
        for snapshot in &self.snapshots {
            snapshot.rollback();
        }
    }

    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl Default for AtomicTransaction {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StateSnapshot<T: Clone + Send + Sync> {
    state: Arc<RwLock<T>>,
    saved: T,
}

impl<T: Clone + Send + Sync + 'static> StateSnapshot<T> {
    pub fn new(state: Arc<RwLock<T>>, saved: T) -> Arc<Self> {
        Arc::new(Self { state, saved })
    }
}

impl<T: Clone + Send + Sync + 'static> Snapshot for StateSnapshot<T> {
    fn rollback(&self) {
        let mut guard = self.state.write();
        *guard = self.saved.clone();
    }
}
