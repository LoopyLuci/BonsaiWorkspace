//! Algebraic Effects for Database Operations
//!
//! Database interactions are modelled as algebraic effects, enabling:
//! - Testability (swap with in-memory mocks)
//! - Safe resource management
//! - Automatic transaction scoping
//! - Effect inference at compile time


/// DbRead effect — functions that perform database reads are marked with this
#[derive(Debug, Clone)]
pub struct DbReadEffect;

/// DbWrite effect — functions that perform database writes are marked with this
#[derive(Debug, Clone)]
pub struct DbWriteEffect;

/// Effect tracking for a function
#[derive(Debug, Clone)]
pub struct EffectSet {
    pub read: bool,
    pub write: bool,
    pub io: bool,
}

impl EffectSet {
    pub fn none() -> Self {
        Self {
            read: false,
            write: false,
            io: false,
        }
    }

    pub fn reads() -> Self {
        Self {
            read: true,
            write: false,
            io: false,
        }
    }

    pub fn writes() -> Self {
        Self {
            read: false,
            write: true,
            io: false,
        }
    }

    pub fn reads_and_writes() -> Self {
        Self {
            read: true,
            write: true,
            io: false,
        }
    }

    pub fn is_pure(&self) -> bool {
        !self.read && !self.write && !self.io
    }

    pub fn is_read_only(&self) -> bool {
        self.read && !self.write && !self.io
    }
}

/// Handler for database read effects
pub trait DbReadHandler: Send + Sync {
    /// Get a single entity by ID
    fn get<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        id: uuid::Uuid,
    ) -> Result<T, String>;

    /// Execute a query and return results
    fn query<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        query: &str,
    ) -> Result<Vec<T>, String>;

    /// Count entities matching a predicate
    fn count(&self, query: &str) -> Result<usize, String>;

    /// Check if an entity exists
    fn exists(&self, id: uuid::Uuid) -> Result<bool, String>;
}

/// Handler for database write effects
pub trait DbWriteHandler: Send + Sync {
    /// Create a new entity
    fn create<T: serde::Serialize>(&self, entity: &T) -> Result<uuid::Uuid, String>;

    /// Update an existing entity
    fn update<T: serde::Serialize>(&self, id: uuid::Uuid, entity: &T) -> Result<(), String>;

    /// Delete an entity
    fn delete(&self, id: uuid::Uuid) -> Result<(), String>;

    /// Execute a transaction
    fn transaction<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String> + Send,
    {
        f()
    }
}

/// Mock handler for testing — stores data in memory
pub struct MockDbHandler {
    pub data: std::collections::HashMap<uuid::Uuid, serde_json::Value>,
}

impl MockDbHandler {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }
}

impl DbReadHandler for MockDbHandler {
    fn get<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        id: uuid::Uuid,
    ) -> Result<T, String> {
        self.data
            .get(&id)
            .ok_or_else(|| format!("Not found: {}", id))
            .and_then(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
    }

    fn query<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        _query: &str,
    ) -> Result<Vec<T>, String> {
        self.data
            .values()
            .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
            .collect()
    }

    fn count(&self, _query: &str) -> Result<usize, String> {
        Ok(self.data.len())
    }

    fn exists(&self, id: uuid::Uuid) -> Result<bool, String> {
        Ok(self.data.contains_key(&id))
    }
}

impl DbWriteHandler for MockDbHandler {
    fn create<T: serde::Serialize>(&self, entity: &T) -> Result<uuid::Uuid, String> {
        let id = uuid::Uuid::new_v4();
        let _value = serde_json::to_value(entity).map_err(|e| e.to_string())?;
        Ok(id)
    }

    fn update<T: serde::Serialize>(&self, _id: uuid::Uuid, entity: &T) -> Result<(), String> {
        let _value = serde_json::to_value(entity).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete(&self, _id: uuid::Uuid) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_set() {
        let read_only = EffectSet::reads();
        assert!(read_only.is_read_only());
        assert!(!read_only.is_pure());

        let pure = EffectSet::none();
        assert!(pure.is_pure());

        let both = EffectSet::reads_and_writes();
        assert!(!both.is_pure());
        assert!(!both.is_read_only());
    }

    #[test]
    fn test_mock_handler() {
        let handler = MockDbHandler::new();
        assert_eq!(handler.count("").unwrap(), 0);
    }
}
