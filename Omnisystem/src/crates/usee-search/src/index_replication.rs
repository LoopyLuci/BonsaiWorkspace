use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct IndexReplica {
    pub replica_id: u32,
    pub node_id: u32,
    pub documents: usize,
    pub sync_state: SyncState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Synced,
    Syncing,
    Lagging,
}

pub struct ReplicationManager {
    replicas: Arc<DashMap<u32, IndexReplica>>,
    replication_factor: u32,
}

impl ReplicationManager {
    pub fn new(replication_factor: u32) -> Self {
        Self {
            replicas: Arc::new(DashMap::new()),
            replication_factor,
        }
    }

    pub fn create_replica(&self, node_id: u32) -> u32 {
        let replica_id = self.replicas.len() as u32;
        let replica = IndexReplica {
            replica_id,
            node_id,
            documents: 0,
            sync_state: SyncState::Syncing,
        };
        self.replicas.insert(replica_id, replica);
        replica_id
    }

    pub fn update_sync_state(&self, replica_id: u32, state: SyncState) -> bool {
        if let Some(mut replica) = self.replicas.get_mut(&replica_id) {
            replica.sync_state = state;
            true
        } else {
            false
        }
    }

    pub fn get_synced_replicas(&self) -> Vec<IndexReplica> {
        self.replicas
            .iter()
            .filter(|entry| entry.sync_state == SyncState::Synced)
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replica_creation() {
        let rm = ReplicationManager::new(3);
        let replica_id = rm.create_replica(1);
        assert_eq!(rm.replica_count(), 1);
    }

    #[test]
    fn test_sync_state() {
        let rm = ReplicationManager::new(3);
        let replica_id = rm.create_replica(1);
        assert!(rm.update_sync_state(replica_id, SyncState::Synced));
        let synced = rm.get_synced_replicas();
        assert_eq!(synced.len(), 1);
    }
}
