use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_rusqlite::Connection;
use rusqlite::OptionalExtension;

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

pub type Db = Arc<Connection>;

// ── Schema ────────────────────────────────────────────────────────────────────

pub async fn migrate(db: &Db) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute_batch(
            r#"PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS bot_sessions (
                id                TEXT PRIMARY KEY,
                platform          TEXT NOT NULL,
                platform_user_id  TEXT NOT NULL,
                platform_chat_id  TEXT NOT NULL,
                buddy_session_id  TEXT NOT NULL,
                display_name      TEXT,
                created_at        INTEGER NOT NULL,
                last_active       INTEGER NOT NULL,
                is_archived       INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_lookup
                ON bot_sessions(platform, platform_user_id, platform_chat_id);
            CREATE TABLE IF NOT EXISTS pending_confirms (
                token        TEXT PRIMARY KEY,
                platform     TEXT NOT NULL,
                chat_id      TEXT NOT NULL,
                user_id      TEXT NOT NULL,
                tool         TEXT NOT NULL,
                args_json    TEXT NOT NULL,
                prompt       TEXT NOT NULL,
                expires_at   INTEGER NOT NULL,
                prompt_state TEXT NOT NULL DEFAULT 'created',
                prompt_nonce INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS runtime_records (
                id           TEXT PRIMARY KEY,
                kind         TEXT NOT NULL,
                script       TEXT NOT NULL,
                user         TEXT,
                pid          INTEGER,
                status       TEXT NOT NULL,
                started_at   INTEGER NOT NULL,
                timeout_secs INTEGER
            );
            CREATE TABLE IF NOT EXISTS skills (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                description TEXT NOT NULL,
                language    TEXT NOT NULL,
                script_path TEXT NOT NULL,
                version     INTEGER NOT NULL DEFAULT 1,
                enabled     INTEGER NOT NULL DEFAULT 1,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );"#,
        )
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
}

/// Insert or update a runtime record
pub async fn upsert_runtime_record(
    db: &Db,
    id: &str,
    kind: &str,
    script: &str,
    user: Option<&str>,
    pid: Option<i64>,
    status: &str,
    started_at: i64,
    timeout_secs: Option<i64>,
) -> Result<(), tokio_rusqlite::Error> {
    let id_s = id.to_string();
    let kind_s = kind.to_string();
    let script_s = script.to_string();
    let status_s = status.to_string();
    let u = user.map(|s| s.to_string());
    db.call(move |conn| {
        conn.execute(
            r#"INSERT INTO runtime_records
               (id, kind, script, user, pid, status, started_at, timeout_secs)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
               ON CONFLICT(id) DO UPDATE SET
                   kind = excluded.kind,
                   script = excluded.script,
                   user = excluded.user,
                   pid = excluded.pid,
                   status = excluded.status,
                   started_at = excluded.started_at,
                   timeout_secs = excluded.timeout_secs"#,
            rusqlite::params![id_s, kind_s, script_s, u, pid, status_s, started_at, timeout_secs],
        )
        .map(|_| ())
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
}

pub async fn update_runtime_status(
    db: &Db,
    id: &str,
    status: &str,
    pid: Option<i64>,
) -> Result<(), tokio_rusqlite::Error> {
    let id_s = id.to_string();
    let status_s = status.to_string();
        let p = pid;
        db.call(move |conn| {
            conn.execute(
                "UPDATE runtime_records SET status = ?1, pid = COALESCE(?2, pid) WHERE id = ?3",
                rusqlite::params![status_s, p, id_s],
            )
            .map(|_| ())
            .map_err(tokio_rusqlite::Error::from)
        })
        .await
}

pub async fn list_runtime_records(db: &Db) -> Vec<serde_json::Value> {
    db.call(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, kind, script, user, pid, status, started_at, timeout_secs FROM runtime_records ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "kind": row.get::<_, String>(1)?,
                "script": row.get::<_, String>(2)?,
                "user": row.get::<_, Option<String>>(3)?,
                "pid": row.get::<_, Option<i64>>(4)?,
                "status": row.get::<_, String>(5)?,
                "started_at": row.get::<_, i64>(6)?,
                "timeout_secs": row.get::<_, Option<i64>>(7)?,
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(rows)
    })
    .await
    .unwrap_or_default()
}

#[allow(dead_code)]
pub async fn get_runtime_record(db: &Db, id: &str) -> Option<serde_json::Value> {
    let id_s = id.to_string();
    db.call(move |conn| {
        conn.query_row(
            "SELECT id, kind, script, user, pid, status, started_at, timeout_secs FROM runtime_records WHERE id = ?1",
            rusqlite::params![id_s],
            |row| Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "kind": row.get::<_, String>(1)?,
                "script": row.get::<_, String>(2)?,
                "user": row.get::<_, Option<String>>(3)?,
                "pid": row.get::<_, Option<i64>>(4)?,
                "status": row.get::<_, String>(5)?,
                "started_at": row.get::<_, i64>(6)?,
                "timeout_secs": row.get::<_, Option<i64>>(7)?,
            }))
        ).optional().map_err(tokio_rusqlite::Error::from)
    })
    .await
    .ok()
    .flatten()
}

