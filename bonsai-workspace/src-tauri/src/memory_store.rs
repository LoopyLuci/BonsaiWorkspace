/// Memory Platform — session memory, working memory (TTL), long-term memory.
///
/// Memory records are stored in SQLite alongside the assistant store.
/// Each record has a domain (session | working | long_term), a sensitivity
/// classification, a score, and an optional expiry for TTL-based eviction.
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── Schema ────────────────────────────────────────────────────────────────────

/// Memory domain — controls retrieval scope and lifecycle policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDomain {
    /// Scoped to one session; purged when the session ends.
    Session,
    /// Short-lived working facts; expires after `ttl_secs`.
    Working,
    /// Persisted across sessions; scored and ranked for retrieval.
    LongTerm,
}

impl MemoryDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Session  => "session",
            Self::Working  => "working",
            Self::LongTerm => "long_term",
        }
    }
}

/// Sensitivity tier — gates which memory write policies apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Personal,
    Confidential,
}

impl Sensitivity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public       => "public",
            Self::Personal     => "personal",
            Self::Confidential => "confidential",
        }
    }
    /// Returns true if this record requires explicit consent before writing.
    pub fn requires_consent(&self) -> bool {
        matches!(self, Self::Confidential)
    }
}

/// A single memory record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id:          String,
    pub domain:      MemoryDomain,
    pub session_id:  Option<String>,
    pub profile_id:  String,
    pub content:     String,
    pub tags:        Vec<String>,
    pub score:       f32,
    pub sensitivity: Sensitivity,
    pub created_at:  i64,
    pub expires_at:  Option<i64>,
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct MemoryStore {
    pool: SqlitePool,
}

