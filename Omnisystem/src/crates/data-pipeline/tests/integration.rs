use data_pipeline::*;

#[tokio::test]
async fn test_full_etl_pipeline() {
    let pipeline = pipeline::Pipeline::new("etl1".to_string(), "Daily ETL".to_string());
    
    pipeline.add_stage(1, pipeline::StageType::Extract).unwrap();
    pipeline.add_stage(2, pipeline::StageType::Transform).unwrap();
    pipeline.add_stage(3, pipeline::StageType::Load).unwrap();
    pipeline.add_stage(4, pipeline::StageType::Validate).unwrap();
    
    assert_eq!(pipeline.stage_count(), 4);
    
    assert!(pipeline.execute().await.is_ok());
    assert_eq!(pipeline.execution_count(), 4);
}

#[test]
fn test_data_extraction() {
    let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
    let rows = extractor::Extractor::extract_from_csv(csv).unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn test_data_transformation() {
    let values = vec![10.0, 20.0, 30.0];
    let normalized = transformer::Transformer::normalize(&values).unwrap();
    assert_eq!(normalized.len(), 3);
}

#[test]
fn test_data_loading() {
    let loader = loader::Loader::new();
    let dest = loader::LoadDestination {
        name: "warehouse".to_string(),
        dest_type: loader::DestinationType::DataWarehouse,
        records_loaded: 0,
    };
    loader.register_destination(dest).unwrap();
    loader.load_data("warehouse", 1000).unwrap();
}

#[test]
fn test_scheduling() {
    let scheduler = scheduler::PipelineScheduler::new();
    let schedule = scheduler::Schedule {
        pipeline_id: "daily_etl".to_string(),
        frequency: scheduler::ScheduleFrequency::Daily,
        last_run: 0,
        next_run: 0,
    };
    scheduler.schedule_pipeline(schedule).unwrap();
    assert_eq!(scheduler.schedule_count(), 1);
}
