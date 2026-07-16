//! Data pipeline CLI: runs a small demo ETL pipeline end to end (extract
//! CSV, normalize a numeric column, load into a destination, execute the
//! staged pipeline, and schedule it for the next run).

use data_pipeline::{
    DestinationType, Extractor, LoadDestination, Loader, Pipeline, PipelineScheduler, Schedule,
    ScheduleFrequency, StageType, Transformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv = "name,score\nalice,10\nbob,20\ncarol,30";
    let rows = Extractor::extract_from_csv(csv)?;
    println!("Extracted {} rows", rows.len());

    let scores = vec![10.0, 20.0, 30.0];
    let normalized = Transformer::normalize(&scores)?;
    println!("Normalized scores: {:?}", normalized);

    let loader = Loader::new();
    loader.register_destination(LoadDestination {
        name: "warehouse".to_string(),
        dest_type: DestinationType::DataWarehouse,
        records_loaded: 0,
    })?;
    loader.load_data("warehouse", rows.len() as u64)?;
    println!("Loaded into {} destination(s)", loader.destination_count());

    let pipeline = Pipeline::new("demo".to_string(), "Demo ETL".to_string());
    pipeline.add_stage(1, StageType::Extract)?;
    pipeline.add_stage(2, StageType::Transform)?;
    pipeline.add_stage(3, StageType::Load)?;
    pipeline.execute().await?;
    println!(
        "Pipeline '{}' ({}) ran {} stage(s), {} execution record(s)",
        pipeline.name(),
        pipeline.id(),
        pipeline.stage_count(),
        pipeline.execution_count()
    );

    let scheduler = PipelineScheduler::new();
    scheduler.schedule_pipeline(Schedule {
        pipeline_id: "demo".to_string(),
        frequency: ScheduleFrequency::Daily,
        last_run: 0,
        next_run: 0,
    })?;
    println!("Due pipelines: {:?}", scheduler.get_due_pipelines());

    Ok(())
}
