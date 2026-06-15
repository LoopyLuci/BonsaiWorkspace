use crate::{CatalogEntry, Result};

pub trait CatalogStorage: Send + Sync {
    fn save(&self, entry: &CatalogEntry) -> Result<()>;
    fn load(&self, id: &str) -> Result<CatalogEntry>;
    fn delete(&self, id: &str) -> Result<()>;
    fn list_all(&self) -> Result<Vec<CatalogEntry>>;
}

pub struct MemoryCatalogStorage;

impl CatalogStorage for MemoryCatalogStorage {
    fn save(&self, _entry: &CatalogEntry) -> Result<()> {
        Ok(())
    }

    fn load(&self, _id: &str) -> Result<CatalogEntry> {
        Err(crate::CatalogError::NotFound("not implemented".to_string()))
    }

    fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<CatalogEntry>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryCatalogStorage;
        assert!(storage.delete("test").is_ok());
    }
}
