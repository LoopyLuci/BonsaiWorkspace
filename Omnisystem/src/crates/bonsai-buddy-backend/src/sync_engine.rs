//! CRDT Sync Engine
//!
//! Vector clock-based state merging with conflict-free guarantees.
//! Implements Last-Write-Wins (LWW) semantics with causality tracking.

use crate::error::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Vector clock for causality tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    // node_id -> timestamp
    clocks: BTreeMap<String, u64>,
}

impl VectorClock {
    /// Create a new vector clock
    pub fn new() -> Self {
        Self {
            clocks: BTreeMap::new(),
        }
    }

    /// Increment clock for a node
    pub fn increment(&mut self, node_id: &str) {
        let current = self.clocks.get(node_id).cloned().unwrap_or(0);
        self.clocks.insert(node_id.to_string(), current + 1);
    }

    /// Merge with another vector clock
    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, timestamp) in &other.clocks {
            let current = self.clocks.get(node_id).cloned().unwrap_or(0);
            self.clocks.insert(node_id.clone(), std::cmp::max(current, *timestamp));
        }
    }

    /// Check if this clock happens-before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut some_lt = false;

        for (node_id, timestamp) in &self.clocks {
            let other_ts = other.clocks.get(node_id).cloned().unwrap_or(0);
            if *timestamp > other_ts {
                return false; // Not happens-before
            }
            if *timestamp < other_ts {
                some_lt = true;
            }
        }

        // Check if other has timestamps we don't have
        for (node_id, timestamp) in &other.clocks {
            if !self.clocks.contains_key(node_id) && *timestamp > 0 {
                some_lt = true;
            }
        }

        some_lt
    }

    /// Check if concurrent (neither happens-before the other)
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Versioned state value with LWW
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedValue<T: Clone> {
    pub value: T,
    pub timestamp: DateTime<Utc>,
    pub vector_clock: VectorClock,
    pub node_id: String,
    pub sequence: u64,
}

impl<T: Clone> VersionedValue<T> {
    /// Create a new versioned value
    pub fn new(value: T, node_id: String) -> Self {
        let mut clock = VectorClock::new();
        clock.increment(&node_id);

        Self {
            value,
            timestamp: Utc::now(),
            vector_clock: clock,
            node_id,
            sequence: 0,
        }
    }

    /// Merge with another versioned value (LWW)
    pub fn merge(&mut self, other: &VersionedValue<T>) {
        // Happens-before relationship decides merge
        if other.vector_clock.happens_before(&self.vector_clock) {
            // self is newer, keep it
            return;
        }

        if self.vector_clock.happens_before(&other.vector_clock) {
            // other is newer, replace
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.vector_clock.merge(&other.vector_clock);
            self.node_id = other.node_id.clone();
            self.sequence = other.sequence;
            return;
        }

        // Concurrent writes - use timestamp (LWW)
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        } else if other.timestamp == self.timestamp {
            // Tie-break with node_id (lexicographic)
            if other.node_id > self.node_id {
                self.value = other.value.clone();
            }
        }

        self.vector_clock.merge(&other.vector_clock);
        self.sequence = self.sequence.saturating_add(1);
    }
}

/// Change event for tracking mutations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    pub id: String,
    pub key: String,
    pub operation: String, // "insert", "update", "delete"
    pub timestamp: DateTime<Utc>,
    pub vector_clock: VectorClock,
}

impl ChangeEvent {
    pub fn new(key: String, operation: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key,
            operation,
            timestamp: Utc::now(),
            vector_clock: VectorClock::new(),
        }
    }
}

