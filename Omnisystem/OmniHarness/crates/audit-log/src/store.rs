use crate::core::Core;
use crate::error::{Error, Result};
use crate::event::{TimelineFilter, UniverseEvent, UniverseSnapshot};
use std::path::Path;
use std::sync::Arc;
use tokio_rusqlite::Connection;

pub struct UniverseStore {
    conn: Arc<Connection>,
    device_id: String,
    /// Hot cache of recently inserted/looked-up events, keyed by event_id.
    cache: Core,
}

impl UniverseStore {
    pub async fn open(db_path: &Path, device_id: impl Into<String>) -> Result<Arc<Self>> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path).await.map_err(Error::from)?;
        let store = Arc::new(Self {
            conn: Arc::new(conn),
            device_id: device_id.into(),
            cache: Core::new(),
        });
        store.init_schema().await?;
        Ok(store)
    }

    async fn init_schema(&self) -> Result<()> {
        self.conn
            .call(|conn| -> rusqlite::Result<()> {
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
                )
            })
            .await
            .map_err(Error::from)
    }

    pub async fn insert_event(&self, event: &UniverseEvent) -> Result<()> {
        let event_id = event.event_id.clone();
        let timestamp_ns = event.timestamp_ns as i64;
        let category = event.category.as_str().to_string();
        let target = event.target.clone();
        let summary = event.summary.clone();
        let source_json = serde_json::to_string(&event.source)?;
        let data_json = serde_json::to_string(event)?;
        let cache_value = data_json.clone();

        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO universe_events
                     (event_id, timestamp_ns, category, target, summary, source_json, data_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![event_id, timestamp_ns, category, target, summary, source_json, data_json],
                ).map(|_| ())
            })
            .await
            .map_err(Error::from)?;

        self.cache.set(event.event_id.clone(), cache_value);
        Ok(())
    }

    pub async fn query_timeline(&self, filter: TimelineFilter) -> Result<Vec<UniverseEvent>> {
        let category = filter.category.map(|c| c.as_str().to_string());
        let target_prefix = filter.target_prefix;
        let since_ns = filter.since_ns.map(|n| n as i64).unwrap_or(0);
        let until_ns = filter.until_ns.map(|n| n as i64).unwrap_or(i64::MAX);
        let limit = filter.limit.unwrap_or(500) as i64;

        self.conn
            .call(move |conn| -> rusqlite::Result<Vec<UniverseEvent>> {
                let mut stmt = conn.prepare(
                    "SELECT data_json FROM universe_events
                     WHERE timestamp_ns >= ?1
                       AND timestamp_ns <= ?2
                       AND (?3 IS NULL OR category = ?3)
                       AND (?4 IS NULL OR target LIKE ?4)
                     ORDER BY timestamp_ns DESC
                     LIMIT ?5"
                )?;

                let target_pattern = target_prefix.map(|p| format!("{}%", p));
                let rows = stmt.query_map(
                    rusqlite::params![since_ns, until_ns, category, target_pattern, limit],
                    |row| row.get::<_, String>(0),
                )?;

                let mut events = Vec::new();
                for row in rows {
                    let json = row?;
                    if let Ok(event) = serde_json::from_str::<UniverseEvent>(&json) {
                        events.push(event);
                    }
                }
                Ok(events)
            })
            .await
            .map_err(Error::from)
    }

    pub async fn get_event(&self, event_id: &str) -> Result<Option<UniverseEvent>> {
        if let Some(json) = self.cache.get(event_id) {
            if let Ok(event) = serde_json::from_str(&json) {
                return Ok(Some(event));
            }
        }

        let event_id_owned = event_id.to_string();
        let json = self.conn
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT data_json FROM universe_events WHERE event_id = ?1",
                    rusqlite::params![event_id_owned],
                    |row| row.get::<_, String>(0),
                );
                match result {
                    Ok(json) => Ok(Some(json)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            })
            .await
            .map_err(Error::from)?;

        match json {
            Some(json) => {
                self.cache.set(event_id.to_string(), json.clone());
                Ok(serde_json::from_str(&json).ok())
            }
            None => Ok(None),
        }
    }

    pub async fn insert_snapshot(&self, snap: &UniverseSnapshot) -> Result<()> {
        let snapshot_id = snap.snapshot_id.clone();
        let timestamp_ns = snap.timestamp_ns as i64;
        let label = snap.label.clone();
        let data_json = serde_json::to_string(snap)?;

        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO universe_snapshots (snapshot_id, timestamp_ns, label, data_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![snapshot_id, timestamp_ns, label, data_json],
                ).map(|_| ())
            })
            .await
            .map_err(Error::from)
    }

    pub async fn list_snapshots(&self, limit: usize) -> Result<Vec<UniverseSnapshot>> {
        let limit = limit as i64;
        self.conn
            .call(move |conn| -> rusqlite::Result<Vec<UniverseSnapshot>> {
                let mut stmt = conn.prepare(
                    "SELECT data_json FROM universe_snapshots ORDER BY timestamp_ns DESC LIMIT ?1"
                )?;
                let rows = stmt.query_map(rusqlite::params![limit], |row| row.get::<_, String>(0))?;
                let mut snaps = Vec::new();
                for row in rows {
                    let json = row?;
                    if let Ok(snap) = serde_json::from_str::<UniverseSnapshot>(&json) {
                        snaps.push(snap);
                    }
                }
                Ok(snaps)
            })
            .await
            .map_err(Error::from)
    }

    pub async fn last_snapshot_before(&self, timestamp_ns: u64) -> Result<Option<UniverseSnapshot>> {
        let ts = timestamp_ns as i64;
        self.conn
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT data_json FROM universe_snapshots WHERE timestamp_ns <= ?1 ORDER BY timestamp_ns DESC LIMIT 1",
                    rusqlite::params![ts],
                    |row| row.get::<_, String>(0),
                );
                match result {
                    Ok(json) => Ok(Some(json)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            })
            .await
            .map_err(Error::from)
            .map(|json| json.and_then(|j| serde_json::from_str(&j).ok()))
    }

    pub async fn event_count(&self) -> u64 {
        self.conn
            .call(|conn| -> rusqlite::Result<u64> {
                let n: i64 = conn.query_row("SELECT COUNT(*) FROM universe_events", [], |r| r.get(0))?;
                Ok(n as u64)
            })
            .await
            .unwrap_or(0)
    }

    /// Delete events older than `cutoff_ns` in the given categories. The hot
    /// cache is cleared afterwards since it may hold entries that were just
    /// deleted from disk.
    pub async fn prune_before(&self, cutoff_ns: u64, categories: &[&str]) -> u64 {
        let cutoff = cutoff_ns as i64;
        let cats: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
        let deleted = self.conn
            .call(move |conn| -> rusqlite::Result<u64> {
                let mut deleted = 0u64;
                for cat in &cats {
                    let n = conn.execute(
                        "DELETE FROM universe_events WHERE timestamp_ns < ?1 AND category = ?2",
                        rusqlite::params![cutoff, cat],
                    )?;
                    deleted += n as u64;
                }
                Ok(deleted)
            })
            .await
            .unwrap_or(0);

        if deleted > 0 {
            self.cache.clear();
        }
        deleted
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}
