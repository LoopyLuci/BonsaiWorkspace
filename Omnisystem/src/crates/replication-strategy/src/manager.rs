use crate::{Record, Error, Result};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct Manager {
    records: Arc<DashMap<Uuid, Record>>,
}

impl Manager {
    pub fn new() -> Self {
        Self { records: Arc::new(DashMap::new()) }
    }

    pub async fn create(&self) -> Result<Record> {
        let rec = Record { id: Uuid::new_v4(), created_at: Utc::now() };
        self.records.insert(rec.id, rec.clone());
        Ok(rec)
    }

    pub fn count(&self) -> usize {
        self.records.len()
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create() {
        let mgr = Manager::new();
        let r = mgr.create().await.unwrap();
        assert!(!r.id.is_nil());
    }

    #[tokio::test]
    async fn test_count() {
        let mgr = Manager::new();
        mgr.create().await.unwrap();
        mgr.create().await.unwrap();
        assert_eq!(mgr.count(), 2);
    }

    #[test]
    fn test_new() {
        let mgr = Manager::new();
        assert_eq!(mgr.count(), 0);
    }

    #[tokio::test]
    async fn test_multi() {
        let mgr = Manager::new();
        for _ in 0..3 {
            mgr.create().await.unwrap();
        }
        assert_eq!(mgr.count(), 3);
    }
}
