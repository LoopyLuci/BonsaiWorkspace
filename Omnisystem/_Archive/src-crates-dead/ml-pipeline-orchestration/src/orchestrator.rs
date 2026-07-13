use crate::{Pipeline, PipelineStatus, PipelineTask, TaskStatus, PipelineExecution, ExecutionStatus, PipelineSchedule, ScheduleType, PipelineError, PipelineResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct MLPipelineOrchestrator {
    pipelines: Arc<DashMap<Uuid, Pipeline>>,
    tasks: Arc<DashMap<Uuid, PipelineTask>>,
    executions: Arc<DashMap<Uuid, PipelineExecution>>,
    schedules: Arc<DashMap<Uuid, PipelineSchedule>>,
}

impl MLPipelineOrchestrator {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            executions: Arc::new(DashMap::new()),
            schedules: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_pipeline(&self, name: &str, description: &str) -> PipelineResult<Pipeline> {
        let pipeline = Pipeline {
            pipeline_id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            created_at: Utc::now(),
            status: PipelineStatus::Draft,
        };

        self.pipelines.insert(pipeline.pipeline_id, pipeline.clone());
        Ok(pipeline)
    }

    pub async fn add_task(&self, pipeline_id: Uuid, task_name: &str, task_type: crate::TaskType) -> PipelineResult<PipelineTask> {
        if self.pipelines.get(&pipeline_id).is_none() {
            return Err(PipelineError::PipelineNotFound);
        }

        let task = PipelineTask {
            task_id: Uuid::new_v4(),
            pipeline_id,
            task_name: task_name.to_string(),
            task_type,
            dependencies: vec![],
            status: TaskStatus::Pending,
        };

        self.tasks.insert(task.task_id, task.clone());
        Ok(task)
    }

    pub async fn execute_pipeline(&self, pipeline_id: Uuid) -> PipelineResult<PipelineExecution> {
        if self.pipelines.get(&pipeline_id).is_none() {
            return Err(PipelineError::PipelineNotFound);
        }

        let execution = PipelineExecution {
            execution_id: Uuid::new_v4(),
            pipeline_id,
            start_time: Utc::now(),
            end_time: None,
            execution_status: ExecutionStatus::Running,
            task_results: vec![],
        };

        self.executions.insert(execution.execution_id, execution.clone());
        Ok(execution)
    }

    pub async fn complete_execution(&self, execution_id: Uuid) -> PipelineResult<()> {
        if let Some(mut entry) = self.executions.get_mut(&execution_id) {
            entry.end_time = Some(Utc::now());
            entry.execution_status = ExecutionStatus::Succeeded;
        } else {
            return Err(PipelineError::ExecutionFailed);
        }

        Ok(())
    }

    pub async fn schedule_pipeline(&self, pipeline_id: Uuid, schedule_type: crate::ScheduleType) -> PipelineResult<PipelineSchedule> {
        if self.pipelines.get(&pipeline_id).is_none() {
            return Err(PipelineError::PipelineNotFound);
        }

        let schedule = PipelineSchedule {
            schedule_id: Uuid::new_v4(),
            pipeline_id,
            schedule_type,
            next_run: Utc::now(),
            enabled: true,
        };

        self.schedules.insert(schedule.schedule_id, schedule.clone());
        Ok(schedule)
    }

    pub fn execution_count(&self) -> usize {
        self.executions.len()
    }
}

impl Default for MLPipelineOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskType;

    #[tokio::test]
    async fn test_create_pipeline() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("data_pipeline", "End-to-end ML pipeline").await.unwrap();

        assert_eq!(pipeline.name, "data_pipeline");
        assert_eq!(pipeline.status, PipelineStatus::Draft);
    }

    #[tokio::test]
    async fn test_add_task() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("ml_pipe", "ML workflow").await.unwrap();

        let task = orchestrator
            .add_task(pipeline.pipeline_id, "preprocess", TaskType::DataPreprocessing)
            .await
            .unwrap();

        assert_eq!(task.task_name, "preprocess");
    }

    #[tokio::test]
    async fn test_execute_pipeline() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("train_pipe", "Training pipeline").await.unwrap();

        let execution = orchestrator.execute_pipeline(pipeline.pipeline_id).await.unwrap();
        assert_eq!(execution.execution_status, ExecutionStatus::Running);
        assert_eq!(orchestrator.execution_count(), 1);
    }

    #[tokio::test]
    async fn test_schedule_pipeline() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("scheduled_pipe", "Scheduled pipeline").await.unwrap();

        let schedule = orchestrator
            .schedule_pipeline(pipeline.pipeline_id, ScheduleType::Daily)
            .await
            .unwrap();

        assert!(schedule.enabled);
    }
}
