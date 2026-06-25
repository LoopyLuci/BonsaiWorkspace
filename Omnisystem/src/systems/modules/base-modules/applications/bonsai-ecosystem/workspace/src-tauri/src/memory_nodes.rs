//! Activity-level memory nodes — a lightweight event log for every user interaction.
//!
//! Unlike `memory_store.rs` (which holds curated long-term/session/working memory),
//! this module records raw activity events: chat turns, tool calls, code edits,
//! terminal commands.  The EternalWorkshop daemon reads these nodes nightly,
//! consolidates them via the DreamAgent model, and updates BONSAI.md.
//!
//! Schema is intentionally simple — no embeddings required at insert time.
//! The daemon can backfill embeddings during the consolidation cycle.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ── Node type ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Chat,
    ToolCall,
    CodeEdit,
    Terminal,
    SurvivalFix,
    ModelReload,
    TrainingCycle,
    Custom(String),
}

impl NodeType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Chat => "chat",
            Self::ToolCall => "tool_call",
            Self::CodeEdit => "code_edit",
            Self::Terminal => "terminal",
            Self::SurvivalFix => "survival_fix",
            Self::ModelReload => "model_reload",
            Self::TrainingCycle => "training_cycle",
            Self::Custom(s) => s.as_str(),
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "chat" => Self::Chat,
            "tool_call" => Self::ToolCall,
            "code_edit" => Self::CodeEdit,
            "terminal" => Self::Terminal,
            "survival_fix" => Self::SurvivalFix,
            "model_reload" => Self::ModelReload,
            "training_cycle" => Self::TrainingCycle,
            other => Self::Custom(other.to_string()),
        }
    }
}

// ── Node ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub timestamp_ms: i64,
    pub node_type: NodeType,
    /// "user" | "assistant" | "tool" | "system"
    pub source: String,
    pub content: String,
    /// Comma-separated in DB; Vec<String> in Rust.
    pub tags: Vec<String>,
    /// Base64-encoded f32 vector, stored only if embeddings are enabled.
    pub embedding: Option<Vec<u8>>,
    /// True once the daemon has consolidated this node.
    pub consolidated: bool,
}

impl MemoryNode {
    pub fn new(node_type: NodeType, source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp_ms: now_ms(),
            node_type,
            source: source.into(),
            content: content.into(),
            tags: Vec::new(),
            embedding: None,
            consolidated: false,
        }
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }
}

// ── Store ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MemoryNodeStore {
    pool: SqlitePool,
}

impl MemoryNodeStore {
    pub async fn open(db_path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&format!("sqlite://{}?mode=rwc", db_path))
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub async fn open_in_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), sqlx::Error> {
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
            );
            CREATE INDEX IF NOT EXISTS idx_mn_timestamp ON memory_nodes(timestamp_ms);
            CREATE INDEX IF NOT EXISTS idx_mn_consolidated ON memory_nodes(consolidated);
            CREATE INDEX IF NOT EXISTS idx_mn_type ON memory_nodes(node_type);",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert(&self, node: &MemoryNode) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR IGNORE INTO memory_nodes
             (id, timestamp_ms, node_type, source, content, tags, embedding, consolidated)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&node.id)
        .bind(node.timestamp_ms)
        .bind(node.node_type.as_str())
        .bind(&node.source)
        .bind(&node.content)
        .bind(node.tags.join(","))
        .bind(&node.embedding)
        .bind(node.consolidated as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all nodes from today (midnight UTC onwards) that haven't been consolidated.
    pub async fn get_pending_nodes(&self) -> Result<Vec<MemoryNode>, sqlx::Error> {
        let midnight_ms = {
            let now_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Round down to UTC midnight
            (now_secs / 86400 * 86400) as i64 * 1000
        };
        let rows = sqlx::query(
            "SELECT id, timestamp_ms, node_type, source, content, tags, embedding, consolidated
             FROM memory_nodes
             WHERE timestamp_ms >= ? AND consolidated = 0
             ORDER BY timestamp_ms ASC
             LIMIT 2000",
        )
        .bind(midnight_ms)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryNode {
                id: r.get("id"),
                timestamp_ms: r.get("timestamp_ms"),
                node_type: NodeType::from_str(r.get::<&str, _>("node_type")),
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
                embedding: r.get("embedding"),
                consolidated: r.get::<i64, _>("consolidated") != 0,
            })
            .collect())
    }

    /// Mark a batch of nodes as consolidated.
    pub async fn mark_consolidated(&self, ids: &[String]) -> Result<(), sqlx::Error> {
        for id in ids {
            sqlx::query("UPDATE memory_nodes SET consolidated = 1 WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// Search by tag (partial match).
    pub async fn search_by_tag(&self, tag: &str) -> Result<Vec<MemoryNode>, sqlx::Error> {
        let pattern = format!("%{}%", tag);
        let rows = sqlx::query(
            "SELECT id, timestamp_ms, node_type, source, content, tags, embedding, consolidated
             FROM memory_nodes WHERE tags LIKE ? ORDER BY timestamp_ms DESC LIMIT 500",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryNode {
                id: r.get("id"),
                timestamp_ms: r.get("timestamp_ms"),
                node_type: NodeType::from_str(r.get::<&str, _>("node_type")),
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
                embedding: r.get("embedding"),
                consolidated: r.get::<i64, _>("consolidated") != 0,
            })
            .collect())
    }

    /// Recent N nodes regardless of consolidation status.
    pub async fn recent(&self, limit: i64) -> Result<Vec<MemoryNode>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, timestamp_ms, node_type, source, content, tags, embedding, consolidated
             FROM memory_nodes ORDER BY timestamp_ms DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryNode {
                id: r.get("id"),
                timestamp_ms: r.get("timestamp_ms"),
                node_type: NodeType::from_str(r.get::<&str, _>("node_type")),
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
                embedding: r.get("embedding"),
                consolidated: r.get::<i64, _>("consolidated") != 0,
            })
            .collect())
    }

    /// Count pending (unconsolidated) nodes.
    pub async fn pending_count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM memory_nodes WHERE consolidated = 0")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("n"))
    }
}
