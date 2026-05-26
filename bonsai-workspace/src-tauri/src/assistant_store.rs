use sqlx::{Row, SqlitePool};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn uuid() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect()
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantProfile {
    pub id:               String,
    pub name:             String,
    pub persona_id:       Option<String>,
    pub avatar_id:        Option<String>,
    pub tts_voice:        String,
    pub tts_speed:        f64,
    pub tts_pitch:        f64,
    pub tts_enabled:      bool,
    pub wake_word:        Option<String>,
    pub tool_permissions: String,
    pub system_prompt:    String,
    pub model_id:         Option<String>,
    pub is_active:        bool,
    pub created_at:       i64,
    pub updated_at:       i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarAsset {
    pub id:            String,
    pub name:          String,
    pub asset_type:    String,
    pub asset_data:    Option<String>,
    pub file_path:     Option<String>,
    pub thumbnail_svg: Option<String>,
    pub validated:     bool,
    pub created_at:    i64,
    pub updated_at:    i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantSession {
    pub id:         String,
    pub profile_id: Option<String>,
    pub title:      String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id:              String,
    pub session_id:      String,
    pub role:            String,
    pub content:         String,
    pub tool_name:       Option<String>,
    pub tool_result:     Option<String>,
    pub tts_synthesized: bool,
    pub created_at:      i64,
    #[serde(default)]
    pub tool_call_id:    Option<String>,
    /// Embedded game state for inline board rendering in chat.
    #[serde(default)]
    pub game_state:      Option<crate::games::ChatGameState>,
}

// ── AssistantStore ────────────────────────────────────────────────────────────

pub struct AssistantStore {
    pool: SqlitePool,
}

impl AssistantStore {
    pub async fn new(pool: SqlitePool) -> Result<Self, String> {
        let store = AssistantStore { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), String> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version    INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL,
                description TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("migrate: {e}"))?;

        self.apply_migration(1, "assistant_profiles", &[
            "CREATE TABLE IF NOT EXISTS assistant_profiles (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL DEFAULT 'Bonsai Buddy',
                persona_id      TEXT,
                avatar_id       TEXT,
                tts_voice       TEXT NOT NULL DEFAULT 'en_US-amy-medium',
                tts_speed       REAL NOT NULL DEFAULT 1.0,
                tts_pitch       REAL NOT NULL DEFAULT 1.0,
                tts_enabled     INTEGER NOT NULL DEFAULT 1,
                wake_word       TEXT,
                tool_permissions TEXT NOT NULL DEFAULT '{}',
                system_prompt   TEXT NOT NULL DEFAULT 'You are Bonsai Buddy, a helpful personal AI assistant.',
                model_id        TEXT,
                is_active       INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            )",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_one_active_profile
                ON assistant_profiles(is_active) WHERE is_active = 1",
        ]).await?;

        self.apply_migration(2, "avatar_assets", &[
            "CREATE TABLE IF NOT EXISTS avatar_assets (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                asset_type    TEXT NOT NULL CHECK(asset_type IN ('svg_builtin','svg_custom','photo')),
                asset_data    TEXT,
                file_path     TEXT,
                thumbnail_svg TEXT,
                validated     INTEGER NOT NULL DEFAULT 0,
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL
            )",
        ]).await?;

        self.apply_migration(3, "assistant_sessions", &[
            "CREATE TABLE IF NOT EXISTS assistant_sessions (
                id         TEXT PRIMARY KEY,
                profile_id TEXT,
                title      TEXT NOT NULL DEFAULT 'New conversation',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(profile_id) REFERENCES assistant_profiles(id) ON DELETE CASCADE
            )",
            "CREATE INDEX IF NOT EXISTS idx_asst_sessions_profile
                ON assistant_sessions(profile_id, updated_at DESC)",
        ]).await?;

        self.apply_migration(4, "assistant_messages", &[
            "CREATE TABLE IF NOT EXISTS assistant_messages (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL,
                role            TEXT NOT NULL CHECK(role IN ('user','assistant','tool')),
                content         TEXT NOT NULL,
                tool_name       TEXT,
                tool_result     TEXT,
                tts_synthesized INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES assistant_sessions(id) ON DELETE CASCADE
            )",
            "CREATE INDEX IF NOT EXISTS idx_asst_msgs_session
                ON assistant_messages(session_id, created_at)",
        ]).await?;

        self.apply_migration(5, "backup_registry", &[
            "CREATE TABLE IF NOT EXISTS backup_registry (
                id         TEXT PRIMARY KEY,
                filename   TEXT NOT NULL,
                file_path  TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                includes   TEXT NOT NULL,
                checksum   TEXT,
                encrypted  INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )",
        ]).await?;

        self.apply_migration(6, "mcp_servers", &[
            "CREATE TABLE IF NOT EXISTS mcp_servers (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                command    TEXT NOT NULL,
                args       TEXT NOT NULL DEFAULT '[]',
                namespace  TEXT NOT NULL,
                enabled    INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ]).await?;

        self.apply_migration(7, "assistant_messages_tool_call_id", &[
            "ALTER TABLE assistant_messages ADD COLUMN tool_call_id TEXT",
        ]).await.ok(); // ok() — ALTER TABLE fails gracefully if column already exists

        // Seed default profile if none exists
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assistant_profiles")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        if count == 0 {
            let id = uuid();
            let ts = now();
            sqlx::query(
                "INSERT INTO assistant_profiles
                 (id,name,tts_voice,tts_speed,tts_pitch,tts_enabled,tool_permissions,system_prompt,is_active,created_at,updated_at)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(&id).bind("Bonsai Buddy").bind("en_US-amy-medium")
            .bind(1.0_f64).bind(1.0_f64).bind(1_i64)
            .bind("{}").bind("You are Bonsai Buddy, a helpful personal AI assistant.")
            .bind(1_i64).bind(ts).bind(ts)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("seed profile: {e}"))?;
        }

        Ok(())
    }

    async fn apply_migration(&self, version: i64, description: &str, stmts: &[&str]) -> Result<(), String> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?",
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
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?,?,?)",
        )
        .bind(version).bind(now()).bind(description)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("record migration {version}: {e}"))?;

        Ok(())
    }

    // ── Profile CRUD ──────────────────────────────────────────────────────────

    pub async fn list_profiles(&self) -> Result<Vec<AssistantProfile>, String> {
        let rows = sqlx::query(
            "SELECT id,name,persona_id,avatar_id,tts_voice,tts_speed,tts_pitch,tts_enabled,
                    wake_word,tool_permissions,system_prompt,model_id,is_active,created_at,updated_at
             FROM assistant_profiles ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_profiles: {e}"))?;

        Ok(rows.iter().map(row_to_profile).collect())
    }

    pub async fn get_active_profile(&self) -> Result<Option<AssistantProfile>, String> {
        let row = sqlx::query(
            "SELECT id,name,persona_id,avatar_id,tts_voice,tts_speed,tts_pitch,tts_enabled,
                    wake_word,tool_permissions,system_prompt,model_id,is_active,created_at,updated_at
             FROM assistant_profiles WHERE is_active = 1 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("get_active_profile: {e}"))?;

        Ok(row.as_ref().map(row_to_profile))
    }

    pub async fn upsert_profile(&self, mut p: AssistantProfile) -> Result<AssistantProfile, String> {
        let ts = now();
        if p.id.is_empty() { p.id = uuid(); p.created_at = ts; }
        p.updated_at = ts;

        sqlx::query(
            "INSERT INTO assistant_profiles
             (id,name,persona_id,avatar_id,tts_voice,tts_speed,tts_pitch,tts_enabled,
              wake_word,tool_permissions,system_prompt,model_id,is_active,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, persona_id=excluded.persona_id,
               avatar_id=excluded.avatar_id, tts_voice=excluded.tts_voice,
               tts_speed=excluded.tts_speed, tts_pitch=excluded.tts_pitch,
               tts_enabled=excluded.tts_enabled, wake_word=excluded.wake_word,
               tool_permissions=excluded.tool_permissions, system_prompt=excluded.system_prompt,
               model_id=excluded.model_id, is_active=excluded.is_active,
               updated_at=excluded.updated_at",
        )
        .bind(&p.id).bind(&p.name).bind(&p.persona_id).bind(&p.avatar_id)
        .bind(&p.tts_voice).bind(p.tts_speed).bind(p.tts_pitch).bind(p.tts_enabled as i64)
        .bind(&p.wake_word).bind(&p.tool_permissions).bind(&p.system_prompt)
        .bind(&p.model_id).bind(p.is_active as i64).bind(p.created_at).bind(p.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_profile: {e}"))?;

        Ok(p)
    }

    pub async fn delete_profile(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM assistant_profiles WHERE id = ?")
            .bind(id).execute(&self.pool).await
            .map_err(|e| format!("delete_profile: {e}"))?;
        Ok(())
    }

    pub async fn set_active_profile(&self, id: &str) -> Result<(), String> {
        sqlx::query("UPDATE assistant_profiles SET is_active = 0, updated_at = ?")
            .bind(now()).execute(&self.pool).await
            .map_err(|e| format!("deactivate profiles: {e}"))?;
        sqlx::query("UPDATE assistant_profiles SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(now()).bind(id).execute(&self.pool).await
            .map_err(|e| format!("activate profile: {e}"))?;
        Ok(())
    }

    // ── Avatar CRUD ───────────────────────────────────────────────────────────

    pub async fn list_avatars(&self) -> Result<Vec<AvatarAsset>, String> {
        let rows = sqlx::query(
            "SELECT id,name,asset_type,asset_data,file_path,thumbnail_svg,validated,created_at,updated_at
             FROM avatar_assets ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_avatars: {e}"))?;

        Ok(rows.iter().map(row_to_avatar).collect())
    }

    pub async fn upsert_avatar(&self, mut a: AvatarAsset) -> Result<AvatarAsset, String> {
        let ts = now();
        if a.id.is_empty() { a.id = uuid(); a.created_at = ts; }
        a.updated_at = ts;

        sqlx::query(
            "INSERT INTO avatar_assets
             (id,name,asset_type,asset_data,file_path,thumbnail_svg,validated,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, asset_type=excluded.asset_type,
               asset_data=excluded.asset_data, file_path=excluded.file_path,
               thumbnail_svg=excluded.thumbnail_svg, validated=excluded.validated,
               updated_at=excluded.updated_at",
        )
        .bind(&a.id).bind(&a.name).bind(&a.asset_type).bind(&a.asset_data)
        .bind(&a.file_path).bind(&a.thumbnail_svg).bind(a.validated as i64)
        .bind(a.created_at).bind(a.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_avatar: {e}"))?;

        Ok(a)
    }

    pub async fn delete_avatar(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM avatar_assets WHERE id = ?")
            .bind(id).execute(&self.pool).await
            .map_err(|e| format!("delete_avatar: {e}"))?;
        Ok(())
    }

    // ── Session CRUD ──────────────────────────────────────────────────────────

    pub async fn list_sessions(&self, profile_id: Option<&str>) -> Result<Vec<AssistantSession>, String> {
        let rows = if let Some(pid) = profile_id {
            sqlx::query(
                "SELECT id,profile_id,title,created_at,updated_at
                 FROM assistant_sessions WHERE profile_id = ? ORDER BY updated_at DESC",
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT id,profile_id,title,created_at,updated_at
                 FROM assistant_sessions ORDER BY updated_at DESC",
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| format!("list_sessions: {e}"))?;

        Ok(rows.iter().map(row_to_session).collect())
    }

    pub async fn create_session(&self, profile_id: Option<&str>, title: &str) -> Result<AssistantSession, String> {
        let ts = now();
        let id = uuid();
        sqlx::query(
            "INSERT INTO assistant_sessions (id,profile_id,title,created_at,updated_at) VALUES (?,?,?,?,?)",
        )
        .bind(&id).bind(profile_id).bind(title).bind(ts).bind(ts)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("create_session: {e}"))?;

        Ok(AssistantSession { id, profile_id: profile_id.map(String::from), title: title.to_string(), created_at: ts, updated_at: ts })
    }

    pub async fn touch_session(&self, id: &str) -> Result<(), String> {
        sqlx::query("UPDATE assistant_sessions SET updated_at = ? WHERE id = ?")
            .bind(now()).bind(id).execute(&self.pool).await
            .map_err(|e| format!("touch_session: {e}"))?;
        Ok(())
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM assistant_sessions WHERE id = ?")
            .bind(id).execute(&self.pool).await
            .map_err(|e| format!("delete_session: {e}"))?;
        Ok(())
    }

    // ── Message CRUD ──────────────────────────────────────────────────────────

    pub async fn load_messages(&self, session_id: &str) -> Result<Vec<AssistantMessage>, String> {
        let rows = sqlx::query(
            "SELECT id,session_id,role,content,tool_name,tool_result,tts_synthesized,created_at,tool_call_id
             FROM assistant_messages WHERE session_id = ? ORDER BY created_at",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load_messages: {e}"))?;

        Ok(rows.iter().map(row_to_message).collect())
    }

    pub async fn append_message(&self, msg: AssistantMessage) -> Result<AssistantMessage, String> {
        let mut m = msg;
        if m.id.is_empty() { m.id = uuid(); }
        if m.created_at == 0 { m.created_at = now(); }

        sqlx::query(
            "INSERT INTO assistant_messages
             (id,session_id,role,content,tool_name,tool_result,tts_synthesized,created_at,tool_call_id)
             VALUES (?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&m.id).bind(&m.session_id).bind(&m.role).bind(&m.content)
        .bind(&m.tool_name).bind(&m.tool_result).bind(m.tts_synthesized as i64)
        .bind(m.created_at).bind(&m.tool_call_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("append_message: {e}"))?;

        let _ = self.touch_session(&m.session_id).await;
        Ok(m)
    }

    // ── Backup support ────────────────────────────────────────────────────────

    pub async fn profile_exists(&self, id: &str) -> Result<bool, String> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assistant_profiles WHERE id = ?")
            .bind(id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub async fn session_exists(&self, id: &str) -> Result<bool, String> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assistant_sessions WHERE id = ?")
            .bind(id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub async fn upsert_session(&self, s: AssistantSession) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO assistant_sessions (id,profile_id,title,created_at,updated_at) VALUES (?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title, updated_at=excluded.updated_at",
        )
        .bind(&s.id).bind(&s.profile_id).bind(&s.title).bind(s.created_at).bind(s.updated_at)
        .execute(&self.pool).await.map_err(|e| format!("upsert_session: {e}"))?;
        Ok(())
    }

    pub async fn delete_all_sessions(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM assistant_messages").execute(&self.pool).await.map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM assistant_sessions").execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_all_profiles(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM assistant_profiles").execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn delete_all_avatars(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM avatar_assets").execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn register_backup(
        &self,
        filename:   &str,
        file_path:  &str,
        size_bytes: i64,
        includes:   &str,
        checksum:   &str,
        encrypted:  bool,
    ) -> Result<(), String> {
        let id = uuid();
        let ts = now();
        sqlx::query(
            "INSERT INTO backup_registry (id,filename,file_path,size_bytes,includes,checksum,encrypted,created_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(&id).bind(filename).bind(file_path)
        .bind(size_bytes).bind(includes).bind(checksum)
        .bind(encrypted as i64).bind(ts)
        .execute(&self.pool).await.map_err(|e| format!("register_backup: {e}"))?;
        Ok(())
    }

    pub async fn rotate_backups(&self, keep: i64) -> Result<(), String> {
        // Delete oldest entries beyond `keep` count
        sqlx::query(
            "DELETE FROM backup_registry WHERE id IN (
               SELECT id FROM backup_registry ORDER BY created_at DESC LIMIT -1 OFFSET ?
             )",
        )
        .bind(keep).execute(&self.pool).await.map_err(|e| format!("rotate_backups: {e}"))?;
        Ok(())
    }

    pub async fn list_backups_raw(&self) -> Result<Vec<crate::assistant_backup::BackupEntry>, String> {
        let rows = sqlx::query(
            "SELECT id,filename,file_path,size_bytes,includes,checksum,encrypted,created_at
             FROM backup_registry ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool).await.map_err(|e| format!("list_backups: {e}"))?;

        Ok(rows.iter().map(|r| {
            let includes_raw: String = r.get("includes");
            let includes = serde_json::from_str::<Vec<String>>(&includes_raw).unwrap_or_default();
            let file_path: String = r.get("file_path");
            let valid = std::path::Path::new(&file_path).exists().then_some(true);
            crate::assistant_backup::BackupEntry {
                id:         r.get("id"),
                filename:   r.get("filename"),
                file_path,
                size_bytes: r.get("size_bytes"),
                includes,
                checksum:   r.get("checksum"),
                encrypted:  r.get::<i64,_>("encrypted") != 0,
                created_at: r.get("created_at"),
                valid,
            }
        }).collect())
    }

    pub async fn delete_backup_entry(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM backup_registry WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn set_session_title(&self, session_id: &str, title: &str) -> Result<(), String> {
        sqlx::query("UPDATE assistant_sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now())
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── MCP server CRUD ───────────────────────────────────────────────────────

    pub async fn list_mcp_servers(&self) -> Result<Vec<crate::mcp_bridge::McpServerConfig>, String> {
        let rows = sqlx::query(
            "SELECT id,name,command,args,namespace,enabled FROM mcp_servers ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_mcp_servers: {e}"))?;

        Ok(rows.iter().map(|r| {
            let args_json: String = r.get("args");
            let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();
            crate::mcp_bridge::McpServerConfig {
                id:        r.get("id"),
                name:      r.get("name"),
                command:   r.get("command"),
                args,
                namespace: r.get("namespace"),
                enabled:   r.get::<i64, _>("enabled") != 0,
            }
        }).collect())
    }

    pub async fn upsert_mcp_server(&self, cfg: &crate::mcp_bridge::McpServerConfig) -> Result<(), String> {
        let args_json = serde_json::to_string(&cfg.args)
            .map_err(|e| format!("serialize args: {e}"))?;
        let ts = now();
        sqlx::query(
            "INSERT INTO mcp_servers (id,name,command,args,namespace,enabled,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, command=excluded.command, args=excluded.args,
               namespace=excluded.namespace, enabled=excluded.enabled, updated_at=excluded.updated_at",
        )
        .bind(&cfg.id).bind(&cfg.name).bind(&cfg.command).bind(&args_json)
        .bind(&cfg.namespace).bind(cfg.enabled as i64).bind(ts).bind(ts)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert_mcp_server: {e}"))?;
        Ok(())
    }

    pub async fn delete_mcp_server(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("delete_mcp_server: {e}"))?;
        Ok(())
    }
}

// ── Row mappers ───────────────────────────────────────────────────────────────

fn row_to_profile(r: &sqlx::sqlite::SqliteRow) -> AssistantProfile {
    AssistantProfile {
        id:               r.get("id"),
        name:             r.get("name"),
        persona_id:       r.get("persona_id"),
        avatar_id:        r.get("avatar_id"),
        tts_voice:        r.get("tts_voice"),
        tts_speed:        r.get("tts_speed"),
        tts_pitch:        r.get("tts_pitch"),
        tts_enabled:      r.get::<i64, _>("tts_enabled") != 0,
        wake_word:        r.get("wake_word"),
        tool_permissions: r.get("tool_permissions"),
        system_prompt:    r.get("system_prompt"),
        model_id:         r.get("model_id"),
        is_active:        r.get::<i64, _>("is_active") != 0,
        created_at:       r.get("created_at"),
        updated_at:       r.get("updated_at"),
    }
}

fn row_to_avatar(r: &sqlx::sqlite::SqliteRow) -> AvatarAsset {
    AvatarAsset {
        id:            r.get("id"),
        name:          r.get("name"),
        asset_type:    r.get("asset_type"),
        asset_data:    r.get("asset_data"),
        file_path:     r.get("file_path"),
        thumbnail_svg: r.get("thumbnail_svg"),
        validated:     r.get::<i64, _>("validated") != 0,
        created_at:    r.get("created_at"),
        updated_at:    r.get("updated_at"),
    }
}

fn row_to_session(r: &sqlx::sqlite::SqliteRow) -> AssistantSession {
    AssistantSession {
        id:         r.get("id"),
        profile_id: r.get("profile_id"),
        title:      r.get("title"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn row_to_message(r: &sqlx::sqlite::SqliteRow) -> AssistantMessage {
    AssistantMessage {
        id:              r.get("id"),
        session_id:      r.get("session_id"),
        role:            r.get("role"),
        content:         r.get("content"),
        tool_name:       r.get("tool_name"),
        tool_result:     r.get("tool_result"),
        tts_synthesized: r.get::<i64, _>("tts_synthesized") != 0,
        created_at:      r.get("created_at"),
        tool_call_id:    r.try_get("tool_call_id").ok().flatten(),
        game_state:      r.try_get::<Option<String>, _>("game_state").ok().flatten()
            .and_then(|s| serde_json::from_str(&s).ok()),
    }
}
