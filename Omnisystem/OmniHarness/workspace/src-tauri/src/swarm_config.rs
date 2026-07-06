//! Custom swarm configuration store — SQLite-backed CRUD.
//!
//! Lets users define, name, and persist custom multi-agent topologies.
//! Configurations are loaded by `swarm_orchestrator` at runtime and can be
//! assembled dynamically by `smart_router`.

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRole {
    Implementer,
    Reviewer,
    Tester,
    Researcher,
    Skeptic,
    Synthesizer,
    #[serde(untagged)]
    Custom(String),
}

impl WorkerRole {
    pub fn system_hint(&self) -> &str {
        match self {
            Self::Implementer => "You are an implementer. Write clean, correct code or prose.",
            Self::Reviewer => "You are a reviewer. Identify bugs, risks, and improvements.",
            Self::Tester => "You are a tester. Design test cases and find edge cases.",
            Self::Researcher => "You are a researcher. Find relevant facts and cite evidence.",
            Self::Skeptic => "You are a skeptic. Challenge assumptions and find flaws.",
            Self::Synthesizer => {
                "You are a synthesizer. Combine worker outputs into a cohesive response."
            }
            Self::Custom(_) => "You are a specialized agent. Follow the task instructions.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChainStrategy {
    /// All workers run in parallel, leader synthesizes outputs.
    ParallelThenSynthesize,
    /// Workers run sequentially; each gates on the previous result.
    SequentialGate,
    /// Workers run in parallel, majority vote determines output.
    ParallelVote,
    /// Implement → Review → Test → Synthesize fixed pipeline.
    DevPipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmWorkerConfig {
    /// Slot index (0-based). Multiple workers can share a slot.
    pub slot: usize,
    pub role: WorkerRole,
    /// Optional override: specific model to load for this worker.
    /// `None` = use whatever is currently loaded in the slot.
    pub model: Option<String>,
    /// Optional LoRA adapter name.
    pub adapter: Option<String>,
    /// GPU layer override (-1 = inherit from orchestrator).
    pub gpu_layers: i32,
    /// Override system prompt for this worker (appended after role hint).
    pub system_prompt: Option<String>,
    /// If set, worker only receives these tools.
    pub allowed_tools: Option<Vec<String>>,
    /// Scheduling priority (higher = runs first in parallel sets).
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workers: Vec<SwarmWorkerConfig>,
    pub chain_strategy: ChainStrategy,
    /// Model used for final synthesis / leader planning.
    pub synthesis_model: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct SwarmConfigStore {
    pool: SqlitePool,
}

impl SwarmConfigStore {
    pub async fn new(pool: SqlitePool) -> anyhow::Result<Arc<Self>> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS swarm_configs (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                description    TEXT NOT NULL DEFAULT '',
                config_json    TEXT NOT NULL,
                enabled        INTEGER NOT NULL DEFAULT 1,
                created_at     TEXT NOT NULL,
                last_used_at   TEXT
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Arc::new(Self { pool }))
    }

    pub async fn create(&self, mut cfg: SwarmConfig) -> anyhow::Result<SwarmConfig> {
        cfg.id = Uuid::new_v4().to_string();
        cfg.created_at = Utc::now().to_rfc3339();
        cfg.last_used_at = None;
        let json = serde_json::to_string(&cfg)?;

        sqlx::query(
            "INSERT INTO swarm_configs (id, name, description, config_json, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&cfg.id)
        .bind(&cfg.name)
        .bind(&cfg.description)
        .bind(&json)
        .bind(cfg.enabled as i32)
        .bind(&cfg.created_at)
        .execute(&self.pool)
        .await?;

        Ok(cfg)
    }

    pub async fn list(&self) -> anyhow::Result<Vec<SwarmConfig>> {
        let rows = sqlx::query("SELECT config_json FROM swarm_configs ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|r| {
                let json: String = r.try_get("config_json")?;
                Ok(serde_json::from_str(&json)?)
            })
            .collect()
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<SwarmConfig>> {
        let row = sqlx::query("SELECT config_json FROM swarm_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(None),
            Some(r) => {
                let json: String = r.try_get("config_json")?;
                Ok(Some(serde_json::from_str(&json)?))
            }
        }
    }

    pub async fn update(&self, cfg: &SwarmConfig) -> anyhow::Result<()> {
        let json = serde_json::to_string(cfg)?;
        sqlx::query(
            "UPDATE swarm_configs SET name=?, description=?, config_json=?, enabled=? WHERE id=?",
        )
        .bind(&cfg.name)
        .bind(&cfg.description)
        .bind(&json)
        .bind(cfg.enabled as i32)
        .bind(&cfg.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM swarm_configs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_last_used(&self, id: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE swarm_configs SET last_used_at=? WHERE id=?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_swarm_config(
    cfg: SwarmConfig,
    state: tauri::State<'_, crate::AppState>,
) -> Result<SwarmConfig, String> {
    state
        .swarm_config_store
        .create(cfg)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_swarm_configs(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<SwarmConfig>, String> {
    state
        .swarm_config_store
        .list()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_swarm_config(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<SwarmConfig>, String> {
    state
        .swarm_config_store
        .get(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_swarm_config(
    cfg: SwarmConfig,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .swarm_config_store
        .update(&cfg)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_swarm_config(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    state
        .swarm_config_store
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Apply a template's workers to the live agent topology that
/// `swarm_orchestrator`/`agent_store` actually run — this used to be the
/// missing half of "activate": the command only fetched the config and
/// touched a timestamp, with the doc comment's claim that "configurations
/// are loaded by swarm_orchestrator at runtime" not backed by any real code
/// path (nothing there ever read from `SwarmConfigStore`).
///
/// Each worker gets an auto-generated `Persona` (role hint + optional
/// override system prompt) so the existing `resolve_agents()` persona
/// resolution — already used by the live Custom Swarm feature — picks it up
/// unchanged. Activation is a clean replace: existing agent configs are
/// removed first, since an additive merge could leave stale/duplicate slots
/// behind from whatever topology was live before.
async fn apply_swarm_config_to_agents(
    cfg: &SwarmConfig,
    state: &crate::AppState,
) -> anyhow::Result<()> {
    let existing = state.agent_store.list_agents().await?;
    for agent in &existing {
        state.agent_store.delete_agent(&agent.id).await?;
    }

    for worker in &cfg.workers {
        let now = Utc::now().timestamp();
        let role_name = match &worker.role {
            WorkerRole::Custom(name) => name.clone(),
            other => format!("{other:?}"),
        };
        let system_prompt = match &worker.system_prompt {
            Some(extra) => format!("{}\n\n{extra}", worker.role.system_hint()),
            None => worker.role.system_hint().to_string(),
        };

        let persona_id = Uuid::new_v4().to_string();
        state
            .agent_store
            .upsert_persona(crate::agent_store::Persona {
                id: persona_id.clone(),
                name: format!("{role_name} (from {})", cfg.name),
                system_prompt,
                model_id: worker.model.clone(),
                color: "#4a9eff".to_string(),
                icon_emoji: "🧩".to_string(),
                created_at: now,
                updated_at: now,
            })
            .await?;

        state
            .agent_store
            .upsert_agent(crate::agent_store::AgentConfig {
                id: Uuid::new_v4().to_string(),
                slot_index: worker.slot as i64,
                label: role_name,
                persona_id: Some(persona_id),
                model_id: worker.model.clone(),
                color: "#4a9eff".to_string(),
                icon_emoji: "🧩".to_string(),
                enabled: true,
                max_tokens: 4096,
                created_at: now,
                updated_at: now,
            })
            .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn activate_swarm(
    id: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<SwarmConfig, String> {
    let cfg = state
        .swarm_config_store
        .get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Swarm config '{id}' not found"))?;

    apply_swarm_config_to_agents(&cfg, &state)
        .await
        .map_err(|e| e.to_string())?;

    let _ = state.swarm_config_store.touch_last_used(&id).await;
    Ok(cfg)
}

#[tauri::command]
pub async fn arena_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::shared_arena::ArenaStats, String> {
    Ok(state.shared_arena.stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_role_wire_format() {
        assert_eq!(serde_json::to_string(&WorkerRole::Implementer).unwrap(), "\"implementer\"");
        assert_eq!(serde_json::to_string(&WorkerRole::Custom("Foo Bar".into())).unwrap(), "\"Foo Bar\"");
        let parsed: WorkerRole = serde_json::from_str("\"reviewer\"").unwrap();
        assert_eq!(parsed, WorkerRole::Reviewer);
        let parsed: WorkerRole = serde_json::from_str("\"my-custom-role\"").unwrap();
        assert_eq!(parsed, WorkerRole::Custom("my-custom-role".into()));
    }
}
