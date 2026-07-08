//! Survival System — the Bug Database.
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_hunter`, `sns_bridge`, `crash_ingest`, `daemon`). Durable,
//! deduplicated storage for everything `bug_hunter`'s scanners, `sns_bridge`,
//! and `crash_ingest` discover. Distinct from `kb.rs`'s `fixes` table on
//! purpose: that table is a pattern→shell-script knowledge base for
//! *runtime process* errors, keyed by substring match. This table stores
//! individual bug *occurrences* (compile errors, test failures, lints,
//! fuzzing/sandbox failures, runtime crashes) with dedup, status tracking,
//! and a link to whatever self-upgrade proposal was generated to fix it.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// A bug is never auto-submitted for fixing more than this many times —
/// after that it's left `open` for a human, rather than silently retried
/// forever against a model that keeps failing to fix it.
pub const MAX_AUTO_FIX_ATTEMPTS: i64 = 3;

pub struct BugDb {
    pub pool: Arc<SqlitePool>,
}

impl BugDb {
    pub fn new(db_path: &str) -> Self {
        let url = format!("sqlite://{db_path}?mode=rwc");
        let pool = tauri::async_runtime::block_on(async {
            let p = SqlitePool::connect(&url).await.unwrap_or_else(|_| {
                tauri::async_runtime::block_on(SqlitePool::connect("sqlite::memory:"))
                    .expect("in-memory DB failed")
            });
            sqlx::query(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS bugs (
                     id                       INTEGER PRIMARY KEY AUTOINCREMENT,
                     fingerprint              TEXT    NOT NULL UNIQUE,
                     source                   TEXT    NOT NULL,
                     severity                 TEXT    NOT NULL,
                     title                    TEXT    NOT NULL,
                     message                  TEXT    NOT NULL,
                     file_path                TEXT,
                     line_number              INTEGER,
                     status                   TEXT    NOT NULL DEFAULT 'open',
                     occurrence_count         INTEGER NOT NULL DEFAULT 1,
                     fix_attempts             INTEGER NOT NULL DEFAULT 0,
                     self_upgrade_proposal_id TEXT,
                     first_seen_ms            INTEGER NOT NULL,
                     last_seen_ms             INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_bugs_status ON bugs(status);",
            )
            .execute(&p)
            .await
            .ok();
            p
        });
        Self { pool: Arc::new(pool) }
    }
}

/// Strips runs of digits (line numbers, addresses, timestamps embedded in
/// messages) so two reports of the "same" bug at slightly different lines
/// or with a different pointer address still fingerprint identically.
fn normalize_message(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut prev_was_digit = false;
    for ch in message.chars() {
        if ch.is_ascii_digit() {
            if !prev_was_digit {
                out.push('#');
            }
            prev_was_digit = true;
        } else {
            out.push(ch);
            prev_was_digit = false;
        }
    }
    out
}

pub fn compute_fingerprint(source: &str, file_path: Option<&str>, message: &str) -> String {
    let normalized = normalize_message(message);
    let key = format!("{source}|{}|{normalized}", file_path.unwrap_or(""));
    blake3::hash(key.as_bytes()).to_hex().to_string()
}

/// A freshly-discovered bug, before it's been reconciled against the DB
/// (i.e. before we know whether it's new or a repeat occurrence).
#[derive(Debug, Clone)]
pub struct DiscoveredBug {
    pub source: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugRecord {
    pub id: i64,
    pub fingerprint: String,
    pub source: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub file_path: Option<String>,
    pub line_number: Option<i64>,
    pub status: String,
    pub occurrence_count: i64,
    pub fix_attempts: i64,
    pub self_upgrade_proposal_id: Option<String>,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl BugDb {
    /// Inserts a newly-discovered bug, or — if a bug with the same
    /// fingerprint already exists — bumps its `occurrence_count`/
    /// `last_seen_ms` and re-opens it if it had been marked `fixed`
    /// (a bug that reappears after being "fixed" clearly wasn't).
    pub async fn upsert(&self, bug: DiscoveredBug) -> Result<BugRecord, String> {
        let fingerprint = compute_fingerprint(&bug.source, bug.file_path.as_deref(), &bug.message);
        let ts = now_ms();

        let existing = sqlx::query("SELECT id, status FROM bugs WHERE fingerprint = ?")
            .bind(&fingerprint)
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(row) = existing {
            let id: i64 = row.try_get(0).unwrap_or(0);
            let status: String = row.try_get(1).unwrap_or_default();
            let next_status = if status == "fixed" { "open" } else { status.as_str() };
            sqlx::query(
                "UPDATE bugs SET occurrence_count = occurrence_count + 1, last_seen_ms = ?, status = ?, message = ?
                 WHERE id = ?",
            )
            .bind(ts)
            .bind(next_status)
            .bind(&bug.message)
            .bind(id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        } else {
            sqlx::query(
                "INSERT INTO bugs (fingerprint, source, severity, title, message, file_path, line_number,
                                    status, occurrence_count, fix_attempts, first_seen_ms, last_seen_ms)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 'open', 1, 0, ?, ?)",
            )
            .bind(&fingerprint)
            .bind(&bug.source)
            .bind(&bug.severity)
            .bind(&bug.title)
            .bind(&bug.message)
            .bind(&bug.file_path)
            .bind(bug.line_number)
            .bind(ts)
            .bind(ts)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        }

        self.fetch_by_fingerprint(&fingerprint).await
    }

    async fn fetch_by_fingerprint(&self, fingerprint: &str) -> Result<BugRecord, String> {
        let row = sqlx::query(
            "SELECT id, fingerprint, source, severity, title, message, file_path, line_number,
                    status, occurrence_count, fix_attempts, self_upgrade_proposal_id, first_seen_ms, last_seen_ms
             FROM bugs WHERE fingerprint = ?",
        )
        .bind(fingerprint)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;
        Ok(row_to_record(&row))
    }

    pub async fn list(&self) -> Result<Vec<BugRecord>, String> {
        let rows = sqlx::query(
            "SELECT id, fingerprint, source, severity, title, message, file_path, line_number,
                    status, occurrence_count, fix_attempts, self_upgrade_proposal_id, first_seen_ms, last_seen_ms
             FROM bugs ORDER BY last_seen_ms DESC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(row_to_record).collect())
    }

    pub async fn set_status(&self, id: i64, status: &str) -> Result<(), String> {
        sqlx::query("UPDATE bugs SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn link_proposal(&self, id: i64, proposal_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE bugs SET status = 'fix_proposed', self_upgrade_proposal_id = ? WHERE id = ?")
            .bind(proposal_id)
            .bind(id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn increment_fix_attempts(&self, id: i64) -> Result<(), String> {
        sqlx::query("UPDATE bugs SET fix_attempts = fix_attempts + 1 WHERE id = ?")
            .bind(id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Open bugs still under the auto-fix attempt cap, most-frequently-seen
    /// first (a bug hit by many scans/crashes is a stronger signal than a
    /// one-off).
    pub async fn fixable_candidates(&self, limit: i64) -> Result<Vec<BugRecord>, String> {
        let rows = sqlx::query(
            "SELECT id, fingerprint, source, severity, title, message, file_path, line_number,
                    status, occurrence_count, fix_attempts, self_upgrade_proposal_id, first_seen_ms, last_seen_ms
             FROM bugs WHERE status = 'open' AND fix_attempts < ?
             ORDER BY occurrence_count DESC LIMIT ?",
        )
        .bind(MAX_AUTO_FIX_ATTEMPTS)
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(row_to_record).collect())
    }
}

fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> BugRecord {
    BugRecord {
        id: row.try_get(0).unwrap_or(0),
        fingerprint: row.try_get(1).unwrap_or_default(),
        source: row.try_get(2).unwrap_or_default(),
        severity: row.try_get(3).unwrap_or_default(),
        title: row.try_get(4).unwrap_or_default(),
        message: row.try_get(5).unwrap_or_default(),
        file_path: row.try_get(6).ok(),
        line_number: row.try_get(7).ok(),
        status: row.try_get(8).unwrap_or_default(),
        occurrence_count: row.try_get(9).unwrap_or(1),
        fix_attempts: row.try_get(10).unwrap_or(0),
        self_upgrade_proposal_id: row.try_get(11).ok(),
        first_seen_ms: row.try_get(12).unwrap_or(0),
        last_seen_ms: row.try_get(13).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_db() -> BugDb {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE bugs (
                id                       INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint              TEXT    NOT NULL UNIQUE,
                source                   TEXT    NOT NULL,
                severity                 TEXT    NOT NULL,
                title                    TEXT    NOT NULL,
                message                  TEXT    NOT NULL,
                file_path                TEXT,
                line_number              INTEGER,
                status                   TEXT    NOT NULL DEFAULT 'open',
                occurrence_count         INTEGER NOT NULL DEFAULT 1,
                fix_attempts             INTEGER NOT NULL DEFAULT 0,
                self_upgrade_proposal_id TEXT,
                first_seen_ms            INTEGER NOT NULL,
                last_seen_ms             INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        BugDb { pool: Arc::new(pool) }
    }

    fn sample_bug() -> DiscoveredBug {
        DiscoveredBug {
            source: "compile".into(),
            severity: "error".into(),
            title: "mismatched types".into(),
            message: "expected `String`, found `&str` at line 42".into(),
            file_path: Some("src/foo.rs".into()),
            line_number: Some(42),
        }
    }

    #[tokio::test]
    async fn same_bug_reported_twice_increments_occurrence_not_duplicate_rows() {
        let db = memory_db().await;
        let first = db.upsert(sample_bug()).await.unwrap();
        assert_eq!(first.occurrence_count, 1);

        // Same underlying bug at a slightly different line — fingerprint
        // normalization should still treat it as the same occurrence.
        let mut again = sample_bug();
        again.message = "expected `String`, found `&str` at line 43".into();
        again.line_number = Some(43);
        let second = db.upsert(again).await.unwrap();

        assert_eq!(second.id, first.id);
        assert_eq!(second.occurrence_count, 2);
        assert_eq!(db.list().await.unwrap().len(), 1, "must not create a duplicate row");
    }

    #[tokio::test]
    async fn different_source_or_file_is_a_different_bug() {
        let db = memory_db().await;
        db.upsert(sample_bug()).await.unwrap();
        let mut different_file = sample_bug();
        different_file.file_path = Some("src/bar.rs".into());
        db.upsert(different_file).await.unwrap();
        assert_eq!(db.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn fixed_bug_reopens_if_it_recurs() {
        let db = memory_db().await;
        let bug = db.upsert(sample_bug()).await.unwrap();
        db.set_status(bug.id, "fixed").await.unwrap();

        let recurred = db.upsert(sample_bug()).await.unwrap();
        assert_eq!(recurred.id, bug.id);
        assert_eq!(recurred.status, "open");
    }

    #[tokio::test]
    async fn fixable_candidates_excludes_attempts_at_cap() {
        let db = memory_db().await;
        let bug = db.upsert(sample_bug()).await.unwrap();
        for _ in 0..MAX_AUTO_FIX_ATTEMPTS {
            db.increment_fix_attempts(bug.id).await.unwrap();
        }
        let candidates = db.fixable_candidates(10).await.unwrap();
        assert!(candidates.is_empty(), "a bug at the attempt cap must not be auto-resubmitted");
    }

    #[tokio::test]
    async fn fixable_candidates_excludes_non_open_status() {
        let db = memory_db().await;
        let bug = db.upsert(sample_bug()).await.unwrap();
        db.link_proposal(bug.id, "proposal-123").await.unwrap();
        let candidates = db.fixable_candidates(10).await.unwrap();
        assert!(candidates.is_empty(), "a bug already pending a proposal must not be resubmitted");

        let updated = db.list().await.unwrap().into_iter().find(|b| b.id == bug.id).unwrap();
        assert_eq!(updated.status, "fix_proposed");
        assert_eq!(updated.self_upgrade_proposal_id.as_deref(), Some("proposal-123"));
    }

    #[tokio::test]
    async fn fixable_candidates_ordered_by_occurrence_count_desc() {
        let db = memory_db().await;
        let mut rare_input = sample_bug();
        rare_input.file_path = Some("src/rare.rs".into());
        let rare = db.upsert(rare_input).await.unwrap();

        let mut frequent_input = sample_bug();
        frequent_input.file_path = Some("src/frequent.rs".into());
        db.upsert(frequent_input.clone()).await.unwrap();
        db.upsert(frequent_input.clone()).await.unwrap();
        let frequent = db.upsert(frequent_input).await.unwrap();
        assert_eq!(frequent.occurrence_count, 3);

        let candidates = db.fixable_candidates(10).await.unwrap();
        assert_eq!(candidates[0].id, frequent.id);
        assert_eq!(candidates[1].id, rare.id);
    }
}
