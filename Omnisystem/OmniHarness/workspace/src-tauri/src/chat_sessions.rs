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
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    /// JSON-encoded token stats (optional; only on assistant messages).
    pub stats: Option<serde_json::Value>,
    /// JSON-encoded tool list used for this response.
    pub tools_used: Option<serde_json::Value>,
    /// Optional agent metadata for swarm/agent-attributed responses.
    pub agent_id: Option<String>,
    pub agent_label: Option<String>,
    pub agent_color: Option<String>,
    pub agent_icon: Option<String>,
    pub agent_slot: Option<i64>,
    pub created_at: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub group_ids: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// Only populated when loading a specific session.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub messages: Vec<SessionMessage>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ChatSessionGroup {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub chat_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
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
                tools_used_json TEXT,
                agent_id    TEXT,
                agent_label TEXT,
                agent_color TEXT,
                agent_icon  TEXT,
                agent_slot  INTEGER,
                created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_msgs_session ON session_messages(session_id, created_at);

            CREATE TABLE IF NOT EXISTS chat_session_groups (
                id          TEXT    PRIMARY KEY,
                title       TEXT    NOT NULL DEFAULT 'New session',
                tags_json   TEXT    NOT NULL DEFAULT '[]',
                is_locked   INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                is_deleted  INTEGER NOT NULL DEFAULT 0,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_chat_session_groups_updated ON chat_session_groups(updated_at DESC);

            CREATE TABLE IF NOT EXISTS chat_group_links (
                group_id TEXT NOT NULL REFERENCES chat_session_groups(id) ON DELETE CASCADE,
                chat_id  TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
                linked_at INTEGER NOT NULL,
                PRIMARY KEY (group_id, chat_id)
            );
            CREATE INDEX IF NOT EXISTS idx_chat_group_links_chat ON chat_group_links(chat_id);
            "#,
        )
        .execute(&pool)
        .await?;

        let _ = sqlx::query(
            "ALTER TABLE chat_sessions ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'",
        )
        .execute(&pool)
        .await;
        let _ = sqlx::query(
            "ALTER TABLE chat_sessions ADD COLUMN is_locked INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&pool)
        .await;
        let _ = sqlx::query(
            "ALTER TABLE chat_sessions ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&pool)
        .await;
        let _ = sqlx::query(
            "ALTER TABLE chat_sessions ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&pool)
        .await;

        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN tools_used_json TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_id TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_label TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_color TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_icon TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_slot INTEGER")
            .execute(&pool)
            .await;

        Ok(Self { pool })
    }

    // ── Session CRUD ─────────────────────────────────────────────────────────

    pub async fn create_session(
        &self,
        title: &str,
        workspace_path: Option<&str>,
    ) -> Result<String> {
        let id = uuid();
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

    pub async fn update_session_meta(
        &self,
        id: &str,
        title: Option<&str>,
        tags: Option<Vec<String>>,
        is_locked: Option<bool>,
        is_favorite: Option<bool>,
        is_deleted: Option<bool>,
    ) -> Result<()> {
        let mut row = sqlx::query(
            "SELECT title, tags_json, is_locked, is_favorite, is_deleted FROM chat_sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let next_title: String = title.map(String::from).unwrap_or_else(|| row.get("title"));
        let next_tags: String = match tags {
            Some(t) => serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()),
            None => row.get("tags_json"),
        };
        let next_locked: i64 = is_locked
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_locked"));
        let next_favorite: i64 = is_favorite
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_favorite"));
        let next_deleted: i64 = is_deleted
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_deleted"));

        sqlx::query(
            "UPDATE chat_sessions
             SET title = ?, tags_json = ?, is_locked = ?, is_favorite = ?, is_deleted = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(next_title)
        .bind(next_tags)
        .bind(next_locked)
        .bind(next_favorite)
        .bind(next_deleted)
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
            let stats_json = message
                .stats
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok());
            let tools_used_json = message
                .tools_used
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok());
            sqlx::query(
                "INSERT INTO session_messages (id, session_id, role, content, stats_json, tools_used_json, agent_id, agent_label, agent_color, agent_icon, agent_slot, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid())
            .bind(&new_id)
            .bind(&message.role)
            .bind(&message.content)
            .bind(stats_json)
            .bind(tools_used_json)
            .bind(&message.agent_id)
            .bind(&message.agent_label)
            .bind(&message.agent_color)
            .bind(&message.agent_icon)
            .bind(message.agent_slot)
            .bind(message.created_at)
            .execute(&self.pool)
            .await?;
        }

        Ok(new_id)
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let lock_row = sqlx::query("SELECT is_locked FROM chat_sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        if let Some(r) = lock_row {
            let is_locked: i64 = r.get("is_locked");
            if is_locked != 0 {
                anyhow::bail!("Chat is locked. Unlock it before deletion.");
            }
        }
        sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let rows = sqlx::query(
            "SELECT id, title, workspace_path, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at \
             FROM chat_sessions
             WHERE is_deleted = 0
             ORDER BY is_favorite DESC, updated_at DESC
             LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| self.row_to_chat_session(r, vec![]))
            .collect())
    }

    pub async fn list_sessions_detailed(&self, include_deleted: bool) -> Result<Vec<ChatSession>> {
        let rows = if include_deleted {
            sqlx::query(
                "SELECT id, title, workspace_path, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at
                 FROM chat_sessions
                 ORDER BY is_favorite DESC, updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, workspace_path, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at
                 FROM chat_sessions
                 WHERE is_deleted = 0
                 ORDER BY is_favorite DESC, updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut out = Vec::with_capacity(rows.len());
        for r in &rows {
            let id: String = r.get("id");
            let groups = self.group_ids_for_chat(&id).await?;
            out.push(self.row_to_chat_session(r, groups));
        }
        Ok(out)
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
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT created_at FROM chat_sessions WHERE id = ?")
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
            let tools_used_json = message
                .get("tools_used")
                .and_then(|s| serde_json::to_string(s).ok());
            let agent_id = message.get("agent_id").and_then(|v| v.as_str());
            let agent_label = message.get("agent_label").and_then(|v| v.as_str());
            let agent_color = message.get("agent_color").and_then(|v| v.as_str());
            let agent_icon = message.get("agent_icon").and_then(|v| v.as_str());
            let agent_slot = message.get("agent_slot").and_then(|v| v.as_i64());
            let created_at = message
                .get("created_at")
                .and_then(|v| v.as_i64())
                .unwrap_or(now);
            sqlx::query(
                "INSERT INTO session_messages (id, session_id, role, content, stats_json, tools_used_json, agent_id, agent_label, agent_color, agent_icon, agent_slot, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid())
            .bind(&id)
            .bind(role)
            .bind(content)
            .bind(stats_json)
            .bind(tools_used_json)
            .bind(agent_id)
            .bind(agent_label)
            .bind(agent_color)
            .bind(agent_icon)
            .bind(agent_slot)
            .bind(created_at)
            .execute(&self.pool)
            .await?;
        }

        Ok(id)
    }

    /// Load a session with all its messages.
    pub async fn load_session(&self, id: &str) -> Result<ChatSession> {
        use sqlx::Row;

        let row = sqlx::query(
            "SELECT id, title, workspace_path, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at \
             FROM chat_sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let msgs = sqlx::query(
              "SELECT id, session_id, role, content, stats_json, tools_used_json, agent_id, agent_label, agent_color, agent_icon, agent_slot, created_at \
             FROM session_messages WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let groups = self.group_ids_for_chat(id).await?;
        let mut chat = self.row_to_chat_session(&row, groups);
        chat.messages = msgs
            .iter()
            .map(|m| {
                let stats_str: Option<String> = m.get("stats_json");
                let tools_used_str: Option<String> = m.get("tools_used_json");
                SessionMessage {
                    id: m.get("id"),
                    session_id: m.get("session_id"),
                    role: m.get("role"),
                    content: m.get("content"),
                    stats: stats_str.and_then(|s| serde_json::from_str(&s).ok()),
                    tools_used: tools_used_str.and_then(|s| serde_json::from_str(&s).ok()),
                    agent_id: m.get("agent_id"),
                    agent_label: m.get("agent_label"),
                    agent_color: m.get("agent_color"),
                    agent_icon: m.get("agent_icon"),
                    agent_slot: m.get("agent_slot"),
                    created_at: m.get("created_at"),
                }
            })
            .collect();
        Ok(chat)
    }

    // ── Session groups (multi-chat containers) ─────────────────────────────

    pub async fn list_groups(&self, include_deleted: bool) -> Result<Vec<ChatSessionGroup>> {
        let rows = if include_deleted {
            sqlx::query(
                "SELECT id, title, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at
                 FROM chat_session_groups
                 ORDER BY is_favorite DESC, updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at
                 FROM chat_session_groups
                 WHERE is_deleted = 0
                 ORDER BY is_favorite DESC, updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let gid: String = r.get("id");
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM chat_group_links WHERE group_id = ?")
                    .bind(&gid)
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or(0);
            out.push(ChatSessionGroup {
                id: gid,
                title: r.get("title"),
                tags: parse_tags(r.get::<String, _>("tags_json")),
                is_locked: r.get::<i64, _>("is_locked") != 0,
                is_favorite: r.get::<i64, _>("is_favorite") != 0,
                is_deleted: r.get::<i64, _>("is_deleted") != 0,
                chat_count: count,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }
        Ok(out)
    }

    pub async fn create_group(&self, title: &str) -> Result<String> {
        let id = uuid();
        let now = now_ms();
        sqlx::query(
            "INSERT INTO chat_session_groups (id, title, tags_json, is_locked, is_favorite, is_deleted, created_at, updated_at)
             VALUES (?, ?, '[]', 0, 0, 0, ?, ?)",
        )
        .bind(&id)
        .bind(title)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn update_group_meta(
        &self,
        id: &str,
        title: Option<&str>,
        tags: Option<Vec<String>>,
        is_locked: Option<bool>,
        is_favorite: Option<bool>,
        is_deleted: Option<bool>,
    ) -> Result<()> {
        let row = sqlx::query(
            "SELECT title, tags_json, is_locked, is_favorite, is_deleted FROM chat_session_groups WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let next_title: String = title.map(String::from).unwrap_or_else(|| row.get("title"));
        let next_tags: String = match tags {
            Some(t) => serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()),
            None => row.get("tags_json"),
        };
        let next_locked: i64 = is_locked
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_locked"));
        let next_favorite: i64 = is_favorite
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_favorite"));
        let next_deleted: i64 = is_deleted
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or_else(|| row.get::<i64, _>("is_deleted"));

        sqlx::query(
            "UPDATE chat_session_groups
             SET title = ?, tags_json = ?, is_locked = ?, is_favorite = ?, is_deleted = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(next_title)
        .bind(next_tags)
        .bind(next_locked)
        .bind(next_favorite)
        .bind(next_deleted)
        .bind(now_ms())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn link_chat_to_group(&self, chat_id: &str, group_id: &str) -> Result<()> {
        let group_locked: Option<i64> =
            sqlx::query_scalar("SELECT is_locked FROM chat_session_groups WHERE id = ?")
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await?;
        if group_locked == Some(1) {
            anyhow::bail!("Session is locked. Unlock it before linking chats.");
        }

        sqlx::query(
            "INSERT INTO chat_group_links (group_id, chat_id, linked_at)
             VALUES (?, ?, ?)
             ON CONFLICT(group_id, chat_id) DO NOTHING",
        )
        .bind(group_id)
        .bind(chat_id)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE chat_session_groups SET updated_at = ? WHERE id = ?")
            .bind(now_ms())
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn unlink_chat_from_group(&self, chat_id: &str, group_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM chat_group_links WHERE group_id = ? AND chat_id = ?")
            .bind(group_id)
            .bind(chat_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE chat_session_groups SET updated_at = ? WHERE id = ?")
            .bind(now_ms())
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_group_chats(&self, group_id: &str) -> Result<Vec<ChatSession>> {
        let rows = sqlx::query(
            "SELECT c.id, c.title, c.workspace_path, c.tags_json, c.is_locked, c.is_favorite, c.is_deleted, c.created_at, c.updated_at
             FROM chat_sessions c
             JOIN chat_group_links l ON c.id = l.chat_id
             WHERE l.group_id = ?
             ORDER BY c.is_favorite DESC, c.updated_at DESC",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let cid: String = r.get("id");
            let groups = self.group_ids_for_chat(&cid).await?;
            out.push(self.row_to_chat_session(&r, groups));
        }
        Ok(out)
    }

    async fn group_ids_for_chat(&self, chat_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT group_id FROM chat_group_links WHERE chat_id = ? ORDER BY linked_at",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| r.get::<String, _>("group_id"))
            .collect())
    }

    fn row_to_chat_session(
        &self,
        r: &sqlx::sqlite::SqliteRow,
        group_ids: Vec<String>,
    ) -> ChatSession {
        ChatSession {
            id: r.get("id"),
            title: r.get("title"),
            workspace_path: r.get("workspace_path"),
            tags: parse_tags(r.get::<String, _>("tags_json")),
            is_locked: r.get::<i64, _>("is_locked") != 0,
            is_favorite: r.get::<i64, _>("is_favorite") != 0,
            is_deleted: r.get::<i64, _>("is_deleted") != 0,
            group_ids,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            messages: vec![],
        }
    }

    // ── Message CRUD ─────────────────────────────────────────────────────────

    pub async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        stats: Option<&serde_json::Value>,
    ) -> Result<String> {
        let id = uuid();
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
        let title = if first_user_msg.len() > 60 {
            format!("{title}…")
        } else {
            title
        };
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

fn parse_tags(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}
