use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtContentType {
    Text,
    Json,
    Code,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtContent {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtSource {
    PrimaryModel { model_id: Option<String>, adapter: Option<String> },
    Adapter { adapter: String },
    Tool { tool_name: String },
    System,
    Other { label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSegment {
    pub id: Uuid,
    pub parent_segment: Option<Uuid>,
    pub turn_id: Uuid,
    pub session_id: Uuid,
    pub timestamp_ms: i64,
    pub source: ThoughtSource,
    pub content_type: ThoughtContentType,
    pub content: ThoughtContent,
    pub confidence: Option<f32>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

pub struct ThoughtsStore {
    pool: SqlitePool,
}

impl ThoughtsStore {
    pub async fn new(pool: SqlitePool) -> Result<Self, String> {
        let s = Self { pool };
        s.migrate().await?;
        Ok(s)
    }

    async fn migrate(&self) -> Result<(), String> {
        // Use a module-scoped migrations table to avoid colliding with other modules
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS thoughts_schema_migrations (
                version    INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("migrate: {e}"))?;

        self.apply_migration(2, "create_thinking_history_table", &[
            "CREATE TABLE IF NOT EXISTS thinking_history (
                id          TEXT PRIMARY KEY,
                session_id  TEXT NOT NULL,
                turn_id     TEXT NOT NULL,
                model_role  TEXT NOT NULL,
                content     TEXT NOT NULL,
                token_count INTEGER NOT NULL DEFAULT 0,
                created_at  INTEGER NOT NULL
            )",
            "CREATE INDEX IF NOT EXISTS idx_th_session  ON thinking_history(session_id)",
            "CREATE INDEX IF NOT EXISTS idx_th_turn     ON thinking_history(turn_id)",
            "CREATE INDEX IF NOT EXISTS idx_th_created  ON thinking_history(created_at)",
        ]).await?;

        self.apply_migration(1, "create_thoughts_table", &[
            "CREATE TABLE IF NOT EXISTS thoughts (
                id TEXT PRIMARY KEY,
                parent_segment TEXT,
                turn_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                source TEXT NOT NULL,
                source_model TEXT,
                source_adapter TEXT,
                source_tool TEXT,
                content_type TEXT NOT NULL,
                content_text TEXT,
                content_json TEXT,
                confidence REAL,
                metadata_json TEXT
            )",
            "CREATE INDEX IF NOT EXISTS idx_thoughts_turn ON thoughts(turn_id)",
            "CREATE INDEX IF NOT EXISTS idx_thoughts_session ON thoughts(session_id)",
        ]).await?;

        Ok(())
    }

    async fn apply_migration(&self, version: i64, description: &str, stmts: &[&str]) -> Result<(), String> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM thoughts_schema_migrations WHERE version = ?",
        )
        .bind(version)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if exists > 0 { return Ok(()); }

        for stmt in stmts {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("migration {version} ({description}): {e}"))?;
        }

        sqlx::query(
            "INSERT INTO thoughts_schema_migrations (version, applied_at, description) VALUES (?,?,?)",
        )
        .bind(version).bind(now_ms()).bind(description)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("record migration {version}: {e}"))?;

        Ok(())
    }

    pub async fn add_thought(&self, t: ThoughtSegment) -> Result<(), String> {
        let id = t.id.to_string();
        let parent = t.parent_segment.map(|u| u.to_string());
        let turn_id = t.turn_id.to_string();
        let session_id = t.session_id.to_string();
        let ts = t.timestamp_ms;

        let (source, source_model, source_adapter, source_tool) = match &t.source {
            ThoughtSource::PrimaryModel { model_id, adapter } => (
                "PrimaryModel".to_string(),
                model_id.clone(),
                adapter.clone(),
                None,
            ),
            ThoughtSource::Adapter { adapter } => (
                "Adapter".to_string(),
                None,
                Some(adapter.clone()),
                None,
            ),
            ThoughtSource::Tool { tool_name } => (
                "Tool".to_string(),
                None,
                None,
                Some(tool_name.clone()),
            ),
            ThoughtSource::System => ("System".to_string(), None, None, None),
            ThoughtSource::Other { label } => (format!("Other:{}", label), None, None, None),
        };

        let content_type = match &t.content_type {
            ThoughtContentType::Text => "Text".to_string(),
            ThoughtContentType::Json => "Json".to_string(),
            ThoughtContentType::Code => "Code".to_string(),
            ThoughtContentType::Other(s) => format!("Other:{}", s),
        };

        let content_text = t.content.text.clone();
        let content_json = t.content.json.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let confidence = t.confidence.map(|f| f as f64);
        let metadata_json = serde_json::to_string(&t.metadata).ok();

        sqlx::query("INSERT INTO thoughts (id,parent_segment,turn_id,session_id,timestamp_ms,source,source_model,source_adapter,source_tool,content_type,content_text,content_json,confidence,metadata_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&id)
            .bind(&parent)
            .bind(&turn_id)
            .bind(&session_id)
            .bind(ts)
            .bind(&source)
            .bind(&source_model)
            .bind(&source_adapter)
            .bind(&source_tool)
            .bind(&content_type)
            .bind(&content_text)
            .bind(&content_json)
            .bind(&confidence)
            .bind(&metadata_json)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("add_thought: {e}"))?;

        Ok(())
    }

    pub async fn get_thoughts_for_turn(&self, turn_id: &str) -> Result<Vec<ThoughtSegment>, String> {
        let rows = sqlx::query(
            "SELECT id,parent_segment,turn_id,session_id,timestamp_ms,source,source_model,source_adapter,source_tool,content_type,content_text,content_json,confidence,metadata_json FROM thoughts WHERE turn_id = ? ORDER BY timestamp_ms ASC",
        )
        .bind(turn_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("get_thoughts_for_turn: {e}"))?;

        let mut out = Vec::new();
        for r in rows.iter() {
            let id_str: String = r.get::<String, _>("id");
            let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil());
            let parent_str: Option<String> = r.get::<Option<String>, _>("parent_segment");
            let parent = parent_str.and_then(|s| Uuid::parse_str(&s).ok());

            let turn_id_uuid = Uuid::parse_str(&r.get::<String, _>("turn_id")).unwrap_or_else(|_| Uuid::nil());
            let session_id_uuid = Uuid::parse_str(&r.get::<String, _>("session_id")).unwrap_or_else(|_| Uuid::nil());
            let ts = r.get::<i64, _>("timestamp_ms");

            let source_str: String = r.get::<String, _>("source");
            let source = if source_str == "PrimaryModel" {
                ThoughtSource::PrimaryModel { model_id: r.get::<Option<String>, _>("source_model"), adapter: r.get::<Option<String>, _>("source_adapter") }
            } else if source_str == "Adapter" {
                ThoughtSource::Adapter { adapter: r.get::<Option<String>, _>("source_adapter").unwrap_or_default() }
            } else if source_str == "Tool" {
                ThoughtSource::Tool { tool_name: r.get::<Option<String>, _>("source_tool").unwrap_or_default() }
            } else if source_str == "System" {
                ThoughtSource::System
            } else if source_str.starts_with("Other:") {
                ThoughtSource::Other { label: source_str.trim_start_matches("Other:").to_string() }
            } else {
                ThoughtSource::Other { label: source_str }
            };

            let content_type_str: String = r.get::<String, _>("content_type");
            let content_type = match content_type_str.as_str() {
                "Text" => ThoughtContentType::Text,
                "Json" => ThoughtContentType::Json,
                "Code" => ThoughtContentType::Code,
                s if s.starts_with("Other:") => ThoughtContentType::Other(s.trim_start_matches("Other:").to_string()),
                _ => ThoughtContentType::Text,
            };

            let content_text = r.get::<Option<String>, _>("content_text");
            let content_json = r.get::<Option<String>, _>("content_json").and_then(|s| serde_json::from_str(&s).ok());
            let content = ThoughtContent { text: content_text, json: content_json };

            let confidence = r.get::<Option<f64>, _>("confidence").map(|f| f as f32);
            let metadata = r.get::<Option<String>, _>("metadata_json").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();

            out.push(ThoughtSegment {
                id,
                parent_segment: parent,
                turn_id: turn_id_uuid,
                session_id: session_id_uuid,
                timestamp_ms: ts,
                source,
                content_type,
                content,
                confidence,
                metadata,
            });
        }

        Ok(out)
    }

    pub async fn clear_thoughts_for_session(&self, session_id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM thoughts WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("clear_thoughts_for_session: {e}"))?;
        Ok(())
    }

    // ── thinking_history helpers ──────────────────────────────────────────────

    pub async fn record_thinking(
        &self,
        session_id: &str,
        turn_id: &str,
        model_role: &str,
        content: &str,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let token_count = content.split_whitespace().count() as i64;
        sqlx::query(
            "INSERT INTO thinking_history (id,session_id,turn_id,model_role,content,token_count,created_at) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(turn_id)
        .bind(model_role)
        .bind(content)
        .bind(token_count)
        .bind(now_ms())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("record_thinking: {e}"))?;
        Ok(id)
    }

    pub async fn search_thinking_history(
        &self,
        query: &str,
        session_id: Option<&str>,
        model_role: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ThinkingHistoryEntry>, String> {
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let rows = sqlx::query(
            "SELECT id, session_id, turn_id, model_role, content, token_count, created_at
             FROM thinking_history
             WHERE content LIKE ?
               AND (? IS NULL OR session_id = ?)
               AND (? IS NULL OR model_role = ?)
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(session_id)
        .bind(session_id)
        .bind(model_role)
        .bind(model_role)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("search_thinking_history: {e}"))?;

        Ok(rows.iter().map(|r| ThinkingHistoryEntry {
            id:          r.get::<String, _>("id"),
            session_id:  r.get::<String, _>("session_id"),
            turn_id:     r.get::<String, _>("turn_id"),
            model_role:  r.get::<String, _>("model_role"),
            content:     r.get::<String, _>("content"),
            token_count: r.get::<i64, _>("token_count"),
            created_at:  r.get::<i64, _>("created_at"),
        }).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingHistoryEntry {
    pub id:          String,
    pub session_id:  String,
    pub turn_id:     String,
    pub model_role:  String,
    pub content:     String,
    pub token_count: i64,
    pub created_at:  i64,
}
