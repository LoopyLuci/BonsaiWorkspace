use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::model_orchestrator::ModelOrchestrator;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub model_id: Option<String>,
    pub color: String,
    pub icon_emoji: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub slot_index: i64,
    pub label: String,
    pub persona_id: Option<String>,
    pub model_id: Option<String>,
    pub color: String,
    pub icon_emoji: String,
    pub enabled: bool,
    pub max_tokens: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAgent {
    pub config: AgentConfig,
    pub persona: Option<Persona>,
    pub system_prompt: String,
    pub effective_model_id: Option<String>,
    pub context_length: u32,
    pub supports_tools: bool,
    pub ram_required_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwarmRun {
    pub id: String,
    pub session_id: Option<String>,
    pub user_prompt: String,
    pub leader_plan: Option<String>,
    pub summary: Option<String>,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwarmAgentResult {
    pub id: String,
    pub swarm_run_id: String,
    pub agent_id: String,
    pub agent_slot: i64,
    pub subtask: String,
    pub result: Option<String>,
    pub stats_json: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct AgentStore {
    pool: SqlitePool,
}

impl AgentStore {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS personas (
                id            TEXT    PRIMARY KEY,
                name          TEXT    NOT NULL,
                system_prompt TEXT    NOT NULL,
                model_id      TEXT,
                color         TEXT    NOT NULL DEFAULT '#4a9eff',
                icon_emoji    TEXT    NOT NULL DEFAULT '🤖',
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_configs (
                id           TEXT    PRIMARY KEY,
                slot_index   INTEGER NOT NULL UNIQUE,
                label        TEXT    NOT NULL,
                persona_id   TEXT    REFERENCES personas(id) ON DELETE SET NULL,
                model_id     TEXT,
                color        TEXT    NOT NULL DEFAULT '#4a9eff',
                icon_emoji   TEXT    NOT NULL DEFAULT '🤖',
                enabled      INTEGER NOT NULL DEFAULT 1,
                max_tokens   INTEGER NOT NULL DEFAULT 4096,
                created_at   INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_agent_configs_slot ON agent_configs(slot_index);

            CREATE TABLE IF NOT EXISTS swarm_runs (
                id           TEXT    PRIMARY KEY,
                session_id   TEXT,
                user_prompt  TEXT    NOT NULL,
                leader_plan  TEXT,
                status       TEXT    NOT NULL DEFAULT 'running',
                started_at   INTEGER NOT NULL,
                completed_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS swarm_agent_results (
                id           TEXT    PRIMARY KEY,
                swarm_run_id TEXT    NOT NULL REFERENCES swarm_runs(id) ON DELETE CASCADE,
                agent_id     TEXT    NOT NULL,
                agent_slot   INTEGER NOT NULL,
                subtask      TEXT    NOT NULL,
                result       TEXT,
                stats_json   TEXT,
                started_at   INTEGER NOT NULL,
                completed_at INTEGER
            );
        "#,
        )
        .execute(&pool)
        .await?;

        // Add agent_id column to session_messages if not present
        let _ = sqlx::query("ALTER TABLE session_messages ADD COLUMN agent_id TEXT")
            .execute(&pool)
            .await;

        let _ =
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_msgs_agent ON session_messages(agent_id)")
                .execute(&pool)
                .await;

        let _ = sqlx::query("ALTER TABLE swarm_runs ADD COLUMN summary TEXT")
            .execute(&pool)
            .await;

        // Seed Leader if table is empty
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_configs")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

        if count == 0 {
            let now = unix_now();
            sqlx::query(
                "INSERT INTO agent_configs (id, slot_index, label, persona_id, model_id, color, icon_emoji, enabled, max_tokens, created_at, updated_at)
                 VALUES (?, 0, 'Leader', NULL, NULL, '#f5a623', '👑', 1, 4096, ?, ?)",
            )
            .bind(uuid())
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await?;
        }

        Ok(Self { pool })
    }

    // ── Agents ────────────────────────────────────────────────────────────────

    pub async fn list_agents(&self) -> Result<Vec<AgentConfig>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, slot_index, label, persona_id, model_id, color, icon_emoji, enabled, max_tokens, created_at, updated_at
             FROM agent_configs ORDER BY slot_index",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| AgentConfig {
                id: r.get("id"),
                slot_index: r.get("slot_index"),
                label: r.get("label"),
                persona_id: r.get("persona_id"),
                model_id: r.get("model_id"),
                color: r.get("color"),
                icon_emoji: r.get("icon_emoji"),
                enabled: r.get::<i64, _>("enabled") != 0,
                max_tokens: r.get("max_tokens"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn resolve_agents(
        &self,
        orchestrator: &ModelOrchestrator,
    ) -> Result<Vec<ResolvedAgent>> {
        let agents = self.list_agents().await?;
        let personas = self.list_personas().await?;
        let models = orchestrator.list_models().await;

        let mut resolved = Vec::new();
        for cfg in agents {
            let persona = cfg
                .persona_id
                .as_deref()
                .and_then(|pid| personas.iter().find(|p| p.id == pid))
                .cloned();

            let system_prompt = persona
                .as_ref()
                .map(|p| p.system_prompt.clone())
                .unwrap_or_default();

            let effective_model_id = cfg
                .model_id
                .clone()
                .or_else(|| persona.as_ref().and_then(|p| p.model_id.clone()));

            let model_info = effective_model_id
                .as_deref()
                .and_then(|mid| models.iter().find(|m| m.id == mid));

            let ram_required_mb = model_info.map(|m| m.ram_required_mb).unwrap_or(0);

            let context_length = model_info.map(|m| m.context_length).unwrap_or(4096);

            let supports_tools = model_info.map(|m| m.supports_tools).unwrap_or(false);

            resolved.push(ResolvedAgent {
                config: cfg,
                persona,
                system_prompt,
                effective_model_id,
                context_length,
                supports_tools,
                ram_required_mb,
            });
        }
        Ok(resolved)
    }

    pub async fn upsert_agent(&self, cfg: AgentConfig) -> Result<AgentConfig> {
        let now = unix_now();
        sqlx::query(
            "INSERT INTO agent_configs (id, slot_index, label, persona_id, model_id, color, icon_emoji, enabled, max_tokens, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               slot_index=excluded.slot_index, label=excluded.label, persona_id=excluded.persona_id,
               model_id=excluded.model_id, color=excluded.color, icon_emoji=excluded.icon_emoji,
               enabled=excluded.enabled, max_tokens=excluded.max_tokens, updated_at=excluded.updated_at",
        )
        .bind(&cfg.id)
        .bind(cfg.slot_index)
        .bind(&cfg.label)
        .bind(&cfg.persona_id)
        .bind(&cfg.model_id)
        .bind(&cfg.color)
        .bind(&cfg.icon_emoji)
        .bind(cfg.enabled as i64)
        .bind(cfg.max_tokens)
        .bind(cfg.created_at)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(cfg)
    }

    pub async fn delete_agent(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM agent_configs WHERE id=? AND slot_index != 0")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Personas ──────────────────────────────────────────────────────────────

    pub async fn list_personas(&self) -> Result<Vec<Persona>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, name, system_prompt, model_id, color, icon_emoji, created_at, updated_at FROM personas ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| Persona {
                id: r.get("id"),
                name: r.get("name"),
                system_prompt: r.get("system_prompt"),
                model_id: r.get("model_id"),
                color: r.get("color"),
                icon_emoji: r.get("icon_emoji"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn upsert_persona(&self, p: Persona) -> Result<Persona> {
        let now = unix_now();
        sqlx::query(
            "INSERT INTO personas (id, name, system_prompt, model_id, color, icon_emoji, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, system_prompt=excluded.system_prompt, model_id=excluded.model_id,
               color=excluded.color, icon_emoji=excluded.icon_emoji, updated_at=excluded.updated_at",
        )
        .bind(&p.id)
        .bind(&p.name)
        .bind(&p.system_prompt)
        .bind(&p.model_id)
        .bind(&p.color)
        .bind(&p.icon_emoji)
        .bind(p.created_at)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(p)
    }

    pub async fn delete_persona(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM personas WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Swarm persistence ─────────────────────────────────────────────────────

    pub async fn save_swarm_run(&self, run: &SwarmRun) -> Result<()> {
        sqlx::query(
            "INSERT INTO swarm_runs (id, session_id, user_prompt, leader_plan, summary, status, started_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               leader_plan=excluded.leader_plan, summary=excluded.summary, status=excluded.status, completed_at=excluded.completed_at",
        )
        .bind(&run.id)
        .bind(&run.session_id)
        .bind(&run.user_prompt)
        .bind(&run.leader_plan)
        .bind(&run.summary)
        .bind(&run.status)
        .bind(run.started_at)
        .bind(run.completed_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn recent_completed_swarm_runs(&self, limit: usize) -> Result<Vec<SwarmRun>> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT id, session_id, user_prompt, leader_plan, summary, status, started_at, completed_at
             FROM swarm_runs
             WHERE status = 'completed' AND summary IS NOT NULL AND TRIM(summary) <> ''
             ORDER BY completed_at DESC, started_at DESC
             LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SwarmRun {
                id: r.get("id"),
                session_id: r.get("session_id"),
                user_prompt: r.get("user_prompt"),
                leader_plan: r.get("leader_plan"),
                summary: r.get("summary"),
                status: r.get("status"),
                started_at: r.get("started_at"),
                completed_at: r.get("completed_at"),
            })
            .collect())
    }

    pub async fn save_agent_result(&self, r: &SwarmAgentResult) -> Result<()> {
        sqlx::query(
            "INSERT INTO swarm_agent_results (id, swarm_run_id, agent_id, agent_slot, subtask, result, stats_json, started_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET result=excluded.result, stats_json=excluded.stats_json, completed_at=excluded.completed_at",
        )
        .bind(&r.id)
        .bind(&r.swarm_run_id)
        .bind(&r.agent_id)
        .bind(r.agent_slot)
        .bind(&r.subtask)
        .bind(&r.result)
        .bind(&r.stats_json)
        .bind(r.started_at)
        .bind(r.completed_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn uuid() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}
