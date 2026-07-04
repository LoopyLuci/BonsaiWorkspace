//! Persistent SQLite store for `ModelData`.
//!
//! Each row serializes its nested structs as JSON columns — keeps the schema
//! simple and avoids migrations when we add fields to the Rust types.
//!
//! Uses the same `SqlitePool` as every other store in the WAL.

use anyhow::Result;
use serde_json;
use sqlx::SqlitePool;

use crate::inference_mode::InferenceMode;
use crate::model_data::{AffinityLevel, ModelData, ModelDataSummary};

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct ModelDataStore {
    pool: SqlitePool,
}

impl ModelDataStore {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_data (
                id                   TEXT    PRIMARY KEY,
                name                 TEXT    NOT NULL,
                family               TEXT,
                version              TEXT,
                description          TEXT    NOT NULL DEFAULT '',
                source_json          TEXT    NOT NULL,
                capabilities_json    TEXT    NOT NULL,
                inference_json       TEXT    NOT NULL,
                inference_mode_json  TEXT    NOT NULL DEFAULT '{"mode":"hybrid","gpu_layers":20}',
                prompt_format_json   TEXT    NOT NULL,
                skill_affinities_json TEXT   NOT NULL DEFAULT '[]',
                authors_json         TEXT    NOT NULL DEFAULT '[]',
                organization         TEXT,
                license              TEXT,
                homepage_url         TEXT,
                training_cutoff      TEXT,
                parameter_count      INTEGER,
                architecture         TEXT,
                tags_json            TEXT    NOT NULL DEFAULT '[]',
                notes                TEXT    NOT NULL DEFAULT '',
                local_file_json      TEXT,
                created_at           INTEGER NOT NULL,
                updated_at           INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_model_data_name       ON model_data(name);
            CREATE INDEX IF NOT EXISTS idx_model_data_updated_at ON model_data(updated_at DESC);
        "#,
        )
        .execute(&pool)
        .await?;

        // Backward-compatible migration for existing databases.
        let _ = sqlx::query(
            "ALTER TABLE model_data ADD COLUMN inference_mode_json TEXT NOT NULL DEFAULT '{\"mode\":\"hybrid\",\"gpu_layers\":20}'",
        )
        .execute(&pool)
        .await;

