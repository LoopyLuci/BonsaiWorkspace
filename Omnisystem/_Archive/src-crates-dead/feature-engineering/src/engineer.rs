use crate::{Feature, FeatureDefinition, FeatureVersion, VersionStatus, FeatureStoreEntry, FeatureError, FeatureResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct FeatureEngineer {
    features: Arc<DashMap<Uuid, Feature>>,
    definitions: Arc<DashMap<String, FeatureDefinition>>,
    versions: Arc<DashMap<String, FeatureVersion>>,
    store: Arc<DashMap<String, FeatureStoreEntry>>,
}

impl FeatureEngineer {
    pub fn new() -> Self {
        Self {
            features: Arc::new(DashMap::new()),
            definitions: Arc::new(DashMap::new()),
            versions: Arc::new(DashMap::new()),
            store: Arc::new(DashMap::new()),
        }
    }

    pub async fn define_feature(&self, name: &str, description: &str, dtype: crate::FeatureDataType) -> FeatureResult<FeatureDefinition> {
        let definition = FeatureDefinition {
            definition_id: Uuid::new_v4(),
            feature_name: name.to_string(),
            description: description.to_string(),
            dtype,
            computation_logic: "".to_string(),
        };

        self.definitions.insert(name.to_string(), definition.clone());
        Ok(definition)
    }

    pub async fn compute_feature(&self, feature_name: &str, entity_id: &str, value: f64) -> FeatureResult<Feature> {
        if self.definitions.get(feature_name).is_none() {
            return Err(FeatureError::FeatureNotFound);
        }

        let feature = Feature {
            feature_id: Uuid::new_v4(),
            feature_name: feature_name.to_string(),
            entity_id: entity_id.to_string(),
            value,
            created_at: Utc::now(),
            valid_from: Utc::now(),
            valid_to: None,
        };

        self.features.insert(feature.feature_id, feature.clone());
        Ok(feature)
    }

    pub async fn version_feature(&self, feature_name: &str, version: &str) -> FeatureResult<FeatureVersion> {
        let feature_version = FeatureVersion {
            version_id: Uuid::new_v4(),
            feature_name: feature_name.to_string(),
            version: version.to_string(),
            created_at: Utc::now(),
            status: VersionStatus::Active,
        };

        self.versions.insert(feature_name.to_string(), feature_version.clone());
        Ok(feature_version)
    }

    pub async fn store_features(&self, entity_id: &str, entity_type: &str, features: Vec<(String, f64)>) -> FeatureResult<FeatureStoreEntry> {
        let entry = FeatureStoreEntry {
            store_id: Uuid::new_v4(),
            entity_id: entity_id.to_string(),
            entity_type: entity_type.to_string(),
            features,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.store.insert(entity_id.to_string(), entry.clone());
        Ok(entry)
    }

    pub async fn retrieve_features(&self, entity_id: &str) -> FeatureResult<FeatureStoreEntry> {
        self.store
            .get(entity_id)
            .map(|e| e.clone())
            .ok_or(FeatureError::FeatureNotFound)
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}

impl Default for FeatureEngineer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FeatureDataType;

    #[tokio::test]
    async fn test_define_feature() {
        let engineer = FeatureEngineer::new();
        let def = engineer.define_feature("age_group", "User age grouping", FeatureDataType::Categorical).await.unwrap();

        assert_eq!(def.feature_name, "age_group");
    }

    #[tokio::test]
    async fn test_compute_feature() {
        let engineer = FeatureEngineer::new();
        engineer.define_feature("user_score", "User activity score", FeatureDataType::Numerical).await.unwrap();

        let feature = engineer.compute_feature("user_score", "user_123", 85.5).await.unwrap();
        assert_eq!(feature.value, 85.5);
        assert_eq!(engineer.feature_count(), 1);
    }

    #[tokio::test]
    async fn test_version_feature() {
        let engineer = FeatureEngineer::new();
        engineer.define_feature("price_feature", "Product price", FeatureDataType::Numerical).await.unwrap();

        let version = engineer.version_feature("price_feature", "v2.1").await.unwrap();
        assert_eq!(version.status, VersionStatus::Active);
    }

    #[tokio::test]
    async fn test_store_features() {
        let engineer = FeatureEngineer::new();
        let features = vec![
            ("feature_a".to_string(), 10.5),
            ("feature_b".to_string(), 20.3),
        ];

        let entry = engineer.store_features("entity_456", "product", features).await.unwrap();
        let retrieved = engineer.retrieve_features("entity_456").await.unwrap();

        assert_eq!(retrieved.entity_type, "product");
        assert_eq!(retrieved.features.len(), 2);
    }
}
