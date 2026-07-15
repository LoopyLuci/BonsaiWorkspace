//! Minimal memory node store for the daemon.
//! Shares the same SQLite schema as `bonsai-workspace/src-tauri/src/memory_nodes.rs`.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub timestamp_ms: i64,
    pub node_type: String,
    pub source: String,
    pub content: String,
    pub tags: Vec<String>,
    pub consolidated: bool,
}

#[derive(Clone)]
pub struct MemoryNodeStore {
    pool: SqlitePool,
}

impl MemoryNodeStore {
    pub async fn open(path: &PathBuf) -> Result<Self, sqlx::Error> {
        std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new("."))).ok();
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        // Ensure table exists (main app may have already created it)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_nodes (
                id            TEXT PRIMARY KEY,
                timestamp_ms  INTEGER NOT NULL,
                node_type     TEXT    NOT NULL,
                source        TEXT    NOT NULL,
                content       TEXT    NOT NULL,
                tags          TEXT    NOT NULL DEFAULT '',
                embedding     BLOB,
                consolidated  INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    /// Get all unconsolidated nodes from the current day.
    pub async fn get_pending_nodes(&self) -> Result<Vec<MemoryNode>, sqlx::Error> {
        let midnight_ms = {
            let now_s = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            (now_s / 86400 * 86400) as i64 * 1000
        };
        let rows = sqlx::query(
            "SELECT id, timestamp_ms, node_type, source, content, tags, consolidated
             FROM memory_nodes WHERE timestamp_ms >= ? AND consolidated = 0
             ORDER BY timestamp_ms ASC LIMIT 2000",
        )
        .bind(midnight_ms)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_node).collect())
    }

    pub async fn mark_consolidated(&self, ids: &[String]) -> Result<(), sqlx::Error> {
        for id in ids {
            sqlx::query("UPDATE memory_nodes SET consolidated = 1 WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn pending_count(&self) -> Result<i64, sqlx::Error> {
        let r = sqlx::query("SELECT COUNT(*) AS n FROM memory_nodes WHERE consolidated = 0")
            .fetch_one(&self.pool)
            .await?;
        Ok(r.get::<i64, _>("n"))
    }

    /// Insert (or replace) a memory node. Normally the main Bonsai Workspace
    /// app is the one writing rows into this shared database; this exists so
    /// the daemon (and its tests/CLI) can seed and exercise the store
    /// without depending on the main app being present.
    pub async fn insert_node(&self, node: &MemoryNode) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO memory_nodes
                (id, timestamp_ms, node_type, source, content, tags, consolidated)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&node.id)
        .bind(node.timestamp_ms)
        .bind(&node.node_type)
        .bind(&node.source)
        .bind(&node.content)
        .bind(node.tags.join(","))
        .bind(node.consolidated as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_node(r: sqlx::sqlite::SqliteRow) -> MemoryNode {
    MemoryNode {
        id: r.get("id"),
        timestamp_ms: r.get("timestamp_ms"),
        node_type: r.get("node_type"),
        source: r.get("source"),
        content: r.get("content"),
        tags: {
            let raw: &str = r.get("tags");
            if raw.is_empty() {
                vec![]
            } else {
                raw.split(',').map(str::to_string).collect()
            }
        },
        consolidated: r.get::<i64, _>("consolidated") != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, timestamp_ms: i64, content: &str) -> MemoryNode {
        MemoryNode {
            id: id.to_string(),
            timestamp_ms,
            node_type: "edit".to_string(),
            source: "editor".to_string(),
            content: content.to_string(),
            tags: vec!["rust".to_string(), "test".to_string()],
            consolidated: false,
        }
    }

    async fn open_temp_store() -> (MemoryNodeStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("memory_nodes.db");
        let store = MemoryNodeStore::open(&db_path).await.unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn insert_and_fetch_pending_round_trips() {
        let (store, _dir) = open_temp_store().await;
        let now_ms = chrono::Utc::now().timestamp_millis();

        store.insert_node(&node("n1", now_ms, "fixed a bug")).await.unwrap();
        store.insert_node(&node("n2", now_ms + 1, "wrote a test")).await.unwrap();

        let pending = store.get_pending_nodes().await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].id, "n1");
        assert_eq!(pending[0].tags, vec!["rust".to_string(), "test".to_string()]);
        assert_eq!(pending[1].id, "n2");
    }

    #[tokio::test]
    async fn mark_consolidated_excludes_from_pending() {
        let (store, _dir) = open_temp_store().await;
        let now_ms = chrono::Utc::now().timestamp_millis();

        store.insert_node(&node("n1", now_ms, "a")).await.unwrap();
        store.insert_node(&node("n2", now_ms, "b")).await.unwrap();

        store.mark_consolidated(&["n1".to_string()]).await.unwrap();

        let pending = store.get_pending_nodes().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "n2");
    }

    #[tokio::test]
    async fn pending_count_matches_get_pending_nodes() {
        let (store, _dir) = open_temp_store().await;
        let now_ms = chrono::Utc::now().timestamp_millis();

        for i in 0..5 {
            store.insert_node(&node(&format!("n{i}"), now_ms, "content")).await.unwrap();
        }

        assert_eq!(store.pending_count().await.unwrap(), 5);
        store.mark_consolidated(&["n0".to_string(), "n1".to_string()]).await.unwrap();
        assert_eq!(store.pending_count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn old_nodes_before_midnight_are_excluded_from_pending() {
        let (store, _dir) = open_temp_store().await;
        let old_ms = chrono::Utc::now().timestamp_millis() - 3 * 24 * 60 * 60 * 1000;

        store.insert_node(&node("old", old_ms, "yesterday's news")).await.unwrap();

        let pending = store.get_pending_nodes().await.unwrap();
        assert!(pending.is_empty());
    }
}
