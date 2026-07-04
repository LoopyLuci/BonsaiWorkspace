use crate::{error::Result, types::Record};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

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
        let record = Record::new();
        self.records.insert(record.id, record.clone());
        Ok(record)
    }

    pub async fn get(&self, id: Uuid) -> Result<Record> {
        self.records
            .get(&id)
            .map(|r| r.clone())
            .ok_or(crate::error::Error::NotFound)
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
