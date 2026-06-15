use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRun {
    pub id: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub base_model: String,
    pub data_path: Option<String>,
    pub adapter_path: Option<String>,
    pub status: String,
    pub metrics: Option<String>,
    pub total_examples: Option<i64>,
    pub curated_examples: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRecord {
    pub model_id: String,
    pub adapter_id: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub latency_ms: i64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceStats {
    pub total_requests: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub window_hours: u32,
}

// ── Store ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TelemetryStore {
    pool: SqlitePool,
}

impl TelemetryStore {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(&format!("sqlite:{db_path}?mode=rwc")).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS training_runs (
                id               TEXT PRIMARY KEY,
                started_at       INTEGER NOT NULL,
                finished_at      INTEGER,
                base_model       TEXT NOT NULL,
                data_path        TEXT,
                adapter_path     TEXT,
                status           TEXT NOT NULL DEFAULT 'running',
                metrics          TEXT,
                total_examples   INTEGER,
                curated_examples INTEGER
            );
            CREATE TABLE IF NOT EXISTS inference_telemetry (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp         INTEGER NOT NULL,
                model_id          TEXT NOT NULL,
                adapter_id        TEXT,
                prompt_tokens     INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms        INTEGER NOT NULL DEFAULT 0,
                success           INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_inf_ts ON inference_telemetry(timestamp);",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    // ── Training runs ─────────────────────────────────────────────────────────

    pub async fn log_training_start(&self, run: &TrainingRun) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO training_runs
             (id, started_at, base_model, data_path, status, total_examples, curated_examples)
             VALUES (?, ?, ?, ?, 'running', ?, ?)",
        )
        .bind(&run.id)
        .bind(run.started_at)
        .bind(&run.base_model)
        .bind(&run.data_path)
        .bind(run.total_examples)
        .bind(run.curated_examples)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn log_training_end(
        &self,
        id: &str,
        status: &str,
        metrics: Option<&str>,
        adapter_path: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE training_runs
             SET finished_at = ?, status = ?, metrics = ?, adapter_path = ?
             WHERE id = ?",
        )
        .bind(now_secs())
        .bind(status)
        .bind(metrics)
        .bind(adapter_path)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_training_runs(&self, limit: i64) -> Result<Vec<TrainingRun>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, started_at, finished_at, base_model, data_path, adapter_path,
                    status, metrics, total_examples, curated_examples
             FROM training_runs ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| TrainingRun {
                id: r.get("id"),
                started_at: r.get("started_at"),
                finished_at: r.get("finished_at"),
                base_model: r.get("base_model"),
                data_path: r.get("data_path"),
                adapter_path: r.get("adapter_path"),
                status: r.get("status"),
                metrics: r.get("metrics"),
                total_examples: r.get("total_examples"),
                curated_examples: r.get("curated_examples"),
            })
            .collect())
    }

    pub async fn get_training_run(&self, id: &str) -> Result<Option<TrainingRun>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, started_at, finished_at, base_model, data_path, adapter_path,
                    status, metrics, total_examples, curated_examples
             FROM training_runs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| TrainingRun {
            id: r.get("id"),
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            base_model: r.get("base_model"),
            data_path: r.get("data_path"),
            adapter_path: r.get("adapter_path"),
            status: r.get("status"),
            metrics: r.get("metrics"),
            total_examples: r.get("total_examples"),
            curated_examples: r.get("curated_examples"),
        }))
    }

    pub async fn get_latest_run(&self) -> Result<Option<TrainingRun>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, started_at, finished_at, base_model, data_path, adapter_path,
                    status, metrics, total_examples, curated_examples
             FROM training_runs ORDER BY started_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| TrainingRun {
            id: r.get("id"),
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            base_model: r.get("base_model"),
            data_path: r.get("data_path"),
            adapter_path: r.get("adapter_path"),
            status: r.get("status"),
            metrics: r.get("metrics"),
            total_examples: r.get("total_examples"),
            curated_examples: r.get("curated_examples"),
        }))
    }

    // ── Inference telemetry ───────────────────────────────────────────────────

    pub async fn log_inference(&self, rec: &InferenceRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO inference_telemetry
             (timestamp, model_id, adapter_id, prompt_tokens, completion_tokens, latency_ms, success)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(now_secs())
        .bind(&rec.model_id)
        .bind(&rec.adapter_id)
        .bind(rec.prompt_tokens)
        .bind(rec.completion_tokens)
        .bind(rec.latency_ms)
        .bind(rec.success as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_inference_stats(&self, hours: u32) -> Result<InferenceStats, sqlx::Error> {
        let since = now_secs() - (hours as i64 * 3600);
        let row = sqlx::query(
            "SELECT
                COUNT(*)                          AS total,
                AVG(CAST(success AS REAL))        AS success_rate,
                AVG(latency_ms)                   AS avg_latency,
                COALESCE(SUM(prompt_tokens), 0)   AS prompt_total,
                COALESCE(SUM(completion_tokens),0) AS completion_total
             FROM inference_telemetry WHERE timestamp >= ?",
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        Ok(InferenceStats {
            total_requests: row.get::<i64, _>("total"),
            success_rate: row.get::<f64, _>("success_rate"),
            avg_latency_ms: row.get::<f64, _>("avg_latency"),
            total_prompt_tokens: row.get::<i64, _>("prompt_total"),
            total_completion_tokens: row.get::<i64, _>("completion_total"),
            window_hours: hours,
        })
    }
}

// ── Convenience: start a new training run with a generated ID ─────────────────

pub fn new_run_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("run_{ts}")
}
