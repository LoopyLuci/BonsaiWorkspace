use crate::{{error::*, types::*}};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub type Store = Arc<DashMap<Uuid, Record>>;

pub struct Manager {
    store: Store,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    pub fn create(&self, req: CreateRequest) -> Result<Record> {
        let record = Record::new(req.created_by);
        self.store.insert(record.id, record.clone());
        Ok(record)
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Record>> {
        Ok(self.store.get(&id).map(|r| r.clone()))
    }

    pub fn update(&self, id: Uuid, req: UpdateRequest) -> Result<Record> {
        self.store
            .get_mut(&id)
            .map(|mut record| {
                record.updated_by = req.updated_by;
                record.updated_at = chrono::Utc::now();
                record.clone()
            })
            .ok_or_else(|| Error::NotFound(id.to_string()))
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        self.store
            .remove(&id)
            .ok_or_else(|| Error::NotFound(id.to_string()))?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Record> {
        self.store.iter().map(|r| r.clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.store.len()
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}
