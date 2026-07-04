use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{Connection, params};
use tracing::info;
use uuid::Uuid;

use crate::module::ModuleManifest;
use crate::{KdbError, Result};

/// Persistent registry of all known knowledge modules.
pub struct KdbStore {
    conn: Connection,
    base_dir: PathBuf,
}

impl KdbStore {
    pub fn open(base_dir: &Path) -> Result<Self> {
        fs::create_dir_all(base_dir)?;
        let db_path = base_dir.join("kdb.sqlite");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS modules (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                version     TEXT NOT NULL,
                domain      TEXT NOT NULL,
                description TEXT NOT NULL,
                dim         INTEGER NOT NULL,
                entry_count INTEGER NOT NULL,
                dir_path    TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                blake3_index  TEXT NOT NULL,
                blake3_values TEXT NOT NULL
            );
        ",
        )?;

        Ok(KdbStore {
            conn,
            base_dir: base_dir.to_path_buf(),
        })
    }

    pub fn register_module(&self, manifest: &ModuleManifest, dir: &Path) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO modules
             (id, name, version, domain, description, dim, entry_count, dir_path, created_at, blake3_index, blake3_values)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                manifest.id.to_string(),
                manifest.name,
                manifest.version,
                manifest.domain,
                manifest.description,
                manifest.dim as i64,
                manifest.entry_count as i64,
                dir.to_string_lossy().as_ref(),
                manifest.created_at.to_rfc3339(),
                manifest.blake3_index,
                manifest.blake3_values,
            ],
        )?;
        info!("kdb store: registered module '{}'", manifest.name);
        Ok(())
    }

    pub fn unregister_module(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM modules WHERE name = ?1", params![name])?;
        Ok(())
    }

    pub fn list_modules(&self) -> Result<Vec<ModuleManifest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, version, domain, description, dim, entry_count, dir_path, created_at, blake3_index, blake3_values FROM modules ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (
                id,
                name,
                version,
                domain,
                description,
                dim,
                entry_count,
                _dir,
                created_at,
                blake3_index,
                blake3_values,
            ) = row?;
            result.push(ModuleManifest {
                id: Uuid::parse_str(&id).map_err(|e| KdbError::Invalid(e.to_string()))?,
                name,
                version,
                domain,
                description,
                dim: dim as usize,
                entry_count: entry_count as usize,
                distance: hnsw::Distance::Cosine,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| KdbError::Invalid(e.to_string()))?
                    .with_timezone(&Utc),
                blake3_index,
                blake3_values,
            });
        }
        Ok(result)
    }

    pub fn module_dir(&self, name: &str) -> PathBuf {
        self.base_dir.join("modules").join(name)
    }
}
