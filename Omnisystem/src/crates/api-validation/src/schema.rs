use crate::{ValidationError, ValidationResult};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Schema {
    pub schema_id: String,
    pub version: String,
    pub definition: String,
}

pub struct SchemaManager {
    schemas: Arc<DashMap<String, Schema>>,
}

impl SchemaManager {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_schema(&self, schema: &Schema) -> ValidationResult<()> {
        self.schemas.insert(schema.schema_id.clone(), schema.clone());
        Ok(())
    }

    pub async fn get_schema(&self, schema_id: &str) -> ValidationResult<Schema> {
        self.schemas
            .get(schema_id)
            .map(|entry| entry.clone())
            .ok_or(ValidationError::SchemaNotFound)
    }

    pub async fn update_schema(&self, schema: &Schema) -> ValidationResult<()> {
        if self.schemas.contains_key(&schema.schema_id) {
            self.schemas.insert(schema.schema_id.clone(), schema.clone());
            Ok(())
        } else {
            Err(ValidationError::SchemaNotFound)
        }
    }

    pub async fn delete_schema(&self, schema_id: &str) -> ValidationResult<()> {
        if self.schemas.remove(schema_id).is_some() {
            Ok(())
        } else {
            Err(ValidationError::SchemaNotFound)
        }
    }

    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }
}

impl Default for SchemaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_schema() {
        let manager = SchemaManager::new();
        let schema = Schema {
            schema_id: "schema-1".to_string(),
            version: "1.0".to_string(),
            definition: "{}".to_string(),
        };

        manager.register_schema(&schema).await.unwrap();
        assert_eq!(manager.schema_count(), 1);
    }

    #[tokio::test]
    async fn test_get_schema() {
        let manager = SchemaManager::new();
        let schema = Schema {
            schema_id: "schema-1".to_string(),
            version: "1.0".to_string(),
            definition: "{}".to_string(),
        };

        manager.register_schema(&schema).await.unwrap();
        let retrieved = manager.get_schema("schema-1").await.unwrap();

        assert_eq!(retrieved.schema_id, "schema-1");
    }

    #[tokio::test]
    async fn test_update_schema() {
        let manager = SchemaManager::new();
        let schema = Schema {
            schema_id: "schema-1".to_string(),
            version: "1.0".to_string(),
            definition: "{}".to_string(),
        };

        manager.register_schema(&schema).await.unwrap();

        let updated_schema = Schema {
            schema_id: "schema-1".to_string(),
            version: "2.0".to_string(),
            definition: "{}".to_string(),
        };

        manager.update_schema(&updated_schema).await.unwrap();
        let retrieved = manager.get_schema("schema-1").await.unwrap();

        assert_eq!(retrieved.version, "2.0");
    }

    #[tokio::test]
    async fn test_delete_schema() {
        let manager = SchemaManager::new();
        let schema = Schema {
            schema_id: "schema-1".to_string(),
            version: "1.0".to_string(),
            definition: "{}".to_string(),
        };

        manager.register_schema(&schema).await.unwrap();
        manager.delete_schema("schema-1").await.unwrap();

        let result = manager.get_schema("schema-1").await;
        assert!(result.is_err());
    }
}