impl MemoryStore {
    pub async fn open(url: &str) -> Result<Self, String> {
        let pool = SqlitePool::connect(url)
            .await
            .map_err(|e| format!("memory_store open: {e}"))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub async fn open_in_memory() -> Result<Self, String> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| format!("memory_store open_in_memory: {e}"))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), String> {
        sqlx::query(r#"
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS memory_records (
                id          TEXT PRIMARY KEY,
                domain      TEXT NOT NULL,
                session_id  TEXT,
                profile_id  TEXT NOT NULL,
                content     TEXT NOT NULL,
                tags_json   TEXT NOT NULL DEFAULT '[]',
                score       REAL NOT NULL DEFAULT 0.5,
                sensitivity TEXT NOT NULL DEFAULT 'public',
                created_at  INTEGER NOT NULL,
                expires_at  INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_memory_profile
                ON memory_records(profile_id, domain);
            CREATE INDEX IF NOT EXISTS idx_memory_session
                ON memory_records(session_id)
                WHERE session_id IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_memory_expires
                ON memory_records(expires_at)
                WHERE expires_at IS NOT NULL;

            CREATE TABLE IF NOT EXISTS memory_audit (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                ts         INTEGER NOT NULL,
                action     TEXT NOT NULL,
                record_id  TEXT NOT NULL,
                profile_id TEXT NOT NULL
            );
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("memory migrate: {e}"))?;
        Ok(())
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Insert or replace a memory record.
    /// Confidential records must have `consent: true` or the write is rejected.
    pub async fn write(&self, record: MemoryRecord, consent: bool) -> Result<(), String> {
        if record.sensitivity.requires_consent() && !consent {
            return Err("write rejected: confidential memory requires explicit consent".into());
        }
        let tags_json = serde_json::to_string(&record.tags).unwrap_or_default();
        let score     = record.score as f64;
        let now       = now_secs();

        sqlx::query(
            r#"INSERT OR REPLACE INTO memory_records
               (id, domain, session_id, profile_id, content, tags_json, score, sensitivity, created_at, expires_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
        )
        .bind(&record.id)
        .bind(record.domain.as_str())
        .bind(&record.session_id)
        .bind(&record.profile_id)
        .bind(&record.content)
        .bind(&tags_json)
        .bind(score)
        .bind(record.sensitivity.as_str())
        .bind(record.created_at)
        .bind(record.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("memory write: {e}"))?;

        sqlx::query(
            "INSERT INTO memory_audit (ts, action, record_id, profile_id) VALUES (?1, 'write', ?2, ?3)",
        )
        .bind(now)
        .bind(&record.id)
        .bind(&record.profile_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("memory audit: {e}"))?;

        Ok(())
    }

    // ── Retrieval ─────────────────────────────────────────────────────────────

    /// Retrieve up to `limit` records for a profile and domain, ordered by score desc.
    pub async fn retrieve(
        &self,
        profile_id: &str,
        domain:     MemoryDomain,
        session_id: Option<&str>,
        limit:      usize,
    ) -> Vec<MemoryRecord> {
        let now   = now_secs();
        let limit = limit as i64;

        let rows = sqlx::query(
            r#"SELECT id, domain, session_id, profile_id, content, tags_json, score,
                      sensitivity, created_at, expires_at
               FROM memory_records
               WHERE profile_id = ?1
                 AND domain = ?2
                 AND (expires_at IS NULL OR expires_at > ?3)
                 AND (session_id IS NULL OR session_id = ?4)
               ORDER BY score DESC
               LIMIT ?5"#,
        )
        .bind(profile_id)
        .bind(domain.as_str())
        .bind(now)
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.into_iter().map(|row| {
            let tags_json: String = row.get("tags_json");
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let domain_str: String = row.get("domain");
            let sens_str:   String = row.get("sensitivity");
            MemoryRecord {
                id:          row.get("id"),
                domain:      match domain_str.as_str() {
                    "session"  => MemoryDomain::Session,
                    "working"  => MemoryDomain::Working,
                    _          => MemoryDomain::LongTerm,
                },
                session_id:  row.get("session_id"),
                profile_id:  row.get("profile_id"),
                content:     row.get("content"),
                tags,
                score:       row.get::<f64, _>("score") as f32,
                sensitivity: match sens_str.as_str() {
                    "personal"     => Sensitivity::Personal,
                    "confidential" => Sensitivity::Confidential,
                    _              => Sensitivity::Public,
                },
                created_at:  row.get("created_at"),
                expires_at:  row.get("expires_at"),
            }
        })
        .collect()
    }

    // ── TTL eviction ──────────────────────────────────────────────────────────

    pub async fn evict_expired(&self) {
        let now = now_secs();
        let _ = sqlx::query(
            "DELETE FROM memory_records WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        )
        .bind(now)
        .execute(&self.pool)
        .await;
    }

    pub async fn evict_session(&self, session_id: &str) {
        let _ = sqlx::query(
            "DELETE FROM memory_records WHERE domain = 'session' AND session_id = ?1",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await;
    }

    // ── Scoped purge ──────────────────────────────────────────────────────────

    pub async fn purge_profile(&self, profile_id: &str) -> Result<usize, String> {
        let result = sqlx::query("DELETE FROM memory_records WHERE profile_id = ?1")
            .bind(profile_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("purge_profile: {e}"))?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn purge_by_tag(&self, profile_id: &str, tag: &str) -> Result<usize, String> {
        let pattern = format!("%\"{tag}\"%");
        let result = sqlx::query(
            "DELETE FROM memory_records WHERE profile_id = ?1 AND tags_json LIKE ?2",
        )
        .bind(profile_id)
        .bind(&pattern)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("purge_by_tag: {e}"))?;
        Ok(result.rows_affected() as usize)
    }
}

// ── Memory candidate extractor / scorer ───────────────────────────────────────

/// Extract candidate memory records from an assistant turn.
pub fn extract_candidates(text: &str, profile_id: &str, session_id: &str) -> Vec<MemoryRecord> {
    let now = now_secs();
    let mut candidates = Vec::new();

    let preference_signals = ["i prefer", "i like", "i hate", "i always", "i never",
                              "my name is", "i am a", "i work at", "i live in"];

    for sentence in text.split('.').map(|s| s.trim()).filter(|s| s.len() > 10) {
        let lower = sentence.to_ascii_lowercase();
        if preference_signals.iter().any(|sig| lower.contains(sig)) {
            candidates.push(MemoryRecord {
                id:          uuid_v4(),
                domain:      MemoryDomain::LongTerm,
                session_id:  Some(session_id.to_string()),
                profile_id:  profile_id.to_string(),
                content:     sentence.to_string(),
                tags:        vec!["preference".to_string()],
                score:       0.7,
                sensitivity: Sensitivity::Personal,
                created_at:  now,
                expires_at:  None,
            });
        }
    }

    candidates
}

/// Score novelty: returns 0.0 if content is very similar to an existing record,
/// 1.0 if completely new. Used to suppress duplicate memory writes.
pub fn novelty_score(candidate: &str, existing: &[MemoryRecord]) -> f32 {
    let candidate_lower = candidate.to_ascii_lowercase();
    for rec in existing {
        let overlap = rec.content.to_ascii_lowercase()
            .split_whitespace()
            .filter(|w| candidate_lower.contains(*w))
            .count();
        let total = candidate_lower.split_whitespace().count().max(1);
        if overlap * 100 / total > 70 {
            return 0.0;
        }
    }
    1.0
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    format!("mem-{ts:08x}-{:08x}", rand_u32())
}

fn rand_u32() -> u32 {
    let x: u32 = 0;
    let addr = &x as *const u32 as u64;
    (addr ^ (addr >> 17)) as u32
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_store() -> MemoryStore {
        MemoryStore::open_in_memory().await.unwrap()
    }

    fn record(domain: MemoryDomain, profile: &str, content: &str) -> MemoryRecord {
        MemoryRecord {
            id:          format!("r-{}-{}", content.len(), profile),
            domain,
            session_id:  Some("sess-1".into()),
            profile_id:  profile.to_string(),
            content:     content.to_string(),
            tags:        vec!["test".into()],
            score:       0.8,
            sensitivity: Sensitivity::Public,
            created_at:  now_secs(),
            expires_at:  None,
        }
    }

    #[tokio::test]
    async fn write_and_retrieve_long_term() {
        let store = mem_store().await;
        let rec = record(MemoryDomain::LongTerm, "p1", "I prefer dark mode");
        store.write(rec, false).await.unwrap();

        let results = store.retrieve("p1", MemoryDomain::LongTerm, None, 10).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "I prefer dark mode");
    }

    #[tokio::test]
    async fn working_memory_expires() {
        let store = mem_store().await;
        let mut rec = record(MemoryDomain::Working, "p1", "temp fact");
        rec.expires_at = Some(now_secs() - 1);
        store.write(rec, false).await.unwrap();

        store.evict_expired().await;
        let results = store.retrieve("p1", MemoryDomain::Working, None, 10).await;
        assert!(results.is_empty(), "expired working memory should be evicted");
    }

    #[tokio::test]
    async fn session_memory_evicted_on_session_close() {
        let store = mem_store().await;
        let rec = record(MemoryDomain::Session, "p1", "session-scoped fact");
        store.write(rec, false).await.unwrap();

        store.evict_session("sess-1").await;
        let results = store.retrieve("p1", MemoryDomain::Session, Some("sess-1"), 10).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn confidential_write_requires_consent() {
        let store = mem_store().await;
        let mut rec = record(MemoryDomain::LongTerm, "p1", "secret info");
        rec.sensitivity = Sensitivity::Confidential;

        let err = store.write(rec, false).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn confidential_write_succeeds_with_consent() {
        let store = mem_store().await;
        let mut rec = record(MemoryDomain::LongTerm, "p1", "secret info");
        rec.sensitivity = Sensitivity::Confidential;

        store.write(rec, true).await.unwrap();
        let results = store.retrieve("p1", MemoryDomain::LongTerm, None, 10).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn profile_isolation_respected() {
        let store = mem_store().await;
        store.write(record(MemoryDomain::LongTerm, "alice", "alice note"), false).await.unwrap();
        store.write(record(MemoryDomain::LongTerm, "bob",   "bob note"),   false).await.unwrap();

        let alice = store.retrieve("alice", MemoryDomain::LongTerm, None, 10).await;
        let bob   = store.retrieve("bob",   MemoryDomain::LongTerm, None, 10).await;
        assert_eq!(alice.len(), 1);
        assert_eq!(bob.len(),   1);
        assert!(alice[0].content.contains("alice"));
        assert!(bob[0].content.contains("bob"));
    }

    #[tokio::test]
    async fn purge_profile_removes_all_records() {
        let store = mem_store().await;
        store.write(record(MemoryDomain::LongTerm, "p2", "fact one"), false).await.unwrap();
        store.write(record(MemoryDomain::LongTerm, "p2", "fact two longer"), false).await.unwrap();

        let n = store.purge_profile("p2").await.unwrap();
        assert_eq!(n, 2);
        let results = store.retrieve("p2", MemoryDomain::LongTerm, None, 10).await;
        assert!(results.is_empty());
    }

    #[test]
    fn novelty_score_detects_duplicate() {
        let existing = vec![MemoryRecord {
            id: "x".into(), domain: MemoryDomain::LongTerm, session_id: None,
            profile_id: "p".into(), content: "I prefer dark mode themes".into(),
            tags: vec![], score: 0.8, sensitivity: Sensitivity::Public,
            created_at: 0, expires_at: None,
        }];
        let score = novelty_score("I prefer dark mode themes always", &existing);
        assert!(score < 0.5, "very similar content should have low novelty score");
    }

    #[test]
    fn novelty_score_new_content_is_high() {
        let existing = vec![MemoryRecord {
            id: "x".into(), domain: MemoryDomain::LongTerm, session_id: None,
            profile_id: "p".into(), content: "I prefer dark mode".into(),
            tags: vec![], score: 0.8, sensitivity: Sensitivity::Public,
            created_at: 0, expires_at: None,
        }];
        let score = novelty_score("The capital of France is Paris", &existing);
        assert!(score > 0.5);
    }

    #[test]
    fn extract_candidates_finds_preference_sentences() {
        let text = "Hello there. I prefer dark mode for all my editors. Goodbye.";
        let candidates = extract_candidates(text, "p1", "sess-1");
        assert!(!candidates.is_empty());
        assert!(candidates[0].content.contains("prefer"));
    }

    #[test]
    fn sensitivity_consent_rules() {
        assert!(!Sensitivity::Public.requires_consent());
        assert!(!Sensitivity::Personal.requires_consent());
        assert!(Sensitivity::Confidential.requires_consent());
    }
}
