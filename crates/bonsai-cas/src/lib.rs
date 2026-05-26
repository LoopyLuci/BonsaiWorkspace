//! Content-Addressed Store — Blake3-keyed blob storage backed by SQLite + flat files.
//!
//! Blobs ≤ INLINE_THRESHOLD bytes are stored inline in the DB for fast access.
//! Larger blobs are written to `<blob_dir>/<key[0..2]>/<key>.bin`.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const INLINE_THRESHOLD: usize = 65_536; // 64 KiB

// ── Key ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CasKey(pub [u8; 32]);

impl CasKey {
    pub fn from_bytes(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    pub fn hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, CasError> {
        let bytes = hex::decode(s).map_err(|_| CasError::InvalidKey(s.to_string()))?;
        if bytes.len() != 32 {
            return Err(CasError::InvalidKey(s.to_string()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for CasKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum CasError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("key not found: {0}")]
    NotFound(String),
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct CasStore {
    db: sqlx::SqlitePool,
    blob_dir: PathBuf,
}

impl CasStore {
    /// Open (or create) a CAS store at `db_path`, with large blobs in `blob_dir`.
    pub async fn open(db_path: &Path, blob_dir: &Path) -> Result<Self, CasError> {
        tokio::fs::create_dir_all(blob_dir).await?;

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = sqlx::SqlitePool::connect(&url).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cas_objects (
                key        TEXT PRIMARY KEY,
                size       INTEGER NOT NULL,
                mime_type  TEXT NOT NULL DEFAULT 'application/octet-stream',
                created_at INTEGER NOT NULL,
                ref_count  INTEGER NOT NULL DEFAULT 0,
                inline_data BLOB
            )"
        ).execute(&pool).await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cas_mime ON cas_objects(mime_type)"
        ).execute(&pool).await?;

        Ok(Self { db: pool, blob_dir: blob_dir.to_path_buf() })
    }

    /// Store `data` with the given MIME type. Returns the content key.
    /// Idempotent — calling with identical bytes returns the same key instantly.
    pub async fn put(&self, data: &[u8], mime: &str) -> Result<CasKey, CasError> {
        let key = CasKey::from_bytes(data);
        let hex = key.hex();

        // Already stored?
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM cas_objects WHERE key = ?)"
        )
        .bind(&hex)
        .fetch_one(&self.db)
        .await?;

        if exists {
            return Ok(key);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if data.len() <= INLINE_THRESHOLD {
            sqlx::query(
                "INSERT INTO cas_objects (key, size, mime_type, created_at, inline_data)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&hex)
            .bind(data.len() as i64)
            .bind(mime)
            .bind(now)
            .bind(data)
            .execute(&self.db)
            .await?;
        } else {
            // Write to flat file first, then record in DB
            self.write_blob_file(&hex, data).await?;
            sqlx::query(
                "INSERT INTO cas_objects (key, size, mime_type, created_at, inline_data)
                 VALUES (?, ?, ?, ?, NULL)"
            )
            .bind(&hex)
            .bind(data.len() as i64)
            .bind(mime)
            .bind(now)
            .execute(&self.db)
            .await?;
        }

        Ok(key)
    }

    /// Retrieve blob by key. Returns `None` if not present.
    pub async fn get(&self, key: &CasKey) -> Result<Option<Vec<u8>>, CasError> {
        let hex = key.hex();

        let row: Option<(i64, Option<Vec<u8>>)> = sqlx::query_as(
            "SELECT size, inline_data FROM cas_objects WHERE key = ?"
        )
        .bind(&hex)
        .fetch_optional(&self.db)
        .await?;

        match row {
            None => Ok(None),
            Some((_size, Some(inline))) => Ok(Some(inline)),
            Some((_size, None)) => {
                // Read from flat file
                let path = self.blob_path(&hex);
                match tokio::fs::read(&path).await {
                    Ok(data) => Ok(Some(data)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(CasError::Io(e)),
                }
            }
        }
    }

    /// Check existence without fetching data.
    pub async fn exists(&self, key: &CasKey) -> Result<bool, CasError> {
        let hex = key.hex();
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM cas_objects WHERE key = ?)"
        )
        .bind(&hex)
        .fetch_one(&self.db)
        .await?;
        Ok(exists)
    }

    /// Increment reference count so the object survives `gc()`.
    pub async fn pin(&self, key: &CasKey) -> Result<(), CasError> {
        let hex = key.hex();
        sqlx::query("UPDATE cas_objects SET ref_count = ref_count + 1 WHERE key = ?")
            .bind(&hex)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Decrement reference count. If it reaches 0 the object becomes GC-eligible.
    pub async fn unpin(&self, key: &CasKey) -> Result<(), CasError> {
        let hex = key.hex();
        sqlx::query(
            "UPDATE cas_objects SET ref_count = MAX(0, ref_count - 1) WHERE key = ?"
        )
        .bind(&hex)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Delete all objects with `ref_count = 0`. Returns number of objects deleted.
    pub async fn gc(&self) -> Result<u64, CasError> {
        // Collect keys of large blobs before deleting DB rows
        let large_keys: Vec<String> = sqlx::query_scalar(
            "SELECT key FROM cas_objects WHERE ref_count = 0 AND inline_data IS NULL"
        )
        .fetch_all(&self.db)
        .await?;

        let result = sqlx::query("DELETE FROM cas_objects WHERE ref_count = 0")
            .execute(&self.db)
            .await?;

        // Remove orphaned blob files
        for hex in large_keys {
            let path = self.blob_path(&hex);
            let _ = tokio::fs::remove_file(&path).await;
        }

        Ok(result.rows_affected())
    }

    /// List all stored keys with their metadata.
    pub async fn list(&self) -> Result<Vec<CasObjectMeta>, CasError> {
        let rows: Vec<(String, i64, String, i64, i64)> = sqlx::query_as(
            "SELECT key, size, mime_type, created_at, ref_count FROM cas_objects ORDER BY created_at DESC"
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|(key, size, mime_type, created_at, ref_count)| {
            CasObjectMeta { key, size, mime_type, created_at, ref_count }
        }).collect())
    }

    /// Total storage: number of objects and byte count.
    pub async fn stats(&self) -> Result<CasStats, CasError> {
        let (count, total_bytes): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM cas_objects"
        )
        .fetch_one(&self.db)
        .await?;
        Ok(CasStats { object_count: count as u64, total_bytes: total_bytes as u64 })
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn blob_path(&self, hex: &str) -> PathBuf {
        self.blob_dir.join(&hex[..2]).join(format!("{}.bin", hex))
    }

    async fn write_blob_file(&self, hex: &str, data: &[u8]) -> Result<(), CasError> {
        let path = self.blob_path(hex);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        Ok(())
    }
}

// ── Metadata types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasObjectMeta {
    pub key: String,
    pub size: i64,
    pub mime_type: String,
    pub created_at: i64,
    pub ref_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasStats {
    pub object_count: u64,
    pub total_bytes: u64,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn open_temp() -> CasStore {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("cas.db");
        let blob_dir = dir.path().join("blobs");
        // leak the tempdir so it lives for the test
        std::mem::forget(dir);
        CasStore::open(&db_path, &blob_dir).await.unwrap()
    }

    #[tokio::test]
    async fn round_trip_inline() {
        let cas = open_temp().await;
        let data = b"hello omnisystem";
        let key = cas.put(data, "text/plain").await.unwrap();
        let got = cas.get(&key).await.unwrap().unwrap();
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn idempotent_put() {
        let cas = open_temp().await;
        let data = b"same bytes";
        let k1 = cas.put(data, "text/plain").await.unwrap();
        let k2 = cas.put(data, "text/plain").await.unwrap();
        assert_eq!(k1, k2);
        let stats = cas.stats().await.unwrap();
        assert_eq!(stats.object_count, 1);
    }

    #[tokio::test]
    async fn gc_removes_unpinned() {
        let cas = open_temp().await;
        let key = cas.put(b"ephemeral", "text/plain").await.unwrap();
        assert!(cas.exists(&key).await.unwrap());
        let removed = cas.gc().await.unwrap();
        assert_eq!(removed, 1);
        assert!(!cas.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn pin_survives_gc() {
        let cas = open_temp().await;
        let key = cas.put(b"important", "text/plain").await.unwrap();
        cas.pin(&key).await.unwrap();
        let removed = cas.gc().await.unwrap();
        assert_eq!(removed, 0);
        assert!(cas.exists(&key).await.unwrap());
    }
}
