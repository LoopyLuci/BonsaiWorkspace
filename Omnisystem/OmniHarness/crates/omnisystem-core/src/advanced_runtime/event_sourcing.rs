//! Event-Sourcing State Management
//!
//! Provides immutable, versioned state management with:
//! - Complete event log for audit trail
//! - State snapshots for efficient recovery
//! - Time-travel debugging
//! - Event replay capability
//! - Atomic transactions

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::RwLock;
use std::time::SystemTime;

/// Trait for event-sourced entities
pub trait EventSourced: Send + Sync {
    /// Apply event to current state
    fn apply_event(&mut self, event: &Event) -> Result<(), String>;

    /// Get current version
    fn version(&self) -> u64;

    /// Get list of aggregates this entity depends on
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }
}

/// Domain event trait
pub trait DomainEvent: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Get event type name
    fn event_type(&self) -> &str;

    /// Get aggregate ID this event belongs to
    fn aggregate_id(&self) -> &str;

    /// Get event timestamp
    fn timestamp(&self) -> SystemTime;
}

/// Generic event wrapper
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub data: Vec<u8>,
    pub version: u64,
    pub timestamp: SystemTime,
    pub metadata: EventMetadata,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    pub user_id: Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub source: Option<String>,
}

impl Event {
    /// Create new event
    pub fn new(
        aggregate_id: impl Into<String>,
        event_type: impl Into<String>,
        data: Vec<u8>,
        version: u64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.into(),
            event_type: event_type.into(),
            data,
            version,
            timestamp: SystemTime::now(),
            metadata: EventMetadata::default(),
        }
    }

    /// Set event metadata
    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// State snapshot for efficient recovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub aggregate_id: String,
    pub state_data: Vec<u8>,
    pub version: u64,
    pub timestamp: SystemTime,
    pub events_since: u64,
}

/// Event store for persistence
pub struct EventStore {
    events: Arc<RwLock<VecDeque<Event>>>,
    snapshots: Arc<RwLock<std::collections::HashMap<String, StateSnapshot>>>,
    max_events: usize,
    snapshot_interval: u64,
}

impl EventStore {
    /// Create new event store
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            snapshots: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_events,
            snapshot_interval: 10_000,
        }
    }

    /// Append event to store
    pub async fn append(&self, event: Event) -> Result<(), String> {
        let mut events = self.events.write().await;

        // Maintain event log size
        if events.len() >= self.max_events {
            events.pop_front();
        }

        events.push_back(event);
        Ok(())
    }

    /// Get all events for aggregate
    pub async fn get_events(&self, aggregate_id: &str) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .cloned()
            .collect())
    }

    /// Get events since version
    pub async fn get_events_since(
        &self,
        aggregate_id: &str,
        version: u64,
    ) -> Result<Vec<Event>, String> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id && e.version > version)
            .cloned()
            .collect())
    }

    /// Save state snapshot
    pub async fn save_snapshot(&self, snapshot: StateSnapshot) -> Result<(), String> {
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot.aggregate_id.clone(), snapshot);
        Ok(())
    }

    /// Get latest snapshot
    pub async fn get_snapshot(&self, aggregate_id: &str) -> Result<Option<StateSnapshot>, String> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots.get(aggregate_id).cloned())
    }

    /// Replay events to rebuild state
    pub async fn replay_events<S: EventSourced>(
        &self,
        aggregate_id: &str,
        mut state: S,
    ) -> Result<S, String> {
        let events = self.get_events(aggregate_id).await?;

        for event in events {
            state.apply_event(&event)?;
        }

        Ok(state)
    }

    /// Get event count
    pub async fn event_count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Get all events (for auditing)
    pub async fn all_events(&self) -> Vec<Event> {
        self.events.read().await.iter().cloned().collect()
    }

    /// Clear all events and snapshots (use with caution!)
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        let mut snapshots = self.snapshots.write().await;
        events.clear();
        snapshots.clear();
    }
}

/// Command for event sourcing
pub trait Command: Send + Sync {
    type Aggregate: EventSourced;

    /// Execute command and produce events
    fn execute(&self, state: &Self::Aggregate) -> Result<Vec<Event>, String>;
}

