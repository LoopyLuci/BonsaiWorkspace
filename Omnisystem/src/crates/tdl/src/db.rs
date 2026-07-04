//! Low-level database operations for TDL.

use crate::error::Result;
use crate::models::{Example, Metadata, Version, VersionInfo};
use chrono::Utc;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{Row, Transaction};
use std::path::Path;
use uuid::Uuid;

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS versions (
    id TEXT PRIMARY KEY,
    version_string TEXT NOT NULL UNIQUE,
    example_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes INTEGER NOT NULL DEFAULT 0,
    created_by TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    avg_quality_score REAL NOT NULL DEFAULT 0.0,
    version_hash TEXT NOT NULL,
    created_at_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS examples (
    id TEXT PRIMARY KEY,
    version_id TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    quality_score REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    content_size_bytes INTEGER NOT NULL,
    FOREIGN KEY (version_id) REFERENCES versions(id)
);

CREATE TABLE IF NOT EXISTS version_examples (
    version_id TEXT NOT NULL,
    example_id TEXT NOT NULL,
    PRIMARY KEY (version_id, example_id),
    FOREIGN KEY (version_id) REFERENCES versions(id),
    FOREIGN KEY (example_id) REFERENCES examples(id)
);

CREATE TABLE IF NOT EXISTS datasets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    version_id TEXT NOT NULL,
    format TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    FOREIGN KEY (version_id) REFERENCES versions(id)
);

CREATE INDEX IF NOT EXISTS idx_examples_version_id ON examples(version_id);
CREATE INDEX IF NOT EXISTS idx_examples_quality ON examples(quality_score);
CREATE INDEX IF NOT EXISTS idx_examples_content_hash ON examples(content_hash);
CREATE INDEX IF NOT EXISTS idx_datasets_version_id ON datasets(version_id);
CREATE INDEX IF NOT EXISTS idx_versions_created_at ON versions(created_at);
"#;

/// Database connection pool for the Training Data Library.
pub struct TrainingDataDb {
    pool: SqlitePool,
}