// ── Sessions ─────────────────────────────────────────────────────────────────

pub async fn find_active_session(
    db: &Db,
    platform: String,
    user_id: String,
    chat_id: String,
) -> Option<String> {
    db.call(move |conn| {
        conn.query_row(
            "SELECT buddy_session_id FROM bot_sessions
             WHERE platform = ?1 AND platform_user_id = ?2 AND platform_chat_id = ?3
               AND is_archived = 0 LIMIT 1",
            rusqlite::params![platform, user_id, chat_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
    .ok()
}

pub async fn upsert_session(
    db: &Db,
    platform: String,
    user_id: String,
    chat_id: String,
    display_name: String,
    buddy_session_id: String,
) -> Result<(), tokio_rusqlite::Error> {
    let ts = now_secs();
    let id = format!("{platform}_{chat_id}_{user_id}");
    db.call(move |conn| {
        conn.execute(
            r#"INSERT INTO bot_sessions
               (id, platform, platform_user_id, platform_chat_id, buddy_session_id,
                display_name, created_at, last_active, is_archived)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 0)
               ON CONFLICT(id) DO UPDATE SET
                   buddy_session_id = excluded.buddy_session_id,
                   display_name     = excluded.display_name,
                   last_active      = excluded.last_active,
                   is_archived      = 0"#,
            rusqlite::params![id, platform, user_id, chat_id, buddy_session_id, display_name, ts],
        )
        .map(|_| ())
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
}

pub async fn touch_session(db: &Db, platform: String, user_id: String, chat_id: String) {
    let ts = now_secs();
    let _ = db.call(move |conn| {
        conn.execute(
            "UPDATE bot_sessions SET last_active = ?1
             WHERE platform = ?2 AND platform_user_id = ?3 AND platform_chat_id = ?4",
            rusqlite::params![ts, platform, user_id, chat_id],
        )
        .map(|_| ())
        .map_err(tokio_rusqlite::Error::from)
    })
    .await;
}

pub async fn list_active_sessions(db: &Db) -> Vec<serde_json::Value> {
    db.call(|conn| {
        let mut stmt = conn.prepare(
            "SELECT platform, platform_user_id, platform_chat_id, display_name, last_active
             FROM bot_sessions WHERE is_archived = 0 ORDER BY last_active DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "platform":      row.get::<_, String>(0)?,
                "user_id":       row.get::<_, String>(1)?,
                "chat_id":       row.get::<_, String>(2)?,
                "display_name":  row.get::<_, Option<String>>(3)?,
                "last_active":   row.get::<_, i64>(4)?,
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(rows)
    })
    .await
    .unwrap_or_default()
}

pub async fn cleanup_stale(db: &Db) {
    let now = now_secs();
    let soft = now - 30 * 86400;
    let hard = now - 90 * 86400;
    let _ = db.call(move |conn: &mut rusqlite::Connection| {
        conn.execute(
            "UPDATE bot_sessions SET is_archived = 1 WHERE last_active < ?1 AND is_archived = 0",
            rusqlite::params![soft],
        ).map(|_| ())?;
        conn.execute(
            "DELETE FROM bot_sessions WHERE is_archived = 1 AND last_active < ?1",
            rusqlite::params![hard],
        ).map(|_| ())?;
        Ok(())
    })
    .await;
}

// ── Pending confirmations ─────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct PendingConfirm {
    pub token:        String,
    pub platform:     String,
    pub chat_id:      String,
    pub user_id:      String,
    pub tool:         String,
    pub args_json:    String,
    pub prompt:       String,
    pub expires_at:   i64,
    pub prompt_state: String,
    pub prompt_nonce: i64,
}

pub async fn insert_confirm(
    db: &Db,
    token: String,
    platform: String,
    chat_id: String,
    user_id: String,
    tool: String,
    args_json: String,
    prompt: String,
    expires_at: i64,
) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute(
            r#"INSERT OR IGNORE INTO pending_confirms
               (token, platform, chat_id, user_id, tool, args_json, prompt, expires_at,
                prompt_state, prompt_nonce)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'created', 0)"#,
            rusqlite::params![token, platform, chat_id, user_id, tool, args_json, prompt, expires_at],
        )
        .map(|_| ())
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
}

pub async fn mark_prompted(db: &Db, token: String) -> Result<i64, tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute(
            "UPDATE pending_confirms
             SET prompt_state = 'prompted', prompt_nonce = prompt_nonce + 1
             WHERE token = ?1",
            rusqlite::params![token],
        )?;
        let nonce: i64 = conn.query_row(
            "SELECT prompt_nonce FROM pending_confirms WHERE token = ?1",
            rusqlite::params![token],
            |row| row.get(0),
        )?;
        Ok(nonce)
    })
    .await
}