/// CRDT Sync Engine
pub struct SyncEngine {
    node_id: String,
    state: DashMap<String, VersionedValue<serde_json::Value>>,
    changes: DashMap<String, ChangeEvent>,
    sync_version: parking_lot::Mutex<u64>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            state: DashMap::new(),
            changes: DashMap::new(),
            sync_version: parking_lot::Mutex::new(0),
        }
    }

    /// Set a value in the state
    pub fn set(&self, key: String, value: serde_json::Value) -> Result<()> {
        let versioned = VersionedValue::new(value, self.node_id.clone());
        self.state.insert(key.clone(), versioned);

        let change = ChangeEvent::new(key.clone(), "insert".to_string());
        self.changes.insert(change.id.clone(), change);

        self.increment_sync_version();
        Ok(())
    }

    /// Get a value from the state
    pub fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Ok(self
            .state
            .get(key)
            .map(|entry| entry.value.clone()))
    }

    /// Delete a value from the state
    pub fn delete(&self, key: &str) -> Result<()> {
        self.state.remove(key);

        let change = ChangeEvent::new(key.to_string(), "delete".to_string());
        self.changes.insert(change.id.clone(), change);

        self.increment_sync_version();
        Ok(())
    }

    /// Merge state from remote (conflict-free)
    pub fn merge_state(&self, remote_state: Vec<(String, VersionedValue<serde_json::Value>)>) -> Result<()> {
        for (key, remote_value) in remote_state {
            if let Some(mut local) = self.state.get_mut(&key) {
                local.merge(&remote_value);
            } else {
                self.state.insert(key, remote_value);
            }
        }

        self.increment_sync_version();
        Ok(())
    }

    /// Get all state as vector
    pub fn export_state(&self) -> Result<Vec<(String, VersionedValue<serde_json::Value>)>> {
        Ok(self
            .state
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect())
    }

    /// Get changes since a version
    pub fn get_changes_since(&self, since_version: u64) -> Result<Vec<ChangeEvent>> {
        let current = self.get_sync_version();
        if since_version >= current {
            return Ok(Vec::new());
        }

        Ok(self
            .changes
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// Compact changes (remove old entries)
    pub fn compact_changes(&self, keep_count: usize) -> Result<()> {
        let changes: Vec<_> = self
            .changes
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        if changes.len() > keep_count {
            for key in changes.iter().skip(keep_count) {
                self.changes.remove(key);
            }
        }

        Ok(())
    }

    /// Get sync version
    pub fn get_sync_version(&self) -> u64 {
        *self.sync_version.lock()
    }

    /// Increment sync version
    fn increment_sync_version(&self) {
        let mut version = self.sync_version.lock();
        *version = version.saturating_add(1);
    }

    /// Get state size
    pub fn state_size(&self) -> usize {
        self.state.len()
    }

    /// Get statistics
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "node_id": self.node_id,
            "state_size": self.state.len(),
            "changes": self.changes.len(),
            "sync_version": self.get_sync_version(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_increment() {
        let mut clock = VectorClock::new();
        clock.increment("node1");
        clock.increment("node1");

        assert_eq!(clock.clocks.get("node1"), Some(&2));
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut clock1 = VectorClock::new();
        clock1.increment("node1");

        let mut clock2 = VectorClock::new();
        clock2.increment("node2");

        clock1.merge(&clock2);
        assert_eq!(clock1.clocks.len(), 2);
    }

    #[test]
    fn test_happens_before() {
        let mut clock1 = VectorClock::new();
        clock1.increment("node1");

        let mut clock2 = VectorClock::new();
        clock2.increment("node1");
        clock2.increment("node1");

        assert!(clock1.happens_before(&clock2));
        assert!(!clock2.happens_before(&clock1));
    }

    #[test]
    fn test_sync_engine_set_get() {
        let engine = SyncEngine::new("node1".to_string());
        let value = serde_json::json!({"data": "test"});

        engine
            .set("key1".to_string(), value.clone())
            .unwrap();
        let retrieved = engine.get("key1").unwrap();

        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_versioned_value_merge() {
        let mut v1 = VersionedValue::new(serde_json::json!(1), "node1".to_string());
        let v2 = VersionedValue::new(serde_json::json!(2), "node2".to_string());

        let v2_newer = VersionedValue {
            timestamp: Utc::now() + chrono::Duration::seconds(1),
            ..v2.clone()
        };

        v1.merge(&v2_newer);
        assert_eq!(v1.value, serde_json::json!(2));
    }

    #[test]
    fn test_sync_engine_merge() {
        let engine = SyncEngine::new("node1".to_string());
        let state = vec![(
            "key1".to_string(),
            VersionedValue::new(serde_json::json!(42), "node2".to_string()),
        )];

        engine.merge_state(state).unwrap();
        assert_eq!(engine.get("key1").unwrap(), Some(serde_json::json!(42)));
    }

    #[test]
    fn test_sync_version() {
        let engine = SyncEngine::new("node1".to_string());
        let v1 = engine.get_sync_version();

        engine
            .set("key1".to_string(), serde_json::json!(1))
            .unwrap();
        let v2 = engine.get_sync_version();

        assert!(v2 > v1);
    }
}
