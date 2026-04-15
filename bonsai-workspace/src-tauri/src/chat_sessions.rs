//! Chat session persistence for Bonsai Workspace.
//!
//! Stores chat sessions and their messages in the existing `bonsai.db` SQLite file.
//! Each session is linked to an optional workspace path and carries a title that
//! defaults to the first user message (truncated).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionMessage {
    pub id:         String,
    pub session_id: String,
    pub role:       String,
    pub content:    String,
    /// JSON-encoded token stats (optional; only on assistant messages).
    pub stats:      Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ChatSession {
    pub id:             String,
    pub title:          String,
    pub workspace_path: Option<String>,
    pub created_at:     i64,
    pub updated_at:     i64,
    /// Only populated when loading a specific session.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub messages:       Vec<SessionMessage>,
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct ChatSessionStore {
    pool: SqlitePool,
}

impl ChatSessionStore {
    /// Connect to (or create) the sessions tables in the existing bonsai.db.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chat_sessions (
                id             TEXT    PRIMARY KEY,
                title          TEXT    NOT NULL DEFAULT 'New chat',
                workspace_path TEXT,
                created_at     INTEGER NOT NULL,
                updated_at     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON chat_sessions(updated_at DESC);

            CREATE TABLE IF NOT EXISTS session_messages (
                id          TEXT    PRIMARY KEY,
                session_id  TEXT    NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
                role        TEXT    NOT NULL,
                content     TEXT    NOT NULL,
                stats_json  TEXT,
                created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_msgs_session ON session_messages(session_id, created_at);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    // ── Session CRUD ─────────────────────────────────────────────────────────

    pub async fn create_session(
        &self,
        title: &str,
        workspace_path: Option<&str>,
    ) -> Result<String> {
        let id  = uuid();
        let now = now_ms();
        sqlx::query(
            "INSERT INTO chat_sessions (id, title, workspace_path, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(title)
        .bind(workspace_path)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn rename_session(&self, id: &str, title: &str) -> Result<()> {
        sqlx::query("UPDATE chat_sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now_ms())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn duplicate_session(&self, id: &str, title: Option<&str>) -> Result<String> {
        let source = self.load_session(id).await?;
        let new_id = uuid();
        let now = now_ms();
        let new_title = title
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!("Copy of {}", source.title));

        sqlx::query(
            "INSERT INTO chat_sessions (id, title, workspace_path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&new_id)
        .bind(&new_title)
        .bind(source.workspace_path.as_deref())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        for message in source.messages {
            let stats_json = message.stats.as_ref().and_then(|s| serde_json::to_string(s).ok());
            sqlx::query(
                "INSERT INTO session_messages (id, session_id, role, content, stats_json, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid())
            .bind(&new_id)
            .bind(&message.role)
            .bind(&message.content)
            .bind(stats_json)
            .bind(message.created_at)
            .execute(&self.pool)
            .await?;
        }

        Ok(new_id)
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let rows = sqlx::query(
            "SELECT id, title, workspace_path, created_at, updated_at \
             FROM chat_sessions ORDER BY updated_at DESC LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| ChatSession {
                id:             r.get("id"),
                title:          r.get("title"),
                workspace_path: r.get("workspace_path"),
                created_at:     r.get("created_at"),
                updated_at:     r.get("updated_at"),
                messages:       vec![],
            })
            .collect())
    }

    pub async fn save_session(
        &self,
        session_id: Option<String>,
        title: &str,
        workspace_path: Option<&str>,
        messages: &[Value],
    ) -> Result<String> {
        let id = session_id.unwrap_or_else(|| uuid());
        let now = now_ms();
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT created_at FROM chat_sessions WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await?;

        let created_at = existing.map(|row| row.0).unwrap_or(now);

        if existing.is_some() {
            sqlx::query(
                "UPDATE chat_sessions SET title = ?, workspace_path = ?, updated_at = ? WHERE id = ?",
            )
            .bind(title)
            .bind(workspace_path)
            .bind(now)
            .bind(&id)
            .execute(&self.pool)
            .await?;

            sqlx::query("DELETE FROM session_messages WHERE session_id = ?")
                .bind(&id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO chat_sessions (id, title, workspace_path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(title)
            .bind(workspace_path)
            .bind(created_at)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        for message in messages {
            let role = message["role"].as_str().unwrap_or("user");
            let content = message["content"].as_str().unwrap_or("");
            let stats_json = message
                .get("stats")
                .and_then(|s| serde_json::to_string(s).ok());
            sqlx::query(
                "INSERT INTO session_messages (id, session_id, role, content, stats_json, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid())
            .bind(&id)
            .bind(role)
            .bind(content)
            .bind(stats_json)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(id)
    }

    /// Load a session with all its messages.
    pub async fn load_session(&self, id: &str) -> Result<ChatSession> {
        use sqlx::Row;

        let row = sqlx::query(
            "SELECT id, title, workspace_path, created_at, updated_at \
             FROM chat_sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let msgs = sqlx::query(
            "SELECT id, session_id, role, content, stats_json, created_at \
             FROM session_messages WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ChatSession {
            id:             row.get("id"),
            title:          row.get("title"),
            workspace_path: row.get("workspace_path"),
            created_at:     row.get("created_at"),
            updated_at:     row.get("updated_at"),
            messages: msgs
                .iter()
                .map(|m| {
                    let stats_str: Option<String> = m.get("stats_json");
                    SessionMessage {
                        id:         m.get("id"),
                        session_id: m.get("session_id"),
                        role:       m.get("role"),
                        content:    m.get("content"),
                        stats:      stats_str.and_then(|s| serde_json::from_str(&s).ok()),
                        created_at: m.get("created_at"),
                    }
                })
                .collect(),
        })
    }

    // ── Message CRUD ─────────────────────────────────────────────────────────

    pub async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        stats: Option<&serde_json::Value>,
    ) -> Result<String> {
        let id  = uuid();
        let now = now_ms();
        let stats_json = stats.map(|s| s.to_string());

        sqlx::query(
            "INSERT INTO session_messages (id, session_id, role, content, stats_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(stats_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // Bump session updated_at
        sqlx::query("UPDATE chat_sessions SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(id)
    }

    /// Auto-title a session from its first user message.
    pub async fn auto_title(&self, session_id: &str, first_user_msg: &str) -> Result<()> {
        let title: String = first_user_msg.chars().take(60).collect();
        let title = if first_user_msg.len() > 60 { format!("{title}…") } else { title };
        self.rename_session(session_id, &title).await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{t:x}-{}", rand_hex(8))
}

fn rand_hex(n: usize) -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..n).map(|_| rand::thread_rng().gen::<u8>()).collect();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
