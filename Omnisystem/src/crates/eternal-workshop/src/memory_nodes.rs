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
