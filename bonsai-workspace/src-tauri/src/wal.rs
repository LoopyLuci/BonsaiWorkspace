use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;
use tauri::AppHandle;

/// Write-Ahead Log: persists every significant event to SQLite for crash recovery and audit.
#[allow(clippy::upper_case_acronyms)]
pub struct WAL {
    pool: SqlitePool,
}

impl WAL {
    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    pub async fn new(app_handle: &AppHandle) -> Result<Self> {
        use tauri::Manager;
        let app_data_dir = app_handle.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("bonsai.db");

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wal_events (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp    TEXT    NOT NULL DEFAULT (datetime('now')),
                event_type   TEXT    NOT NULL,
                payload_json TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_wal_timestamp ON wal_events(timestamp);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn log_event(&self, event_type: &str, payload: Value) -> Result<()> {
        sqlx::query("INSERT INTO wal_events (event_type, payload_json) VALUES (?, ?)")
            .bind(event_type)
            .bind(payload.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn recent_events(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        // Use query() (runtime) rather than query!() (compile-time) to avoid
        // requiring a live database during `cargo build`.
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, timestamp, event_type, payload_json \
             FROM wal_events ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id":         r.get::<i64,    _>("id"),
                    "timestamp":  r.get::<String, _>("timestamp"),
                    "event_type": r.get::<String, _>("event_type"),
                    "payload":    r.get::<String, _>("payload_json"),
                })
            })
            .collect())
    }
}
