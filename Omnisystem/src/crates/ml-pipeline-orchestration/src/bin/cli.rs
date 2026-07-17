//! CLI for exercising the ml-pipeline-orchestration crate.

use ml_pipeline_orchestration::{MLPipelineOrchestrator, ScheduleType, TaskType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orchestrator = MLPipelineOrchestrator::new();

    let pipeline = orchestrator.create_pipeline("training_pipeline", "Preprocess -> Train -> Evaluate").await?;
    println!("Created pipeline '{}'", pipeline.name);

    let preprocess = orchestrator
        .add_task(pipeline.pipeline_id, "preprocess", TaskType::DataPreprocessing, vec![])
        .await?;
    let train = orchestrator
        .add_task(pipeline.pipeline_id, "train", TaskType::ModelTraining, vec![preprocess.task_id])
        .await?;
    let _evaluate = orchestrator
        .add_task(pipeline.pipeline_id, "evaluate", TaskType::Evaluation, vec![train.task_id])
        .await?;

    let execution = orchestrator.execute_pipeline(pipeline.pipeline_id).await?;
    println!(
        "Execution {:?}: {}/{} tasks succeeded",
        execution.execution_status,
        execution.task_results.iter().filter(|(_, s)| *s == ml_pipeline_orchestration::TaskStatus::Succeeded).count(),
        execution.task_results.len()
    );

    orchestrator.schedule_pipeline(pipeline.pipeline_id, ScheduleType::Daily).await?;
    println!("Scheduled pipeline to run daily");

    println!("Total executions tracked: {}", orchestrator.execution_count());
    Ok(())
}
