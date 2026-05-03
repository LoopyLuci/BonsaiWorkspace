/// User-defined skills — macro tools and shell scripts that register as
/// first-class `Tool` trait implementations at runtime.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::tool_core::{
    ToolContext, ToolError, ToolOutput, ToolPolicyHint, ToolRegistry, ToolResult,
    RetryPolicy, SideEffectProfile,
};

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn new_id() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect()
}

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSkillRow {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub kind:        String,   // "shell" | "sequence"
    pub body:        String,
    pub tags:        String,   // JSON array of strings
    pub enabled:     bool,
    pub created_at:  i64,
    pub updated_at:  i64,
}

impl UserSkillRow {
    pub fn tags_vec(&self) -> Vec<String> {
        serde_json::from_str::<Vec<String>>(&self.tags).unwrap_or_default()
    }
}

// ── UserSkillTool — wraps a row as a Tool trait object ────────────────────────

pub struct UserSkillTool {
    row:          UserSkillRow,
    static_name:  &'static str,
    static_desc:  &'static str,
    static_tags:  &'static [&'static str],
}

impl UserSkillTool {
    pub fn new(row: UserSkillRow) -> Self {
        let static_name = crate::tool_core::intern_str(format!("skill_{}", row.name));
        let static_desc = crate::tool_core::intern_str(row.description.clone());
        let tag_vec = row.tags_vec();
        let leaked_tags: Vec<&'static str> = tag_vec
            .into_iter()
            .map(crate::tool_core::intern_str)
            .collect();
        let static_tags: &'static [&'static str] = Box::leak(leaked_tags.into_boxed_slice());

        Self { row, static_name, static_desc, static_tags }
    }
}

#[async_trait::async_trait]
impl crate::tool_core::Tool for UserSkillTool {
    fn name(&self) -> &'static str { self.static_name }
    fn description(&self) -> &'static str { self.static_desc }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "string",
                    "description": "Optional arguments or input data for the skill."
                }
            },
            "required": []
        })
    }

    fn policy_hint(&self) -> ToolPolicyHint {
        if self.row.kind == "shell" {
            ToolPolicyHint::external()
        } else {
            ToolPolicyHint::safe()
        }
    }

    fn side_effects(&self) -> SideEffectProfile {
        if self.row.kind == "shell" {
            SideEffectProfile::External
        } else {
            SideEffectProfile::None
        }
    }

    fn tags(&self) -> &'static [&'static str] { self.static_tags }

    fn retry_policy(&self) -> RetryPolicy { RetryPolicy::none() }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        match self.row.kind.as_str() {
            "shell" => {
                let body = self.row.body.clone();
                let extra_args = args.get("args").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let script = if extra_args.is_empty() {
                    body
                } else {
                    format!("{body}\n# args: {extra_args}")
                };

                let child_fut = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&script)
                    .kill_on_drop(true)
                    .output();

                match tokio::time::timeout(Duration::from_secs(30), child_fut).await {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let exit_code = output.status.code().unwrap_or(-1);
                        Ok(ToolOutput::Complete(json!({
                            "stdout": stdout,
                            "stderr": stderr,
                            "exit_code": exit_code,
                        })))
                    }
                    Ok(Err(e)) => Err(ToolError::Internal {
                        message: format!("Failed to spawn shell: {e}"),
                    }),
                    Err(_) => Err(ToolError::Timeout { duration_ms: 30_000 }),
                }
            }
            "sequence" => {
                // Parse body as JSON array of steps and return the definition.
                let steps: Vec<Value> = serde_json::from_str(&self.row.body)
                    .unwrap_or_else(|_| vec![]);
                Ok(ToolOutput::Complete(json!({
                    "steps": steps,
                    "_note": "Sequence skill definition returned. A sequencer will execute these steps."
                })))
            }
            other => Err(ToolError::Internal {
                message: format!("Unknown skill kind: {other}"),
            }),
        }
    }
}

