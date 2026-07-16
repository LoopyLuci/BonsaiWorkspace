//! CLI: register a feature, store a value for an entity, and read it back.

use chrono::Utc;
use feature_store::{Feature, FeatureStore, FeatureValue, FeatureVersion};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = FeatureStore::new();

    let feature_id = Uuid::new_v4();
    let feature = Feature {
        feature_id,
        name: "user_age".to_string(),
        feature_group: "user_demographics".to_string(),
        data_type: "integer".to_string(),
        version: "1.0".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.register_feature(&feature).await?;

    let value = FeatureValue {
        value_id: Uuid::new_v4(),
        feature_id,
        entity_id: "user-123".to_string(),
        value: 34.0,
        timestamp: Utc::now(),
    };
    store.store_feature_value(&value).await?;

    let retrieved = store.get_feature_value(feature_id, "user-123").await?;
    println!("user-123's {} = {}", feature.name, retrieved.value);

    let version = FeatureVersion {
        version_id: Uuid::new_v4(),
        feature_id,
        version: "2.0".to_string(),
        created_at: Utc::now(),
        is_active: true,
    };
    store.create_feature_version(&version).await?;
    let active = store.get_active_version(feature_id).await?;
    println!("active version: {}", active.version);

    println!(
        "total features: {}, total values: {}",
        store.feature_count(),
        store.value_count()
    );

    Ok(())
}
