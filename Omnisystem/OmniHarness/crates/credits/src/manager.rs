use std::path::PathBuf;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CreditError;

/// Receipt proving that credits were earned by contributing resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarnReceipt {
    pub id: Uuid,
    pub contributor_id: Uuid,
    pub project_id: Uuid,
    pub minute_timestamp: i64,
    pub cpu_urv_used: f64,
    pub gpu_urv_used: f64,
    pub ram_used_gb: f64,
    pub credits_earned: f64,
    pub seq_num: u64,
    /// Stub — cryptographic signing deferred.
    pub signature: Vec<u8>,
}

/// Receipt proving that credits were spent renting resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendReceipt {
    pub id: Uuid,
    pub renter_id: Uuid,
    pub provider_id: Uuid,
    pub project_id: Uuid,
    pub minute_timestamp: i64,
    pub credits_spent: f64,
    pub seq_num: u64,
    /// Stub — cryptographic signing deferred.
    pub signature: Vec<u8>,
}

/// Manages the local credit balance and receipt history backed by SQLite.
pub struct CreditManager {
    pub db_path: PathBuf,
}

impl CreditManager {
    pub fn new(db_path: PathBuf) -> Result<Self, CreditError> {
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS earn_receipts (
                id TEXT PRIMARY KEY,
                contributor_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                minute_timestamp INTEGER NOT NULL,
                cpu_urv_used REAL NOT NULL,
                gpu_urv_used REAL NOT NULL,
                ram_used_gb REAL NOT NULL,
                credits_earned REAL NOT NULL,
                seq_num INTEGER NOT NULL,
                signature BLOB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS spend_receipts (
                id TEXT PRIMARY KEY,
                renter_id TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                minute_timestamp INTEGER NOT NULL,
                credits_spent REAL NOT NULL,
                seq_num INTEGER NOT NULL,
                signature BLOB NOT NULL
            );
        ")?;
        Ok(CreditManager { db_path })
    }

    fn open(&self) -> Result<Connection, CreditError> {
        Ok(Connection::open(&self.db_path)?)
    }

    pub fn balance(&self) -> Result<f64, CreditError> {
        let conn = self.open()?;
        let earned: f64 = conn.query_row(
            "SELECT COALESCE(SUM(credits_earned), 0.0) FROM earn_receipts",
            [],
            |row| row.get(0),
        )?;
        let spent: f64 = conn.query_row(
            "SELECT COALESCE(SUM(credits_spent), 0.0) FROM spend_receipts",
            [],
            |row| row.get(0),
        )?;
        Ok(earned - spent)
    }

    pub fn record_earn(&self, receipt: EarnReceipt) -> Result<(), CreditError> {
        let conn = self.open()?;
        conn.execute(
            "INSERT OR REPLACE INTO earn_receipts
             (id, contributor_id, project_id, minute_timestamp, cpu_urv_used,
              gpu_urv_used, ram_used_gb, credits_earned, seq_num, signature)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                receipt.id.to_string(),
                receipt.contributor_id.to_string(),
                receipt.project_id.to_string(),
                receipt.minute_timestamp,
                receipt.cpu_urv_used,
                receipt.gpu_urv_used,
                receipt.ram_used_gb,
                receipt.credits_earned,
                receipt.seq_num as i64,
                receipt.signature,
            ],
        )?;
        Ok(())
    }

    pub fn record_spend(&self, receipt: SpendReceipt) -> Result<(), CreditError> {
        let conn = self.open()?;
        conn.execute(
            "INSERT OR REPLACE INTO spend_receipts
             (id, renter_id, provider_id, project_id, minute_timestamp,
              credits_spent, seq_num, signature)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                receipt.id.to_string(),
                receipt.renter_id.to_string(),
                receipt.provider_id.to_string(),
                receipt.project_id.to_string(),
                receipt.minute_timestamp,
                receipt.credits_spent,
                receipt.seq_num as i64,
                receipt.signature,
            ],
        )?;
        Ok(())
    }

    pub fn earnings_history(&self, days: u32) -> Result<Vec<EarnReceipt>, CreditError> {
        let conn = self.open()?;
        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86400);
        let mut stmt = conn.prepare(
            "SELECT id, contributor_id, project_id, minute_timestamp,
                    cpu_urv_used, gpu_urv_used, ram_used_gb, credits_earned,
                    seq_num, signature
             FROM earn_receipts WHERE minute_timestamp >= ?1 ORDER BY minute_timestamp DESC",
        )?;
        let rows = stmt.query_map(params![cutoff], |row| {
            Ok(EarnReceipt {
                id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                contributor_id: row.get::<_, String>(1)?.parse().unwrap_or_default(),
                project_id: row.get::<_, String>(2)?.parse().unwrap_or_default(),
                minute_timestamp: row.get(3)?,
                cpu_urv_used: row.get(4)?,
                gpu_urv_used: row.get(5)?,
                ram_used_gb: row.get(6)?,
                credits_earned: row.get(7)?,
                seq_num: row.get::<_, i64>(8)? as u64,
                signature: row.get(9)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CreditError::Db)
    }

    pub fn spending_history(&self, days: u32) -> Result<Vec<SpendReceipt>, CreditError> {
        let conn = self.open()?;
        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86400);
        let mut stmt = conn.prepare(
            "SELECT id, renter_id, provider_id, project_id, minute_timestamp,
                    credits_spent, seq_num, signature
             FROM spend_receipts WHERE minute_timestamp >= ?1 ORDER BY minute_timestamp DESC",
        )?;
        let rows = stmt.query_map(params![cutoff], |row| {
            Ok(SpendReceipt {
                id: row.get::<_, String>(0)?.parse().unwrap_or_default(),
                renter_id: row.get::<_, String>(1)?.parse().unwrap_or_default(),
                provider_id: row.get::<_, String>(2)?.parse().unwrap_or_default(),
                project_id: row.get::<_, String>(3)?.parse().unwrap_or_default(),
                minute_timestamp: row.get(4)?,
                credits_spent: row.get(5)?,
                seq_num: row.get::<_, i64>(6)? as u64,
                signature: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CreditError::Db)
    }
}
