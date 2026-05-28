/// Knowledge base — SQLite-backed store for issue→fix mappings.
///
/// Every entry contains:
/// - one or more symptom patterns (substrings / regexes matched against logs)
/// - a repair script (shell command or structured action)
/// - metadata: confidence, usage, who created it

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixEntry {
    pub id:              i64,
    pub error_pattern:   String,
    pub solution_type:   String,  // "rule" | "ai" | "user"
    pub solution_script: String,
    pub confidence:      f64,
    pub usage_count:     i64,
    pub success_count:   i64,
    pub created_by:      String,  // "bonsai" | "user" | "agent"
    pub verified:        bool,
}

pub struct KnowledgeBase {
    conn: Connection,
}

impl KnowledgeBase {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS fixes (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                error_pattern  TEXT    NOT NULL,
                solution_type  TEXT    NOT NULL DEFAULT 'rule',
                solution_script TEXT   NOT NULL,
                confidence     REAL    NOT NULL DEFAULT 0.5,
                usage_count    INTEGER NOT NULL DEFAULT 0,
                success_count  INTEGER NOT NULL DEFAULT 0,
                created_by     TEXT    NOT NULL DEFAULT 'system',
                verified       INTEGER NOT NULL DEFAULT 0,
                created_at     DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_fixes_pattern ON fixes(error_pattern);
        ")?;
        Ok(Self { conn })
    }

    /// Find fixes whose error_pattern appears as a substring of `log`.
    pub fn find_matching(&self, log: &str) -> Vec<FixEntry> {
        let mut stmt = self.conn.prepare(
            "SELECT id, error_pattern, solution_type, solution_script, confidence,
                    usage_count, success_count, created_by, verified
             FROM fixes
             ORDER BY success_count DESC, confidence DESC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok(FixEntry {
                id:              row.get(0)?,
                error_pattern:   row.get(1)?,
                solution_type:   row.get(2)?,
                solution_script: row.get(3)?,
                confidence:      row.get(4)?,
                usage_count:     row.get(5)?,
                success_count:   row.get(6)?,
                created_by:      row.get(7)?,
                verified:        row.get::<_, i64>(8)? != 0,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter(|f| log.contains(&f.error_pattern))
        .collect()
    }

    /// Record a new fix (returns its row id).
    pub fn insert_fix(
        &self,
        pattern:  &str,
        stype:    &str,
        script:   &str,
        confidence: f64,
        created_by: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pattern, stype, script, confidence, created_by],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Increment usage counter; increment success counter if `success` is true.
    pub fn record_outcome(&self, id: i64, success: bool) -> Result<()> {
        if success {
            self.conn.execute(
                "UPDATE fixes SET usage_count = usage_count+1, success_count = success_count+1, verified = 1 WHERE id = ?",
                params![id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE fixes SET usage_count = usage_count+1 WHERE id = ?",
                params![id],
            )?;
        }
        Ok(())
    }

    /// Export all entries as JSONL (for training data generation).
    pub fn export_jsonl(&self) -> Vec<serde_json::Value> {
        let mut stmt = self.conn.prepare(
            "SELECT error_pattern, solution_script FROM fixes WHERE success_count > 0"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|(pattern, script)| serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are an expert system administrator. Given an error log from the Bonsai application, output a single shell command that fixes the problem. Output NOT_FIXABLE if you cannot determine a fix."},
                {"role": "user",   "content": pattern},
                {"role": "assistant", "content": script},
            ]
        }))
        .collect()
    }
}
