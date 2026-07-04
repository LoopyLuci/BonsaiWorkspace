use crate::event::{TimelineFilter, UniverseEvent, UniverseSnapshot};
use std::path::Path;
use std::sync::Arc;
use tokio_rusqlite::Connection;

pub struct UniverseStore {
    conn: Arc<Connection>,
    device_id: String,
}

impl UniverseStore {
    pub async fn open(db_path: &Path, device_id: impl Into<String>) -> Result<Arc<Self>, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(db_path).await.map_err(|e| e.to_string())?;
        let store = Arc::new(Self { conn: Arc::new(conn), device_id: device_id.into() });
        store.init_schema().await?;
        Ok(store)
    }

    async fn init_schema(&self) -> Result<(), String> {
        self.conn
            .call(|conn| -> Result<(), String> {
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL;
                     PRAGMA synchronous=NORMAL;

                     CREATE TABLE IF NOT EXISTS universe_events (
                         event_id    TEXT PRIMARY KEY,
                         timestamp_ns INTEGER NOT NULL,
                         category    TEXT NOT NULL,
                         target      TEXT NOT NULL,
                         summary     TEXT NOT NULL,
                         source_json TEXT NOT NULL,
                         data_json   TEXT NOT NULL
                     );
                     CREATE INDEX IF NOT EXISTS idx_events_ts       ON universe_events (timestamp_ns DESC);
                     CREATE INDEX IF NOT EXISTS idx_events_category  ON universe_events (category);
                     CREATE INDEX IF NOT EXISTS idx_events_target    ON universe_events (target);

                     CREATE TABLE IF NOT EXISTS universe_snapshots (
                         snapshot_id TEXT PRIMARY KEY,
                         timestamp_ns INTEGER NOT NULL,
                         label       TEXT,
                         data_json   TEXT NOT NULL
                     );
                     CREATE INDEX IF NOT EXISTS idx_snaps_ts ON universe_snapshots (timestamp_ns DESC);
                    ",
                ).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn insert_event(&self, event: &UniverseEvent) -> Result<(), String> {
        let event_id = event.event_id.clone();
        let timestamp_ns = event.timestamp_ns as i64;
        let category = event.category.as_str().to_string();
        let target = event.target.clone();
        let summary = event.summary.clone();
        let source_json = serde_json::to_string(&event.source).map_err(|e| e.to_string())?;
        let data_json = serde_json::to_string(event).map_err(|e| e.to_string())?;

        self.conn
            .call(move |conn| -> Result<(), String> {
                conn.execute(
                    "INSERT OR IGNORE INTO universe_events
                     (event_id, timestamp_ns, category, target, summary, source_json, data_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![event_id, timestamp_ns, category, target, summary, source_json, data_json],
                ).map(|_| ()).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn query_timeline(&self, filter: TimelineFilter) -> Result<Vec<UniverseEvent>, String> {
        let category = filter.category.map(|c| c.as_str().to_string());
        let target_prefix = filter.target_prefix;
        let since_ns = filter.since_ns.map(|n| n as i64).unwrap_or(0);
        let until_ns = filter.until_ns.map(|n| n as i64).unwrap_or(i64::MAX);
        let limit = filter.limit.unwrap_or(500) as i64;

        self.conn
            .call(move |conn| -> Result<Vec<UniverseEvent>, String> {
                let mut stmt = conn.prepare(
                    "SELECT data_json FROM universe_events
                     WHERE timestamp_ns >= ?1
                       AND timestamp_ns <= ?2
                       AND (?3 IS NULL OR category = ?3)
                       AND (?4 IS NULL OR target LIKE ?4)
                     ORDER BY timestamp_ns DESC
                     LIMIT ?5"
                ).map_err(|e| e.to_string())?;

                let target_pattern = target_prefix.map(|p| format!("{}%", p));
                let rows = stmt.query_map(
                    rusqlite::params![since_ns, until_ns, category, target_pattern, limit],
                    |row| row.get::<_, String>(0),
                ).map_err(|e| e.to_string())?;

                let mut events = Vec::new();
                for row in rows {
                    let json = row.map_err(|e| e.to_string())?;
                    if let Ok(event) = serde_json::from_str::<UniverseEvent>(&json) {
                        events.push(event);
                    }
                }
                Ok(events)
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_event(&self, event_id: &str) -> Result<Option<UniverseEvent>, String> {
        let event_id = event_id.to_string();
        self.conn
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT data_json FROM universe_events WHERE event_id = ?1",
                    rusqlite::params![event_id],
                    |row| row.get::<_, String>(0),
                );
                match result {
                    Ok(json) => Ok(serde_json::from_str(&json).ok()),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.to_string()),
                }
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn insert_snapshot(&self, snap: &UniverseSnapshot) -> Result<(), String> {
        let snapshot_id = snap.snapshot_id.clone();
        let timestamp_ns = snap.timestamp_ns as i64;
        let label = snap.label.clone();
        let data_json = serde_json::to_string(snap).map_err(|e| e.to_string())?;

        self.conn
            .call(move |conn| -> Result<(), String> {
                conn.execute(
                    "INSERT OR IGNORE INTO universe_snapshots (snapshot_id, timestamp_ns, label, data_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![snapshot_id, timestamp_ns, label, data_json],
                ).map(|_| ()).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_snapshots(&self, limit: usize) -> Result<Vec<UniverseSnapshot>, String> {
        let limit = limit as i64;
        self.conn
            .call(move |conn| -> Result<Vec<UniverseSnapshot>, String> {
                let mut stmt = conn.prepare(
                    "SELECT data_json FROM universe_snapshots ORDER BY timestamp_ns DESC LIMIT ?1"
                ).map_err(|e| e.to_string())?;
                let rows = stmt.query_map(rusqlite::params![limit], |row| row.get::<_, String>(0))
                    .map_err(|e| e.to_string())?;
                let mut snaps = Vec::new();
                for row in rows {
                    let json = row.map_err(|e| e.to_string())?;
                    if let Ok(snap) = serde_json::from_str::<UniverseSnapshot>(&json) {
                        snaps.push(snap);
                    }
                }
                Ok(snaps)
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn last_snapshot_before(&self, timestamp_ns: u64) -> Result<Option<UniverseSnapshot>, String> {
        let ts = timestamp_ns as i64;
        self.conn
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT data_json FROM universe_snapshots WHERE timestamp_ns <= ?1 ORDER BY timestamp_ns DESC LIMIT 1",
                    rusqlite::params![ts],
                    |row| row.get::<_, String>(0),
                );
                match result {
                    Ok(json) => Ok(serde_json::from_str(&json).ok()),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.to_string()),
                }
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn event_count(&self) -> u64 {
        self.conn
            .call(|conn| -> Result<u64, String> {
                let n: i64 = conn.query_row("SELECT COUNT(*) FROM universe_events", [], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                Ok(n as u64)
            })
            .await
            .unwrap_or(0)
    }

    pub async fn prune_before(&self, cutoff_ns: u64, categories: &[&str]) -> u64 {
        let cutoff = cutoff_ns as i64;
        let cats: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
        self.conn
            .call(move |conn| -> Result<u64, String> {
                let mut deleted = 0u64;
                for cat in &cats {
                    let n = conn.execute(
                        "DELETE FROM universe_events WHERE timestamp_ns < ?1 AND category = ?2",
                        rusqlite::params![cutoff, cat],
                    ).map_err(|e| e.to_string())?;
                    deleted += n as u64;
                }
                Ok(deleted)
            })
            .await
            .unwrap_or(0)
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}
