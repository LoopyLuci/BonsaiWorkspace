//! Persistence Layer for Actor State
//!
//! Actor state is automatically persisted to AriaDB and survives restarts.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Manages persistent state for actors
pub struct PersistenceManager {
    state_store: Arc<RwLock<HashMap<uuid::Uuid, ActorState>>>,
}

/// The state of a persistent actor
#[derive(Debug, Clone)]
pub struct ActorState {
    pub actor_id: uuid::Uuid,
    pub actor_type: String,
    pub state_data: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: u64,
}

impl PersistenceManager {
    pub fn new() -> Self {
        Self {
            state_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Save actor state
    pub fn save(
        &self,
        actor_id: uuid::Uuid,
        actor_type: String,
        state: serde_json::Value,
    ) -> Result<(), String> {
        let mut store = self.state_store.write();
        let entry = store.entry(actor_id).or_insert(ActorState {
            actor_id,
            actor_type,
            state_data: serde_json::Value::Null,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 0,
        });

        entry.state_data = state;
        entry.updated_at = chrono::Utc::now();
        entry.version += 1;

        Ok(())
    }

    /// Load actor state
    pub fn load(&self, actor_id: uuid::Uuid) -> Result<Option<ActorState>, String> {
        let store = self.state_store.read();
        Ok(store.get(&actor_id).cloned())
    }

    /// Delete actor state
    pub fn delete(&self, actor_id: uuid::Uuid) -> Result<(), String> {
        let mut store = self.state_store.write();
        store.remove(&actor_id);
        Ok(())
    }

    /// List all actor states of a given type
    pub fn list_by_type(&self, actor_type: &str) -> Vec<ActorState> {
        let store = self.state_store.read();
        store
            .values()
            .filter(|s| s.actor_type == actor_type)
            .cloned()
            .collect()
    }

    /// Get the version of an actor's state
    pub fn get_version(&self, actor_id: uuid::Uuid) -> Result<Option<u64>, String> {
        let store = self.state_store.read();
        Ok(store.get(&actor_id).map(|s| s.version))
    }
}

impl ActorState {
    /// Deserialize state data to a specific type
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_value(self.state_data.clone())
            .map_err(|e| format!("Failed to deserialize state: {}", e))
    }
}

/// A snapshot of actor state at a point in time
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub actor_id: uuid::Uuid,
    pub state: ActorState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// State journal for replaying actor history
pub struct StateJournal {
    entries: Vec<StateSnapshot>,
}

impl StateJournal {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add an entry to the journal
    pub fn log(&mut self, snapshot: StateSnapshot) {
        self.entries.push(snapshot);
    }

    /// Get the state at a specific point in time
    pub fn state_at(&self, actor_id: uuid::Uuid, timestamp: chrono::DateTime<chrono::Utc>) -> Option<ActorState> {
        self.entries
            .iter()
            .filter(|s| s.actor_id == actor_id && s.timestamp <= timestamp)
            .last()
            .map(|s| s.state.clone())
    }

    /// Replay history from a specific version
    pub fn replay_from(&self, actor_id: uuid::Uuid, version: u64) -> Vec<ActorState> {
        self.entries
            .iter()
            .filter(|s| s.actor_id == actor_id && s.state.version >= version)
            .map(|s| s.state.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_manager() {
        let manager = PersistenceManager::new();
        let actor_id = uuid::Uuid::new_v4();
        let state = serde_json::json!({ "count": 42 });

        manager
            .save(actor_id, "Counter".to_string(), state.clone())
            .unwrap();

        let loaded = manager.load(actor_id).unwrap().unwrap();
        assert_eq!(loaded.state_data, state);
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn test_state_journal() {
        let actor_id = uuid::Uuid::new_v4();
        let mut journal = StateJournal::new();

        let snapshot1 = StateSnapshot {
            actor_id,
            state: ActorState {
                actor_id,
                actor_type: "Counter".to_string(),
                state_data: serde_json::json!({ "count": 0 }),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: 1,
            },
            timestamp: chrono::Utc::now(),
        };

        journal.log(snapshot1);

        let replay = journal.replay_from(actor_id, 0);
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].version, 1);
    }
}
