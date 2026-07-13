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
    Self {
      records: Arc::new(DashMap::new()),
    }
  }

  pub async fn create(&self) -> Result<Record> {
    let record = Record {
      id: Uuid::new_v4(),
      created_at: Utc::now(),
    };
    self.records.insert(record.id, record.clone());
    Ok(record)
  }

  pub async fn get(&self, id: Uuid) -> Result<Record> {
    self.records.get(&id)
      .map(|r| r.value().clone())
      .ok_or(Error::NotFound)
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
    let record = mgr.create().await.unwrap();
    assert_eq!(mgr.count(), 1);
    assert!(!record.id.is_nil());
  }

  #[tokio::test]
  async fn test_get() {
    let mgr = Manager::new();
    let record = mgr.create().await.unwrap();
    let fetched = mgr.get(record.id).await.unwrap();
    assert_eq!(fetched.id, record.id);
  }

  #[tokio::test]
  async fn test_get_not_found() {
    let mgr = Manager::new();
    let result = mgr.get(Uuid::new_v4()).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_multi() {
    let mgr = Manager::new();
    for _ in 0..10 {
      mgr.create().await.unwrap();
    }
    assert_eq!(mgr.count(), 10);
  }
}
