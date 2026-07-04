use crate::{StorageValue, Result, DataError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct DataStorage {
    data: Arc<DashMap<String, StorageValue>>,
}

impl DataStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    pub fn save(&self, id: String, data: Vec<u8>) -> Result<()> {
        self.data.insert(id.clone(), StorageValue { id, data });
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<StorageValue> {
        self.data.get(id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| DataError::NotFound(id.to_string()))
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.data.remove(id)
            .ok_or_else(|| DataError::NotFound(id.to_string()))?;
        Ok(())
    }
}

impl Default for DataStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_save_load() {
        let storage = DataStorage::new();
        storage.save("id1".to_string(), vec![1, 2, 3]).unwrap();
        let loaded = storage.load("id1").unwrap();
        assert_eq!(loaded.data, vec![1, 2, 3]);
    }
}
