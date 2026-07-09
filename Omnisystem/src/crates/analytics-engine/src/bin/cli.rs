//! CLI

use analytics_engine::{AnalyticsEngine, DataPoint};
use chrono::Utc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = AnalyticsEngine::new();
    let dataset_id = Uuid::new_v4();
    let point = DataPoint {
        point_id: Uuid::new_v4(),
        dataset_id,
        timestamp: Utc::now(),
        value: 42.5,
        dimensions: vec![],
    };
    engine.ingest_data_point(&point).await?;

    let stats = engine.compute_statistics(dataset_id).await?;
    println!("Mean: {}, min: {}, max: {}", stats.mean, stats.min, stats.max);
    println!("Total points: {}", engine.point_count());
    Ok(())
}
