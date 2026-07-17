use crate::{DomainEvent, EventStore, Snapshot, EventProjection, ReplayLog, EventSourcingError, EventSourcingResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct EventSourcingEngine {
    events: Arc<DashMap<Uuid, DomainEvent>>,
    event_stores: Arc<DashMap<Uuid, EventStore>>,
    snapshots: Arc<DashMap<Uuid, Snapshot>>,
    projections: Arc<DashMap<Uuid, EventProjection>>,
    replays: Arc<DashMap<Uuid, ReplayLog>>,
}

impl EventSourcingEngine {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            event_stores: Arc::new(DashMap::new()),
            snapshots: Arc::new(DashMap::new()),
            projections: Arc::new(DashMap::new()),
            replays: Arc::new(DashMap::new()),
        }
    }

    pub async fn append_event(&self, aggregate_id: Uuid, event_type: &str, data: &[u8]) -> EventSourcingResult<DomainEvent> {
        // Version must monotonically increase per-aggregate so that
        // ordering/replay by version range actually means something; a
        // fixed version of 1 for every event would make every event for
        // an aggregate collide at version 1.
        let next_version = self
            .events
            .iter()
            .filter(|e| e.value().aggregate_id == aggregate_id)
            .map(|e| e.value().version)
            .max()
            .map(|v| v + 1)
            .unwrap_or(1);

        let event = DomainEvent {
            event_id: Uuid::new_v4(),
            aggregate_id,
            event_type: event_type.to_string(),
            version: next_version,
            timestamp: Utc::now(),
            data: data.to_vec(),
        };

        self.events.insert(event.event_id, event.clone());
        Ok(event)
    }

    /// Fetch all events for an aggregate, ordered by version, so a
    /// consumer can actually replay/fold them into current state (the
    /// original `replay_events` only produces a summary log, not the
    /// events themselves).
    pub async fn get_aggregate_events(&self, aggregate_id: Uuid) -> Vec<DomainEvent> {
        let mut events: Vec<DomainEvent> = self
            .events
            .iter()
            .filter(|e| e.value().aggregate_id == aggregate_id)
            .map(|e| e.value().clone())
            .collect();
        events.sort_by_key(|e| e.version);
        events
    }

    pub async fn create_event_store(&self, aggregate_type: &str) -> EventSourcingResult<EventStore> {
        let store = EventStore {
            store_id: Uuid::new_v4(),
            aggregate_type: aggregate_type.to_string(),
            event_count: 0,
            last_event_time: Utc::now(),
        };

        self.event_stores.insert(store.store_id, store.clone());
        Ok(store)
    }

    pub async fn create_snapshot(&self, aggregate_id: Uuid, version: u32, state: &[u8]) -> EventSourcingResult<Snapshot> {
        let snapshot = Snapshot {
            snapshot_id: Uuid::new_v4(),
            aggregate_id,
            version,
            state: state.to_vec(),
            created_at: Utc::now(),
        };

        self.snapshots.insert(snapshot.snapshot_id, snapshot.clone());
        Ok(snapshot)
    }

    pub async fn get_snapshot(&self, aggregate_id: Uuid) -> EventSourcingResult<Option<Snapshot>> {
        for entry in self.snapshots.iter() {
            if entry.value().aggregate_id == aggregate_id {
                return Ok(Some(entry.value().clone()));
            }
        }

        Ok(None)
    }

    pub async fn project_state(&self, aggregate_id: Uuid, state: &[u8], version: u32) -> EventSourcingResult<EventProjection> {
        let projection = EventProjection {
            projection_id: Uuid::new_v4(),
            aggregate_id,
            projected_state: state.to_vec(),
            version,
            updated_at: Utc::now(),
        };

        self.projections.insert(projection.projection_id, projection.clone());
        Ok(projection)
    }

    pub async fn replay_events(&self, aggregate_id: Uuid, start_version: u32, end_version: u32) -> EventSourcingResult<ReplayLog> {
        if start_version > end_version {
            return Err(EventSourcingError::InvalidVersionRange { start: start_version, end: end_version });
        }

        let mut replayed = 0;

        for entry in self.events.iter() {
            if entry.value().aggregate_id == aggregate_id 
                && entry.value().version >= start_version 
                && entry.value().version <= end_version {
                replayed += 1;
            }
        }

        let log = ReplayLog {
            replay_id: Uuid::new_v4(),
            aggregate_id,
            start_version,
            end_version,
            events_replayed: replayed,
            completed_at: Utc::now(),
        };

        self.replays.insert(log.replay_id, log.clone());
        Ok(log)
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for EventSourcingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_event() {
        let engine = EventSourcingEngine::new();
        let aggregate_id = Uuid::new_v4();

        let event = engine.append_event(aggregate_id, "UserCreated", b"user_data").await.unwrap();
        assert_eq!(event.event_type, "UserCreated");
        assert_eq!(engine.event_count(), 1);
    }

    #[tokio::test]
    async fn test_create_event_store() {
        let engine = EventSourcingEngine::new();
        let store = engine.create_event_store("User").await.unwrap();

        assert_eq!(store.aggregate_type, "User");
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let engine = EventSourcingEngine::new();
        let aggregate_id = Uuid::new_v4();

        let snapshot = engine.create_snapshot(aggregate_id, 1, b"state").await.unwrap();
        assert_eq!(snapshot.version, 1);
    }

    #[tokio::test]
    async fn test_replay_events() {
        let engine = EventSourcingEngine::new();
        let aggregate_id = Uuid::new_v4();

        engine.append_event(aggregate_id, "EventType1", b"data1").await.unwrap();
        engine.append_event(aggregate_id, "EventType2", b"data2").await.unwrap();

        let log = engine.replay_events(aggregate_id, 1, 2).await.unwrap();
        assert_eq!(log.events_replayed, 2);
    }

    #[tokio::test]
    async fn test_replay_events_rejects_invalid_range() {
        let engine = EventSourcingEngine::new();
        let result = engine.replay_events(Uuid::new_v4(), 5, 1).await;
        assert!(matches!(result, Err(EventSourcingError::InvalidVersionRange { start: 5, end: 1 })));
    }

    #[tokio::test]
    async fn test_append_event_versions_increment_per_aggregate() {
        let engine = EventSourcingEngine::new();
        let aggregate_a = Uuid::new_v4();
        let aggregate_b = Uuid::new_v4();

        let a1 = engine.append_event(aggregate_a, "Created", b"a").await.unwrap();
        let a2 = engine.append_event(aggregate_a, "Updated", b"a").await.unwrap();
        let a3 = engine.append_event(aggregate_a, "Updated", b"a").await.unwrap();
        // A different aggregate's version sequence must be independent.
        let b1 = engine.append_event(aggregate_b, "Created", b"b").await.unwrap();

        assert_eq!(a1.version, 1);
        assert_eq!(a2.version, 2);
        assert_eq!(a3.version, 3);
        assert_eq!(b1.version, 1);
    }

    #[tokio::test]
    async fn test_get_aggregate_events_ordered_by_version() {
        let engine = EventSourcingEngine::new();
        let aggregate_id = Uuid::new_v4();

        engine.append_event(aggregate_id, "Created", b"1").await.unwrap();
        engine.append_event(aggregate_id, "Updated", b"2").await.unwrap();
        engine.append_event(aggregate_id, "Deleted", b"3").await.unwrap();

        let events = engine.get_aggregate_events(aggregate_id).await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "Created");
        assert_eq!(events[1].event_type, "Updated");
        assert_eq!(events[2].event_type, "Deleted");
        assert!(events.windows(2).all(|w| w[0].version < w[1].version));
    }
}