pub async fn resolve_confirm(db: &Db, token: String) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute(
            "UPDATE pending_confirms SET prompt_state = 'resolved' WHERE token = ?1",
            rusqlite::params![token],
        )
        .map(|_| ())
        .map_err(tokio_rusqlite::Error::from)
    })
    .await
}

pub async fn load_unresolved_confirms(db: &Db) -> Vec<PendingConfirm> {
    let now = now_secs();
    db.call(move |conn| {
        let mut stmt = conn.prepare(
            r#"SELECT token, platform, chat_id, user_id, tool, args_json, prompt,
                      expires_at, prompt_state, prompt_nonce
               FROM pending_confirms
               WHERE prompt_state IN ('created', 'prompted') AND expires_at > ?1"#,
        )?;
        let rows = stmt.query_map(rusqlite::params![now], |row| {
            Ok(PendingConfirm {
                token:        row.get(0)?,
                platform:     row.get(1)?,
                chat_id:      row.get(2)?,
                user_id:      row.get(3)?,
                tool:         row.get(4)?,
                args_json:    row.get(5)?,
                prompt:       row.get(6)?,
                expires_at:   row.get(7)?,
                prompt_state: row.get(8)?,
                prompt_nonce: row.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(rows)
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> Db {
        let db = Arc::new(tokio_rusqlite::Connection::open_in_memory().await.unwrap());
        migrate(&db).await.unwrap();
        db
    }

    fn future_secs(delta: i64) -> i64 {
        now_secs() + delta
    }

    #[tokio::test]
    async fn session_create_and_find() {
        let db = mem_db().await;
        upsert_session(&db, "discord".into(), "u1".into(), "c1".into(), "User".into(), "sess-1".into()).await.unwrap();
        let found = find_active_session(&db, "discord".into(), "u1".into(), "c1".into()).await;
        assert_eq!(found, Some("sess-1".to_string()));
    }

    #[tokio::test]
    async fn session_not_found_different_platform() {
        let db = mem_db().await;
        upsert_session(&db, "discord".into(), "u2".into(), "c2".into(), "User".into(), "sess-2".into()).await.unwrap();
        let found = find_active_session(&db, "telegram".into(), "u2".into(), "c2".into()).await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn confirm_insert_mark_resolve_nonce() {
        let db = mem_db().await;
        let expires = future_secs(120);
        insert_confirm(&db, "tok1".into(), "discord".into(), "c1".into(), "u1".into(),
            "run_cmd".into(), "{}".into(), "Run rm -rf?".into(), expires).await.unwrap();

        // mark_prompted increments nonce to 1
        let nonce1 = mark_prompted(&db, "tok1".into()).await.unwrap();
        assert_eq!(nonce1, 1);

        // second mark_prompted (restart replay) increments to 2
        let nonce2 = mark_prompted(&db, "tok1".into()).await.unwrap();
        assert_eq!(nonce2, 2);

        // load_unresolved_confirms returns the row
        let pending = load_unresolved_confirms(&db).await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].token, "tok1");
        assert_eq!(pending[0].prompt_nonce, 2);

        // resolve → no longer returned by load_unresolved
        resolve_confirm(&db, "tok1".into()).await.unwrap();
        let pending2 = load_unresolved_confirms(&db).await;
        assert!(pending2.is_empty());
    }

    #[tokio::test]
    async fn expired_confirm_not_returned() {
        let db = mem_db().await;
        // Insert with expires_at in the past
        let past = now_secs() - 10;
        insert_confirm(&db, "tok-expired".into(), "discord".into(), "c1".into(), "u1".into(),
            "tool".into(), "{}".into(), "prompt".into(), past).await.unwrap();
        let pending = load_unresolved_confirms(&db).await;
        assert!(pending.is_empty(), "expired confirm should not be returned");
    }

    #[tokio::test]
    async fn stale_nonce_detect() {
        let db = mem_db().await;
        let expires = future_secs(120);
        insert_confirm(&db, "tok-stale".into(), "discord".into(), "c1".into(), "u1".into(),
            "tool".into(), "{}".into(), "prompt".into(), expires).await.unwrap();
        let nonce = mark_prompted(&db, "tok-stale".into()).await.unwrap(); // nonce = 1

        // Simulate restart: prompt sent again, nonce becomes 2
        let new_nonce = mark_prompted(&db, "tok-stale".into()).await.unwrap(); // nonce = 2

        // A Discord callback arrives with the OLD nonce (1) — must be rejected
        let pending = load_unresolved_confirms(&db).await;
        let stored = pending.iter().find(|p| p.token == "tok-stale").map(|p| p.prompt_nonce);
        assert_ne!(stored, Some(nonce), "old nonce should not match stored nonce after restart");
        assert_eq!(stored, Some(new_nonce));
    }
}

// ── Skill registry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillRecord {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub language:    String,
    pub script_path: String,
    pub version:     i64,
    pub enabled:     bool,
    pub created_at:  i64,
    pub updated_at:  i64,
}

pub async fn upsert_skill(db: &Db, rec: SkillRecord) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute(
            r#"INSERT INTO skills (id, name, description, language, script_path, version, enabled, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
               ON CONFLICT(id) DO UPDATE SET
                 name        = excluded.name,
                 description = excluded.description,
                 language    = excluded.language,
                 script_path = excluded.script_path,
                 version     = version + 1,
                 updated_at  = excluded.updated_at"#,
            rusqlite::params![
                rec.id, rec.name, rec.description, rec.language,
                rec.script_path, rec.version, rec.enabled as i64,
                rec.created_at, rec.updated_at,
            ],
        ).map(|_| ())?;
        Ok(())
    }).await
}

pub async fn list_skills(db: &Db) -> Vec<SkillRecord> {
    db.call(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, language, script_path, version, enabled, created_at, updated_at
             FROM skills ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SkillRecord {
                id:          row.get(0)?,
                name:        row.get(1)?,
                description: row.get(2)?,
                language:    row.get(3)?,
                script_path: row.get(4)?,
                version:     row.get(5)?,
                enabled:     row.get::<_, i64>(6)? != 0,
                created_at:  row.get(7)?,
                updated_at:  row.get(8)?,
            })
        })?;
        Ok(rows.flatten().collect())
    }).await.unwrap_or_default()
}

