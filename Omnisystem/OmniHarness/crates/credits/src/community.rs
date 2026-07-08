use std::path::PathBuf;
use rusqlite::{Connection, params};
use crate::CreditError;
use crate::urv::credits_per_minute;

/// Platform-level pool funded by the 3% fee on paid transactions.
/// Its balance is stored in the same SQLite database as credit receipts.
pub struct CommunityPool {
    pub balance: f64,
}

impl CommunityPool {
    /// Fee collected from each paid transaction: 3% of the paid amount.
    pub fn collect_fee(paid_credits: f64) -> f64 {
        paid_credits * 0.03
    }

    /// Payout to a free-tier contributor from the community pool.
    ///
    /// `device_urv` — URV score of the contributing device.
    /// `offered_pct` — fraction of capacity offered (0.0–1.0).
    /// `utilization` — actual utilisation during the period (0.0–1.0).
    /// `minutes` — duration of the contribution period.
    pub fn payout_free_tier(
        device_urv: f64,
        offered_pct: f64,
        utilization: f64,
        minutes: f64,
    ) -> f64 {
        credits_per_minute(device_urv, offered_pct, utilization) * minutes
    }
}

/// Persistent community pool balance backed by SQLite.
pub struct PersistentCommunityPool {
    db_path: PathBuf,
}

impl PersistentCommunityPool {
    pub fn new(db_path: PathBuf) -> Result<Self, CreditError> {
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS community_pool (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                balance REAL NOT NULL DEFAULT 0.0
            );
            INSERT OR IGNORE INTO community_pool (id, balance) VALUES (1, 0.0);
        ")?;
        Ok(PersistentCommunityPool { db_path })
    }

    fn open(&self) -> Result<Connection, CreditError> {
        Ok(Connection::open(&self.db_path)?)
    }

    pub fn balance(&self) -> Result<f64, CreditError> {
        let conn = self.open()?;
        let bal: f64 = conn.query_row(
            "SELECT balance FROM community_pool WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(bal)
    }

    pub fn add(&self, amount: f64) -> Result<(), CreditError> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE community_pool SET balance = balance + ?1 WHERE id = 1",
            params![amount],
        )?;
        Ok(())
    }

    pub fn deduct(&self, amount: f64) -> Result<(), CreditError> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE community_pool SET balance = balance - ?1 WHERE id = 1",
            params![amount],
        )?;
        Ok(())
    }
}