// ── UserSkillStore ────────────────────────────────────────────────────────────

pub struct UserSkillStore {
    pool: SqlitePool,
}

impl UserSkillStore {
    pub async fn new(pool: SqlitePool) -> Result<Self, String> {
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), String> {
        // Ensure schema_migrations exists (may already exist from assistant_store)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version     INTEGER PRIMARY KEY,
                applied_at  INTEGER NOT NULL,
                description TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("user_skills migrate: {e}"))?;

        // Check if migration 100 (user_skills) is already applied
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 100",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if exists == 0 {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS user_skills (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL UNIQUE,
                    description TEXT NOT NULL,
                    kind        TEXT NOT NULL CHECK(kind IN ('shell','sequence')),
                    body        TEXT NOT NULL,
                    tags        TEXT NOT NULL DEFAULT '[]',
                    enabled     INTEGER NOT NULL DEFAULT 1,
                    created_at  INTEGER NOT NULL,
                    updated_at  INTEGER NOT NULL
                )",
            )
            .execute(&self.pool)
            .await
            .map_err(|e| format!("create user_skills table: {e}"))?;

            sqlx::query(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (100, ?, 'user_skills')",
            )
            .bind(now())
            .execute(&self.pool)
            .await
            .map_err(|e| format!("record user_skills migration: {e}"))?;
        }

        Ok(())
    }

    pub async fn list_enabled(&self) -> Result<Vec<UserSkillRow>, String> {
        let rows = sqlx::query(
            "SELECT id,name,description,kind,body,tags,enabled,created_at,updated_at
             FROM user_skills WHERE enabled = 1 ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_enabled skills: {e}"))?;

        Ok(rows.iter().map(row_to_skill).collect())
    }

    pub async fn list_all(&self) -> Result<Vec<UserSkillRow>, String> {
        let rows = sqlx::query(
            "SELECT id,name,description,kind,body,tags,enabled,created_at,updated_at
             FROM user_skills ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("list_all skills: {e}"))?;

        Ok(rows.iter().map(row_to_skill).collect())
    }

    pub async fn upsert(&self, row: &UserSkillRow) -> Result<(), String> {
        let ts = now();
        let id = if row.id.is_empty() { new_id() } else { row.id.clone() };
        let created_at = if row.created_at == 0 { ts } else { row.created_at };

        sqlx::query(
            "INSERT INTO user_skills (id,name,description,kind,body,tags,enabled,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, description=excluded.description,
               kind=excluded.kind, body=excluded.body,
               tags=excluded.tags, enabled=excluded.enabled,
               updated_at=excluded.updated_at",
        )
        .bind(&id)
        .bind(&row.name)
        .bind(&row.description)
        .bind(&row.kind)
        .bind(&row.body)
        .bind(&row.tags)
        .bind(row.enabled as i64)
        .bind(created_at)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("upsert skill: {e}"))?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM user_skills WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("delete skill: {e}"))?;
        Ok(())
    }

    /// Load all enabled skills into the registry; returns count registered.
    pub async fn load_into_registry(&self, registry: &mut ToolRegistry) -> Result<usize, String> {
        let skills = self.list_enabled().await?;
        let count = skills.len();
        for row in skills {
            let tool = UserSkillTool::new(row);
            registry.register(tool);
        }
        Ok(count)
    }
}

// ── Row mapper ────────────────────────────────────────────────────────────────

fn row_to_skill(r: &sqlx::sqlite::SqliteRow) -> UserSkillRow {
    UserSkillRow {
        id:          r.get("id"),
        name:        r.get("name"),
        description: r.get("description"),
        kind:        r.get("kind"),
        body:        r.get("body"),
        tags:        r.get("tags"),
        enabled:     r.get::<i64, _>("enabled") != 0,
        created_at:  r.get("created_at"),
        updated_at:  r.get("updated_at"),
    }
}