pub async fn toggle_skill(db: &Db, id: String, enabled: bool) -> Result<(), tokio_rusqlite::Error> {
    let now = now_secs();
    db.call(move |conn| {
        conn.execute(
            "UPDATE skills SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![enabled as i64, now, id],
        ).map(|_| ())?;
        Ok(())
    }).await
}

pub async fn delete_skill(db: &Db, id: String) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute("DELETE FROM skills WHERE id = ?1", rusqlite::params![id])
            .map(|_| ())?;
        Ok(())
    }).await
}

/// Scan manifest files under `base_dirs` and sync them into the skills table.
pub async fn sync_skills_from_disk(db: &Db, base_dirs: Vec<String>) {
    let now = now_secs();
    for base in &base_dirs {
        let skills_dir = std::path::PathBuf::from(base).join("skills");
        if !skills_dir.exists() { continue; }
        let entries = match std::fs::read_dir(&skills_dir) {
            Ok(e) => e, Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !fname.ends_with(".skill.json") { continue; }

            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                    let rec = SkillRecord {
                        id:          v["id"].as_str().unwrap_or_default().to_string(),
                        name:        v["name"].as_str().unwrap_or_default().to_string(),
                        description: v["description"].as_str().unwrap_or_default().to_string(),
                        language:    v["language"].as_str().unwrap_or_default().to_string(),
                        script_path: v["script_path"].as_str().unwrap_or_default().to_string(),
                        version:     1,
                        enabled:     true,
                        created_at:  now,
                        updated_at:  now,
                    };
                    if !rec.id.is_empty() {
                        let _ = upsert_skill(db, rec).await;
                    }
                }
            }
        }
    }
}

pub async fn purge_expired_confirms(db: &Db) {
    let now = now_secs();
    let stale = now - 3600;
    let _ = db.call(move |conn: &mut rusqlite::Connection| {
        conn.execute(
            "UPDATE pending_confirms SET prompt_state = 'expired'
             WHERE expires_at < ?1 AND prompt_state IN ('created', 'prompted')",
            rusqlite::params![now],
        ).map(|_| ())?;
        conn.execute(
            "DELETE FROM pending_confirms WHERE prompt_state = 'expired' AND expires_at < ?1",
            rusqlite::params![stale],
        ).map(|_| ())?;
        Ok(())
    })
    .await;
}
