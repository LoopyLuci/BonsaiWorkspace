use std::path::PathBuf;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CreditError;

/// A double-entry style ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub entry_type: String,
    pub amount: f64,
    pub counterparty: Uuid,
    pub project_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub receipt_hash: String,
}

/// Append-only ledger backed by SQLite.
pub struct Ledger {
    pub db_path: PathBuf,
}

impl Ledger {
    pub fn new(db_path: PathBuf) -> Result<Self, CreditError> {
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS ledger (
                id TEXT PRIMARY KEY,
                entry_type TEXT NOT NULL,
                amount REAL NOT NULL,
                counterparty TEXT NOT NULL,
                project_id TEXT,
                timestamp INTEGER NOT NULL,
                receipt_hash TEXT NOT NULL
            );
        ")?;
        Ok(Ledger { db_path })
    }

    fn open(&self) -> Result<Connection, CreditError> {
        Ok(Connection::open(&self.db_path)?)
    }

    pub fn append(&self, entry: LedgerEntry) -> Result<(), CreditError> {
        let conn = self.open()?;
        conn.execute(
            "INSERT OR REPLACE INTO ledger
             (id, entry_type, amount, counterparty, project_id, timestamp, receipt_hash)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                entry.id.to_string(),
                entry.entry_type,
                entry.amount,
                entry.counterparty.to_string(),
                entry.project_id.map(|u| u.to_string()),
                entry.timestamp.timestamp(),
                entry.receipt_hash,
            ],
        )?;
        Ok(())
    }

    pub fn balance(&self) -> Result<f64, CreditError> {
        let conn = self.open()?;
        let bal: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0.0) FROM ledger",
            [],
            |row| row.get(0),
        )?;
        Ok(bal)
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<LedgerEntry>, CreditError> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, amount, counterparty, project_id, timestamp, receipt_hash
             FROM ledger ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let ts: i64 = row.get(5)?;
            Ok(LedgerEntry {
                id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                entry_type: row.get(1)?,
                amount: row.get(2)?,
                counterparty: row.get::<_, String>(3)?.parse().unwrap_or_default(),
                project_id: row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| s.parse().ok()),
                timestamp: DateTime::from_timestamp(ts, 0).unwrap_or_default(),
                receipt_hash: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CreditError::Db)
    }
}
