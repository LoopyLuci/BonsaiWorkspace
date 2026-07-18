//! CLI: create a pipeline, ingest and transform a record, then aggregate.

use analytics_pipeline::{AnalyticsProcessor, DataRecord, Pipeline, RuleType, TransformationRule};
use chrono::Utc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let processor = AnalyticsProcessor::new();

    let pipeline_id = Uuid::new_v4();
    let pipeline = Pipeline {
        pipeline_id,
        name: "user_events".to_string(),
        stages: vec!["ingest".to_string(), "transform".to_string(), "aggregate".to_string()],
        is_active: true,
        created_at: Utc::now(),
    };
    processor.create_pipeline(&pipeline).await?;

    let record = DataRecord {
        record_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        data: vec![("EVENT".to_string(), "Click".to_string())],
        tags: vec!["web".to_string()],
    };
    processor.ingest_data(&record).await?;

    let rule = TransformationRule {
        rule_id: Uuid::new_v4(),
        name: "lowercase_event".to_string(),
        source_field: "EVENT".to_string(),
        target_field: "EVENT".to_string(),
        rule_type: RuleType::Normalize,
    };
    processor.register_transformation_rule(&rule).await?;
    let transformed = processor.transform_data(&record, &rule).await?;
    println!("transformed field: {:?}", transformed.data);

    let result = processor.aggregate_data(pipeline_id, vec![record, transformed]).await?;
    println!(
        "aggregation: record_count={} aggregations={:?}",
        result.record_count, result.aggregations
    );

    println!("total pipelines: {}", processor.pipeline_count());

    Ok(())
}
