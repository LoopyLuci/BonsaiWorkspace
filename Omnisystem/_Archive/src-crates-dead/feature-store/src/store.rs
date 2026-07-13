use crate::{Feature, FeatureValue, FeatureGroup, FeatureVersion, FeatureSet, FeatureMetadata, FeatureError, FeatureResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct FeatureStore {
    features: Arc<DashMap<Uuid, Feature>>,
    values: Arc<DashMap<Uuid, FeatureValue>>,
    groups: Arc<DashMap<Uuid, FeatureGroup>>,
    versions: Arc<DashMap<Uuid, FeatureVersion>>,
}

impl FeatureStore {
    pub fn new() -> Self {
        Self {
            features: Arc::new(DashMap::new()),
            values: Arc::new(DashMap::new()),
            groups: Arc::new(DashMap::new()),
            versions: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_feature(&self, feature: &Feature) -> FeatureResult<()> {
        self.features.insert(feature.feature_id, feature.clone());
        Ok(())
    }

    pub async fn get_feature(&self, feature_id: Uuid) -> FeatureResult<Feature> {
        self.features
            .get(&feature_id)
            .map(|f| f.clone())
            .ok_or(FeatureError::FeatureNotFound)
    }

    pub async fn store_feature_value(&self, value: &FeatureValue) -> FeatureResult<()> {
        self.values.insert(value.value_id, value.clone());
        Ok(())
    }

    pub async fn get_feature_value(&self, feature_id: Uuid, entity_id: &str) -> FeatureResult<FeatureValue> {
        for entry in self.values.iter() {
            let val = entry.value();
            if val.feature_id == feature_id && val.entity_id == entity_id {
                return Ok(val.clone());
            }
        }

        Err(FeatureError::RetrievalFailed)
    }

    pub async fn create_feature_group(&self, group: &FeatureGroup) -> FeatureResult<()> {
        self.groups.insert(group.group_id, group.clone());
        Ok(())
    }

    pub async fn get_feature_group(&self, group_id: Uuid) -> FeatureResult<FeatureGroup> {
        self.groups
            .get(&group_id)
            .map(|g| g.clone())
            .ok_or(FeatureError::FeatureNotFound)
    }

    pub async fn create_feature_version(&self, version: &FeatureVersion) -> FeatureResult<()> {
        self.versions.insert(version.version_id, version.clone());
        Ok(())
    }

    pub async fn get_active_version(&self, feature_id: Uuid) -> FeatureResult<FeatureVersion> {
        for entry in self.versions.iter() {
            let v = entry.value();
            if v.feature_id == feature_id && v.is_active {
                return Ok(v.clone());
            }
        }

        Err(FeatureError::VersioningFailed)
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    pub fn value_count(&self) -> usize {
        self.values.len()
    }
}

impl Default for FeatureStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_feature() {
        let store = FeatureStore::new();
        let feature = Feature {
            feature_id: Uuid::new_v4(),
            name: "user_age".to_string(),
            feature_group: "user_demographics".to_string(),
            data_type: "integer".to_string(),
            version: "1.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.register_feature(&feature).await.unwrap();
        assert_eq!(store.feature_count(), 1);
    }

    #[tokio::test]
    async fn test_store_feature_value() {
        let store = FeatureStore::new();
        let feature_id = Uuid::new_v4();

        let feature = Feature {
            feature_id,
            name: "user_income".to_string(),
            feature_group: "user_profile".to_string(),
            data_type: "float".to_string(),
            version: "1.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.register_feature(&feature).await.unwrap();

        let value = FeatureValue {
            value_id: Uuid::new_v4(),
            feature_id,
            entity_id: "user123".to_string(),
            value: 75000.0,
            timestamp: Utc::now(),
        };

        store.store_feature_value(&value).await.unwrap();
        assert_eq!(store.value_count(), 1);
    }

    #[tokio::test]
    async fn test_get_feature_value() {
        let store = FeatureStore::new();
        let feature_id = Uuid::new_v4();

        let feature = Feature {
            feature_id,
            name: "score".to_string(),
            feature_group: "metrics".to_string(),
            data_type: "float".to_string(),
            version: "1.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        store.register_feature(&feature).await.unwrap();

        let value = FeatureValue {
            value_id: Uuid::new_v4(),
            feature_id,
            entity_id: "entity1".to_string(),
            value: 85.5,
            timestamp: Utc::now(),
        };

        store.store_feature_value(&value).await.unwrap();

        let retrieved = store.get_feature_value(feature_id, "entity1").await.unwrap();
        assert_eq!(retrieved.value, 85.5);
    }

    #[tokio::test]
    async fn test_create_feature_version() {
        let store = FeatureStore::new();
        let feature_id = Uuid::new_v4();

        let version = FeatureVersion {
            version_id: Uuid::new_v4(),
            feature_id,
            version: "2.0".to_string(),
            created_at: Utc::now(),
            is_active: true,
        };

        store.create_feature_version(&version).await.unwrap();
        let active = store.get_active_version(feature_id).await.unwrap();
        assert!(active.is_active);
    }
}
