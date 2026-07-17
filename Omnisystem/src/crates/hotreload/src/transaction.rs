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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_marks_committed() {
        let mut tx = AtomicTransaction::new();
        assert!(!tx.is_committed());
        tx.commit().unwrap();
        assert!(tx.is_committed());
    }

    #[test]
    fn test_rollback_restores_snapshotted_state() {
        let state = Arc::new(RwLock::new(10u32));
        let saved = *state.read();

        let mut tx = AtomicTransaction::new();
        tx.add_snapshot(StateSnapshot::new(state.clone(), saved));

        *state.write() = 999;
        assert_eq!(*state.read(), 999);

        tx.rollback();
        assert_eq!(*state.read(), 10);
    }

    #[test]
    fn test_rollback_restores_multiple_snapshots() {
        let a = Arc::new(RwLock::new("a".to_string()));
        let b = Arc::new(RwLock::new(vec![1, 2, 3]));

        let mut tx = AtomicTransaction::new();
        tx.add_snapshot(StateSnapshot::new(a.clone(), a.read().clone()));
        tx.add_snapshot(StateSnapshot::new(b.clone(), b.read().clone()));

        *a.write() = "mutated".to_string();
        b.write().push(4);

        tx.rollback();

        assert_eq!(*a.read(), "a");
        assert_eq!(*b.read(), vec![1, 2, 3]);
    }
}
