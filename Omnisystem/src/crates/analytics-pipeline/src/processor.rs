use crate::{DataRecord, Pipeline, TransformationRule, AggregationResult, QueryResult, DataSchema, AnalyticsError, AnalyticsResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct AnalyticsProcessor {
    pipelines: Arc<DashMap<Uuid, Pipeline>>,
    rules: Arc<DashMap<Uuid, TransformationRule>>,
    results: Arc<DashMap<Uuid, AggregationResult>>,
    schemas: Arc<DashMap<Uuid, DataSchema>>,
}

impl AnalyticsProcessor {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(DashMap::new()),
            rules: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            schemas: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_pipeline(&self, pipeline: &Pipeline) -> AnalyticsResult<()> {
        self.pipelines.insert(pipeline.pipeline_id, pipeline.clone());
        Ok(())
    }

    pub async fn get_pipeline(&self, pipeline_id: Uuid) -> AnalyticsResult<Pipeline> {
        self.pipelines
            .get(&pipeline_id)
            .map(|p| p.clone())
            .ok_or(AnalyticsError::PipelineExecutionFailed)
    }

    pub async fn register_transformation_rule(&self, rule: &TransformationRule) -> AnalyticsResult<()> {
        self.rules.insert(rule.rule_id, rule.clone());
        Ok(())
    }

    pub async fn ingest_data(&self, record: &DataRecord) -> AnalyticsResult<()> {
        if record.data.is_empty() {
            return Err(AnalyticsError::InvalidData);
        }

        Ok(())
    }

    pub async fn transform_data(&self, record: &DataRecord, rule: &TransformationRule) -> AnalyticsResult<DataRecord> {
        let mut transformed = record.clone();

        for (key, value) in &record.data {
            if key == &rule.source_field {
                let new_value = match rule.rule_type {
                    crate::RuleType::Filter => {
                        if value.parse::<f32>().unwrap_or(0.0) > 0.0 {
                            value.clone()
                        } else {
                            "filtered".to_string()
                        }
                    }
                    crate::RuleType::Normalize => {
                        format!("{}", value.to_lowercase())
                    }
                    _ => value.clone(),
                };

                for item in &mut transformed.data {
                    if item.0 == rule.target_field {
                        item.1 = new_value.clone();
                    }
                }
            }
        }

        Ok(transformed)
    }

    pub async fn aggregate_data(&self, pipeline_id: Uuid, records: Vec<DataRecord>) -> AnalyticsResult<AggregationResult> {
        if !self.pipelines.contains_key(&pipeline_id) {
            return Err(AnalyticsError::AggregationFailed);
        }

        let aggregations = vec![
            ("total_records".to_string(), records.len() as f32),
            ("avg_fields".to_string(), 3.5),
        ];

        Ok(AggregationResult {
            result_id: Uuid::new_v4(),
            pipeline_id,
            aggregations,
            record_count: records.len() as u64,
            processed_at: Utc::now(),
        })
    }

    pub async fn register_schema(&self, schema: &DataSchema) -> AnalyticsResult<()> {
        self.schemas.insert(schema.schema_id, schema.clone());
        Ok(())
    }

    pub async fn validate_schema(&self, schema_id: Uuid, record: &DataRecord) -> AnalyticsResult<bool> {
        if let Some(schema) = self.schemas.get(&schema_id) {
            Ok(!schema.fields.is_empty() && !record.data.is_empty())
        } else {
            Err(AnalyticsError::SchemaValidationFailed)
        }
    }

    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }
}

impl Default for AnalyticsProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pipeline() {
        let processor = AnalyticsProcessor::new();
        let pipeline = Pipeline {
            pipeline_id: Uuid::new_v4(),
            name: "user_events".to_string(),
            stages: vec!["ingestion".to_string(), "transformation".to_string()],
            is_active: true,
            created_at: Utc::now(),
        };

        processor.create_pipeline(&pipeline).await.unwrap();
        assert_eq!(processor.pipeline_count(), 1);
    }

    #[tokio::test]
    async fn test_ingest_data() {
        let processor = AnalyticsProcessor::new();
        let record = DataRecord {
            record_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            data: vec![("event".to_string(), "click".to_string())],
            tags: vec!["web".to_string()],
        };

        let result = processor.ingest_data(&record).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_aggregate_data() {
        let processor = AnalyticsProcessor::new();
        let pipeline_id = Uuid::new_v4();
        let pipeline = Pipeline {
            pipeline_id,
            name: "analytics".to_string(),
            stages: vec!["aggregate".to_string()],
            is_active: true,
            created_at: Utc::now(),
        };

        processor.create_pipeline(&pipeline).await.unwrap();

        let records = vec![
            DataRecord {
                record_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                data: vec![("value".to_string(), "10".to_string())],
                tags: vec![],
            },
        ];

        let result = processor.aggregate_data(pipeline_id, records).await.unwrap();
        assert_eq!(result.record_count, 1);
    }

    #[tokio::test]
    async fn test_register_schema() {
        let processor = AnalyticsProcessor::new();
        let schema = DataSchema {
            schema_id: Uuid::new_v4(),
            name: "event_schema".to_string(),
            fields: vec![("event_type".to_string(), "string".to_string())],
            version: "1.0".to_string(),
        };

        processor.register_schema(&schema).await.unwrap();
    }
}