        Ok(Self { pool })
    }

    // ── CRUD ──────────────────────────────────────────────────────────────────

    pub async fn get(&self, id: &str) -> Result<Option<ModelData>> {
        let row = sqlx::query_as::<_, ModelDataRow>("SELECT * FROM model_data WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|r| r.into_model_data()).transpose()
    }

    pub async fn list(&self) -> Result<Vec<ModelData>> {
        let rows =
            sqlx::query_as::<_, ModelDataRow>("SELECT * FROM model_data ORDER BY updated_at DESC")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter().map(|r| r.into_model_data()).collect()
    }

    pub async fn list_summaries(&self) -> Result<Vec<ModelDataSummary>> {
        Ok(self
            .list()
            .await?
            .iter()
            .map(ModelDataSummary::from)
            .collect())
    }

    pub async fn save(&self, data: &ModelData) -> Result<()> {
        let source_json = serde_json::to_string(&data.source)?;
        let capabilities_json = serde_json::to_string(&data.capabilities)?;
        let inference_json = serde_json::to_string(&data.inference)?;
        let inference_mode_json = serde_json::to_string(&data.inference_mode)?;
        let prompt_format_json = serde_json::to_string(&data.prompt_format)?;
        let skill_affinities_json = serde_json::to_string(&data.skill_affinities)?;
        let authors_json = serde_json::to_string(&data.authors)?;
        let tags_json = serde_json::to_string(&data.tags)?;
        let local_file_json = data
            .local_file
            .as_ref()
            .map(|lf| serde_json::to_string(lf))
            .transpose()?;

        sqlx::query(r#"
            INSERT INTO model_data (
                id, name, family, version, description,
                source_json, capabilities_json, inference_json, inference_mode_json, prompt_format_json,
                skill_affinities_json, authors_json, organization, license,
                homepage_url, training_cutoff, parameter_count, architecture,
                tags_json, notes, local_file_json, created_at, updated_at
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            ON CONFLICT(id) DO UPDATE SET
                name                  = excluded.name,
                family                = excluded.family,
                version               = excluded.version,
                description           = excluded.description,
                source_json           = excluded.source_json,
                capabilities_json     = excluded.capabilities_json,
                inference_json        = excluded.inference_json,
                inference_mode_json   = excluded.inference_mode_json,
                prompt_format_json    = excluded.prompt_format_json,
                skill_affinities_json = excluded.skill_affinities_json,
                authors_json          = excluded.authors_json,
                organization          = excluded.organization,
                license               = excluded.license,
                homepage_url          = excluded.homepage_url,
                training_cutoff       = excluded.training_cutoff,
                parameter_count       = excluded.parameter_count,
                architecture          = excluded.architecture,
                tags_json             = excluded.tags_json,
                notes                 = excluded.notes,
                local_file_json       = excluded.local_file_json,
                updated_at            = excluded.updated_at
        "#)
        .bind(&data.id)
        .bind(&data.name)
        .bind(&data.family)
        .bind(&data.version)
        .bind(&data.description)
        .bind(&source_json)
        .bind(&capabilities_json)
        .bind(&inference_json)
        .bind(&inference_mode_json)
        .bind(&prompt_format_json)
        .bind(&skill_affinities_json)
        .bind(&authors_json)
        .bind(&data.organization)
        .bind(&data.license)
        .bind(&data.homepage_url)
        .bind(&data.training_cutoff)
        .bind(data.parameter_count.map(|n| n as i64))
        .bind(&data.architecture)
        .bind(&tags_json)
        .bind(&data.notes)
        .bind(&local_file_json)
        .bind(data.created_at)
        .bind(data.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM model_data WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Find all models whose source links to a given registry (file-hash) ID.
    pub async fn find_by_registry_id(&self, registry_id: &str) -> Result<Option<ModelData>> {
        // The registry_id lives inside source_json — use a JSON-extract approach.
        // SQLite's json_extract works on TEXT columns.
        let row = sqlx::query_as::<_, ModelDataRow>(
            "SELECT * FROM model_data WHERE json_extract(source_json, '$.registry_id') = ?",
        )
        .bind(registry_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.into_model_data()).transpose()
    }

    /// Find models that have an `Excellent` or `Good` affinity for a given skill.
    pub async fn find_for_skill(&self, skill_id: &str) -> Result<Vec<ModelData>> {
        // Filter in Rust after fetch — skill_affinities_json is an array.
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|m| {
                m.skill_affinities.iter().any(|a| {
                    a.skill_id == skill_id
                        && matches!(a.level, AffinityLevel::Excellent | AffinityLevel::Good)
                })
            })
            .collect())
    }

    /// Return all models sorted best-first for a given skill (Excellent → Good → rest).
    pub async fn rank_for_skill(&self, skill_id: &str) -> Result<Vec<ModelData>> {
        let mut all = self.list().await?;
        all.sort_by_key(|m| {
            m.skill_affinities
                .iter()
                .find(|a| a.skill_id == skill_id)
                .map(|a| match a.level {
                    AffinityLevel::Excellent => 0,
                    AffinityLevel::Good => 1,
                    AffinityLevel::Fair => 2,
                    AffinityLevel::Poor => 3,
                    AffinityLevel::Incompatible => 4,
                })
                .unwrap_or(5)
        });
        Ok(all)
    }

    /// Simple text search across name, description, family, tags, and notes.
    pub async fn search(&self, query: &str) -> Result<Vec<ModelData>> {
        let q = query.to_lowercase();
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.family
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
                    || m.notes.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect())
    }

    /// Ensure all local GGUF registry models have a ModelData entry.
    /// Creates skeleton entries for any that are missing and returns the count created.
    pub async fn sync_from_registry(
        &self,
        models: &[crate::model_registry::ModelInfo],
        default_inference_mode: &InferenceMode,
    ) -> Result<usize> {
        let mut created = 0usize;
        for info in models {
            if self.find_by_registry_id(&info.id).await?.is_none() {
                let data = ModelData::from_registry_with_mode(info, default_inference_mode.clone());
                self.save(&data).await?;
                created += 1;
            }
        }
        Ok(created)
    }

    pub async fn count(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM model_data")
            .fetch_one(&self.pool)
            .await?)
    }
}

// ── SQLx row ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ModelDataRow {
    id: String,
    name: String,
    family: Option<String>,
    version: Option<String>,
    description: String,
    source_json: String,
    capabilities_json: String,
    inference_json: String,
    inference_mode_json: Option<String>,
    prompt_format_json: String,
    skill_affinities_json: String,
    authors_json: String,
    organization: Option<String>,
    license: Option<String>,
    homepage_url: Option<String>,
    training_cutoff: Option<String>,
    parameter_count: Option<i64>,
    architecture: Option<String>,
    tags_json: String,
    notes: String,
    local_file_json: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl ModelDataRow {
    fn into_model_data(self) -> Result<ModelData> {
        Ok(ModelData {
            id: self.id,
            name: self.name,
            family: self.family,
            version: self.version,
            description: self.description,
            source: serde_json::from_str(&self.source_json)?,
            capabilities: serde_json::from_str(&self.capabilities_json)?,
            inference: serde_json::from_str(&self.inference_json)?,
            inference_mode: self
                .inference_mode_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default(),
            prompt_format: serde_json::from_str(&self.prompt_format_json)?,
            skill_affinities: serde_json::from_str(&self.skill_affinities_json)?,
            authors: serde_json::from_str(&self.authors_json)?,
            organization: self.organization,
            license: self.license,
            homepage_url: self.homepage_url,
            training_cutoff: self.training_cutoff,
            parameter_count: self.parameter_count.map(|n| n as u64),
            architecture: self.architecture,
            tags: serde_json::from_str(&self.tags_json)?,
            notes: self.notes,
            local_file: self
                .local_file_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
