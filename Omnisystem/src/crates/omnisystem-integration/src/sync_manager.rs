use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SyncState {
    pub system_a: String,
    pub system_b: String,
    pub last_sync: u64,
    pub sync_status: SyncStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Synced,
    Syncing,
    OutOfSync,
    Error,
}

pub struct SyncManager {
    sync_states: Arc<DashMap<String, SyncState>>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            sync_states: Arc::new(DashMap::new()),
        }
    }

    pub fn init_sync(&self, system_a: String, system_b: String) -> String {
        let sync_id = format!("sync_{}_{}", system_a, system_b);
        let state = SyncState {
            system_a,
            system_b,
            last_sync: 0,
            sync_status: SyncStatus::Syncing,
        };
        self.sync_states.insert(sync_id.clone(), state);
        sync_id
    }

    pub fn mark_synced(&self, sync_id: &str) -> bool {
        if let Some(mut state) = self.sync_states.get_mut(sync_id) {
            state.sync_status = SyncStatus::Synced;
            state.last_sync = 0;
            true
        } else {
            false
        }
    }

    pub fn get_sync_state(&self, sync_id: &str) -> Option<SyncState> {
        self.sync_states.get(sync_id).map(|s| s.clone())
    }

    pub fn is_synced(&self, sync_id: &str) -> bool {
        self.sync_states
            .get(sync_id)
            .map(|s| s.sync_status == SyncStatus::Synced)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_init() {
        let sm = SyncManager::new();
        let sync_id = sm.init_sync("buddy".to_string(), "omni-bot".to_string());
        let state = sm.get_sync_state(&sync_id).unwrap();
        assert_eq!(state.sync_status, SyncStatus::Syncing);
    }

    #[test]
    fn test_mark_synced() {
        let sm = SyncManager::new();
        let sync_id = sm.init_sync("buddy".to_string(), "usee".to_string());
        assert!(sm.mark_synced(&sync_id));
        assert!(sm.is_synced(&sync_id));
    }
}
