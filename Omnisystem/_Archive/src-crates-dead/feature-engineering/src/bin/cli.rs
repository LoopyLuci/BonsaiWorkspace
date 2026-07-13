//! CLI

use feature_engineering::{FeatureDataType, FeatureEngineer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engineer = FeatureEngineer::new();
    engineer
        .define_feature("user_score", "User activity score", FeatureDataType::Numerical)
        .await?;

    let feature = engineer.compute_feature("user_score", "user_123", 85.5).await?;
    println!("Computed feature: {} = {}", feature.feature_name, feature.value);
    println!("Total features: {}", engineer.feature_count());

    Ok(())
}