impl TrainingDataDb {
    /// Create or open a training data database at the given path.
    pub async fn new(db_path: &Path) -> Result<Self> {
        let connection_string = format!("sqlite://{}", db_path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        // Run schema initialization
        sqlx::query(SCHEMA_V1).execute(&pool).await?;

        Ok(TrainingDataDb { pool })
    }

    /// Create a new version.
    pub async fn create_version(
        &self,
        version_string: &str,
        created_by: &str,
        description: &str,
        tags: Vec<String>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let tags_json = serde_json::to_string(&tags)?;
        let version_hash = blake3::hash(version_string.as_bytes()).to_hex().to_string();

        sqlx::query(
            "INSERT INTO versions (id, version_string, created_by, description, created_at, tags, version_hash, example_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)"
        )
        .bind(id.to_string())
        .bind(version_string)
        .bind(created_by)
        .bind(description)
        .bind(now.to_rfc3339())
        .bind(tags_json)
        .bind(version_hash)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Add an example to a version.
    pub async fn add_example(
        &self,
        version_id: Uuid,
        content: String,
        metadata: Metadata,
        quality_score: f32,
    ) -> Result<Uuid> {
        if !(0.0..=1.0).contains(&quality_score) {
            return Err(crate::TdlError::InvalidQualityScore(quality_score));
        }

        let example_id = Uuid::new_v4();
        let now = Utc::now();
        let metadata_json = serde_json::to_string(&metadata)?;
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let content_size = content.len() as i64;

        let mut tx = self.pool.begin().await?;

        // Insert example
        sqlx::query(
            "INSERT INTO examples (id, version_id, content, metadata, quality_score, created_at, updated_at, content_hash, content_size_bytes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(example_id.to_string())
        .bind(version_id.to_string())
        .bind(&content)
        .bind(&metadata_json)
        .bind(quality_score)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(&content_hash)
        .bind(content_size)
        .execute(&mut *tx)
        .await?;

        // Link to version
        sqlx::query(
            "INSERT OR IGNORE INTO version_examples (version_id, example_id) VALUES (?, ?)"
        )
        .bind(version_id.to_string())
        .bind(example_id.to_string())
        .execute(&mut *tx)
        .await?;

        // Update version statistics
        self.update_version_stats(&mut tx, version_id).await?;

        tx.commit().await?;

        Ok(example_id)
    }

    /// Get examples by tags with pagination.
    pub async fn get_examples_by_tags(
        &self,
        tags: Vec<String>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Example>> {
        let mut examples = Vec::new();

        for row in sqlx::query(
            "SELECT e.id, e.version_id, e.content, e.metadata, e.quality_score, e.created_at, e.updated_at, e.content_hash
             FROM examples e
             WHERE EXISTS (
                 SELECT 1 FROM examples WHERE id = e.id
                 AND metadata LIKE ? OR (? = '')
             )
             LIMIT ? OFFSET ?"
        )
        .bind(if tags.is_empty() {
            String::new()
        } else {
            format!("%{}%", tags[0])
        })
        .bind(if tags.is_empty() { "1" } else { "" })
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?
        {
            let id: String = row.get(0);
            let version_id: String = row.get(1);
            let content: String = row.get(2);
            let metadata_json: String = row.get(3);
            let quality_score: f32 = row.get(4);
            let created_at: String = row.get(5);
            let updated_at: String = row.get(6);
            let content_hash: String = row.get(7);

            let metadata: Metadata = serde_json::from_str(&metadata_json)?;
            let created_at_dt = chrono::DateTime::parse_from_rfc3339(&created_at)?
                .with_timezone(&Utc);
            let updated_at_dt = chrono::DateTime::parse_from_rfc3339(&updated_at)?
                .with_timezone(&Utc);

            examples.push(Example {
                id: Uuid::parse_str(&id)?,
                content,
                metadata,
                quality_score,
                created_at: created_at_dt,
                updated_at: updated_at_dt,
                version_id: Uuid::parse_str(&version_id)?,
                content_hash,
            });
        }

        Ok(examples)
    }

    /// Get examples by minimum quality score.
    pub async fn get_examples_by_quality(
        &self,
        min_score: f32,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Example>> {
        let mut examples = Vec::new();

        for row in sqlx::query(
            "SELECT id, version_id, content, metadata, quality_score, created_at, updated_at, content_hash
             FROM examples
             WHERE quality_score >= ?
             ORDER BY quality_score DESC
             LIMIT ? OFFSET ?"
        )
        .bind(min_score)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?
        {
            let id: String = row.get(0);
            let version_id: String = row.get(1);
            let content: String = row.get(2);
            let metadata_json: String = row.get(3);
            let quality_score: f32 = row.get(4);
            let created_at: String = row.get(5);
            let updated_at: String = row.get(6);
            let content_hash: String = row.get(7);

            let metadata: Metadata = serde_json::from_str(&metadata_json)?;
            let created_at_dt = chrono::DateTime::parse_from_rfc3339(&created_at)?
                .with_timezone(&Utc);
            let updated_at_dt = chrono::DateTime::parse_from_rfc3339(&updated_at)?
                .with_timezone(&Utc);

            examples.push(Example {
                id: Uuid::parse_str(&id)?,
                content,
                metadata,
                quality_score,
                created_at: created_at_dt,
                updated_at: updated_at_dt,
                version_id: Uuid::parse_str(&version_id)?,
                content_hash,
            });
        }

        Ok(examples)
    }

    /// Get version history.
    pub async fn get_version_history(&self) -> Result<Vec<VersionInfo>> {
        let mut versions = Vec::new();

        for row in sqlx::query(
            "SELECT id, version_string, example_count, created_by, created_at, avg_quality_score
             FROM versions
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?
        {
            let id: String = row.get(0);
            let version_string: String = row.get(1);
            let example_count: i64 = row.get(2);
            let created_by: String = row.get(3);
            let created_at: String = row.get(4);
            let avg_quality_score: f32 = row.get(5);

            let created_at_dt = chrono::DateTime::parse_from_rfc3339(&created_at)?
                .with_timezone(&Utc);

            versions.push(VersionInfo {
                id: Uuid::parse_str(&id)?,
                version_string,
                example_count: example_count as usize,
                created_by,
                created_at: created_at_dt,
                avg_quality_score,
            });
        }

        Ok(versions)
    }

    /// Get a single version by ID.
    pub async fn get_version(&self, version_id: Uuid) -> Result<Option<Version>> {
        let row = sqlx::query(
            "SELECT id, version_string, example_count, total_size_bytes, created_by, description, created_at, tags, avg_quality_score, version_hash
             FROM versions WHERE id = ?"
        )
        .bind(version_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let id: String = r.get(0);
                let version_string: String = r.get(1);
                let example_count: i64 = r.get(2);
                let total_size_bytes: i64 = r.get(3);
                let created_by: String = r.get(4);
                let description: String = r.get(5);
                let created_at: String = r.get(6);
                let tags_json: String = r.get(7);
                let avg_quality_score: f32 = r.get(8);
                let version_hash: String = r.get(9);

                let tags: Vec<String> = serde_json::from_str(&tags_json)?;
                let created_at_dt = chrono::DateTime::parse_from_rfc3339(&created_at)?
                    .with_timezone(&Utc);

                Ok(Some(Version {
                    id: Uuid::parse_str(&id)?,
                    version_string,
                    example_count: example_count as usize,
                    total_size_bytes,
                    created_by,
                    description,
                    created_at: created_at_dt,
                    tags,
                    avg_quality_score,
                    version_hash,
                }))
            }
            None => Ok(None),
        }
    }

    /// Update version statistics (example count, total size, avg quality).
    async fn update_version_stats(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        version_id: Uuid,
    ) -> Result<()> {
        let row = sqlx::query(
            "SELECT COUNT(*), SUM(content_size_bytes), AVG(quality_score)
             FROM examples
             WHERE version_id = ?"
        )
        .bind(version_id.to_string())
        .fetch_one(&mut **tx)
        .await?;

        let count: i64 = row.get(0);
        let total_size: Option<i64> = row.get(1);
        let avg_quality: Option<f64> = row.get(2);

        sqlx::query(
            "UPDATE versions SET example_count = ?, total_size_bytes = ?, avg_quality_score = ? WHERE id = ?"
        )
        .bind(count)
        .bind(total_size.unwrap_or(0))
        .bind(avg_quality.unwrap_or(0.0) as f32)
        .bind(version_id.to_string())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Get all examples in a version.
    pub async fn get_version_examples(&self, version_id: Uuid) -> Result<Vec<Example>> {
        let mut examples = Vec::new();

        for row in sqlx::query(
            "SELECT id, version_id, content, metadata, quality_score, created_at, updated_at, content_hash
             FROM examples
             WHERE version_id = ?
             ORDER BY created_at DESC"
        )
        .bind(version_id.to_string())
        .fetch_all(&self.pool)
        .await?
        {
            let id: String = row.get(0);
            let vid: String = row.get(1);
            let content: String = row.get(2);
            let metadata_json: String = row.get(3);
            let quality_score: f32 = row.get(4);
            let created_at: String = row.get(5);
            let updated_at: String = row.get(6);
            let content_hash: String = row.get(7);

            let metadata: Metadata = serde_json::from_str(&metadata_json)?;
            let created_at_dt = chrono::DateTime::parse_from_rfc3339(&created_at)?
                .with_timezone(&Utc);
            let updated_at_dt = chrono::DateTime::parse_from_rfc3339(&updated_at)?
                .with_timezone(&Utc);

            examples.push(Example {
                id: Uuid::parse_str(&id)?,
                content,
                metadata,
                quality_score,
                created_at: created_at_dt,
                updated_at: updated_at_dt,
                version_id: Uuid::parse_str(&vid)?,
                content_hash,
            });
        }

        Ok(examples)
    }

    /// Register a dataset export.
    pub async fn register_dataset(
        &self,
        name: &str,
        description: &str,
        version_id: Uuid,
        format: &str,
        file_path: &str,
        size_bytes: i64,
        checksum: &str,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO datasets (id, name, description, version_id, format, file_path, created_at, size_bytes, checksum)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(name)
        .bind(description)
        .bind(version_id.to_string())
        .bind(format)
        .bind(file_path)
        .bind(now.to_rfc3339())
        .bind(size_bytes)
        .bind(checksum)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }
}