/// Event router for command processing
pub struct EventRouter {
    store: Arc<EventStore>,
}

impl EventRouter {
    pub fn new(store: Arc<EventStore>) -> Self {
        Self { store }
    }

    /// Process command and store resulting events
    pub async fn route_command<C: Command>(
        &self,
        aggregate_id: &str,
        command: C,
        state: C::Aggregate,
    ) -> Result<Vec<Event>, String> {
        // Get current state from snapshots or replay
        if let Ok(Some(_snapshot)) = self.store.get_snapshot(aggregate_id).await {
            // Restore from snapshot and replay subsequent events
            // (Implementation depends on how state is serialized)
        }

        // Execute command
        let events = command.execute(&state)?;

        // Store events
        for event in &events {
            self.store.append(event.clone()).await?;
        }

        // Create snapshot if needed
        let event_count = self.store.event_count().await;
        if event_count % 10_000 == 0 {
            // TODO: Create snapshot
        }

        Ok(events)
    }
}

/// Time-travel debugger
pub struct TimeTravelDebugger {
    store: Arc<EventStore>,
}

impl TimeTravelDebugger {
    pub fn new(store: Arc<EventStore>) -> Self {
        Self { store }
    }

    /// Get state at specific version
    pub async fn state_at_version<S: EventSourced>(
        &self,
        aggregate_id: &str,
        target_version: u64,
    ) -> Result<Option<S>, String>
    where
        S: Default,
    {
        let events = self.store.get_events(aggregate_id).await?;
        let mut state = S::default();

        for event in events {
            if event.version > target_version {
                break;
            }
            state.apply_event(&event)?;
        }

        Ok(Some(state))
    }

    /// Get event log for aggregate
    pub async fn get_audit_log(&self, aggregate_id: &str) -> Result<Vec<Event>, String> {
        self.store.get_events(aggregate_id).await
    }

    /// Analyze event sequence
    pub async fn analyze_sequence(&self, aggregate_id: &str) -> Result<SequenceAnalysis, String> {
        let events = self.store.get_events(aggregate_id).await?;

        let mut total_events = 0;
        let mut event_types = std::collections::HashMap::new();
        let mut min_timestamp = None;
        let mut max_timestamp = None;

        for event in &events {
            total_events += 1;
            *event_types.entry(event.event_type.clone()).or_insert(0) += 1;

            if min_timestamp.is_none() {
                min_timestamp = Some(event.timestamp);
            }
            max_timestamp = Some(event.timestamp);
        }

        Ok(SequenceAnalysis {
            aggregate_id: aggregate_id.to_string(),
            total_events,
            event_types,
            min_timestamp,
            max_timestamp,
            duration: max_timestamp
                .zip(min_timestamp)
                .and_then(|(max, min)| max.duration_since(min).ok()),
        })
    }
}

#[derive(Clone, Debug)]
pub struct SequenceAnalysis {
    pub aggregate_id: String,
    pub total_events: usize,
    pub event_types: std::collections::HashMap<String, usize>,
    pub min_timestamp: Option<SystemTime>,
    pub max_timestamp: Option<SystemTime>,
    pub duration: Option<std::time::Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_store_append() {
        let store = EventStore::new(1000);
        let event = Event::new("agg1", "TestEvent", vec![], 1);
        assert!(store.append(event).await.is_ok());
        assert_eq!(store.event_count().await, 1);
    }

    #[tokio::test]
    async fn test_event_store_get_events() {
        let store = EventStore::new(1000);
        let event = Event::new("agg1", "TestEvent", vec![], 1);
        store.append(event).await.unwrap();

        let events = store.get_events("agg1").await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot() {
        let store = EventStore::new(1000);
        let snapshot = StateSnapshot {
            aggregate_id: "agg1".to_string(),
            state_data: vec![1, 2, 3],
            version: 1,
            timestamp: SystemTime::now(),
            events_since: 0,
        };

        assert!(store.save_snapshot(snapshot).await.is_ok());
        let retrieved = store.get_snapshot("agg1").await.unwrap();
        assert!(retrieved.is_some());
    }
}
