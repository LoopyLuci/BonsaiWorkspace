use crate::worker::FailureReport;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Bridge between F³ failure reports and the Survival Knowledge Base.
/// Deduplicates, persists, and queues issues for automated repair.
pub struct SurvivalBridge {
    /// Path to the Survival KB SQLite database.
    db_path:      String,
    /// Recent failures kept in memory for the UI.
    recent:       RwLock<Vec<FailureReport>>,
    /// Fingerprints already in the KB — prevents duplicate rules.
    known_fps:    RwLock<std::collections::HashSet<String>>,
}

impl SurvivalBridge {
    pub fn new(db_path: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            db_path: db_path.into(),
            recent: RwLock::new(Vec::new()),
            known_fps: RwLock::new(std::collections::HashSet::new()),
        })
    }

    /// Report a failure to the Survival KB. Returns true if a new rule was added.
    pub async fn report_failure(&self, failure: &FailureReport) -> bool {
        let fp = failure.fingerprint();

        // Dedup check
        if self.known_fps.read().await.contains(&fp) {
            return false;
        }
        self.known_fps.write().await.insert(fp.clone());

        // Keep in recent list (capped at 1000)
        {
            let mut recent = self.recent.write().await;
            recent.push(failure.clone());
            if recent.len() > 1000 {
                recent.remove(0);
            }
        }

        // Write to Survival KB SQLite
        let pattern = failure.error_pattern.clone();
        let script = failure.auto_fix_script.clone()
            .unwrap_or_else(|| format!("Investigate F³ finding: {}", failure.error_pattern));
        let db_path = self.db_path.clone();
        let target = failure.target.clone();
        let campaign_id = failure.campaign_id.clone();

        let added = tokio::task::spawn_blocking(move || -> bool {
            use rusqlite::Connection;
            let conn = match Connection::open(&db_path) {
                Ok(c) => c,
                Err(e) => { warn!("SurvivalBridge: cannot open KB: {}", e); return false; }
            };
            // Ensure schema exists (in case watchdog hasn't seeded yet)
            let _ = conn.execute_batch("CREATE TABLE IF NOT EXISTS fixes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                error_pattern TEXT NOT NULL,
                solution_type TEXT NOT NULL DEFAULT 'rule',
                solution_script TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                usage_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                created_by TEXT NOT NULL DEFAULT 'system',
                verified INTEGER NOT NULL DEFAULT 0,
                category TEXT NOT NULL DEFAULT 'other',
                tags TEXT NOT NULL DEFAULT '',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );");

            let already: i64 = conn.query_row(
                "SELECT COUNT(*) FROM fixes WHERE error_pattern = ?1",
                rusqlite::params![&pattern],
                |r| r.get(0),
            ).unwrap_or(0);

            if already > 0 { return false; }

            let result = conn.execute(
                "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by, category, tags)
                 VALUES (?1, 'f3_discovered', ?2, 0.7, 'f3_orchestrator', 'discovered', ?3)",
                rusqlite::params![
                    &pattern,
                    &script,
                    format!("f3,{},campaign:{}", target, campaign_id),
                ],
            );

            match result {
                Ok(_) => { info!("SurvivalBridge: new rule added for '{}'", &pattern[..pattern.len().min(60)]); true }
                Err(e) => { warn!("SurvivalBridge: insert failed: {}", e); false }
            }
        }).await.unwrap_or(false);

        added
    }

    pub fn recent_failures(&self) -> Vec<FailureReport> {
        self.recent.try_read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    pub fn failure_count(&self) -> usize {
        self.recent.try_read().map(|r| r.len()).unwrap_or(0)
    }
}
