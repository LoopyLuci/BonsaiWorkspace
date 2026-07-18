use crate::{Pipeline, PipelineStatus, PipelineTask, TaskStatus, PipelineExecution, ExecutionStatus, PipelineSchedule, PipelineError, PipelineResult};
use dashmap::DashMap;
use std::collections::HashMap;
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

    /// Add a task to a pipeline with its real upstream dependencies, so
    /// `PipelineTask::dependencies` is actually populated instead of
    /// always being an empty Vec.
    pub async fn add_task(&self, pipeline_id: Uuid, task_name: &str, task_type: crate::TaskType, dependencies: Vec<Uuid>) -> PipelineResult<PipelineTask> {
        if self.pipelines.get(&pipeline_id).is_none() {
            return Err(PipelineError::PipelineNotFound);
        }

        let task = PipelineTask {
            task_id: Uuid::new_v4(),
            pipeline_id,
            task_name: task_name.to_string(),
            task_type,
            dependencies,
            status: TaskStatus::Pending,
        };

        self.tasks.insert(task.task_id, task.clone());
        Ok(task)
    }

    /// Resolve a pipeline's tasks in dependency order and run them
    /// (synchronously, since this in-memory orchestrator has no real
    /// compute backend), cascading failure to any task whose dependency
    /// failed or is missing entirely, and detecting cyclic dependencies
    /// instead of silently accepting them.
    fn resolve_tasks(&self, pipeline_id: Uuid) -> PipelineResult<Vec<(Uuid, TaskStatus)>> {
        let pipeline_tasks: HashMap<Uuid, PipelineTask> = self
            .tasks
            .iter()
            .filter(|e| e.value().pipeline_id == pipeline_id)
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();

        let mut resolved: HashMap<Uuid, TaskStatus> = HashMap::new();
        let mut remaining: Vec<Uuid> = pipeline_tasks.keys().copied().collect();

        while !remaining.is_empty() {
            let mut progressed = false;
            let mut still_remaining = Vec::new();

            for task_id in remaining {
                let task = &pipeline_tasks[&task_id];
                let mut all_deps_ready = true;
                let mut any_dep_failed = false;

                for dep_id in &task.dependencies {
                    match resolved.get(dep_id) {
                        Some(TaskStatus::Succeeded) => {}
                        Some(_) => any_dep_failed = true,
                        None => {
                            if pipeline_tasks.contains_key(dep_id) {
                                // Dependency belongs to this pipeline but
                                // hasn't been resolved yet -- wait for it.
                                all_deps_ready = false;
                            } else {
                                // Dependency references a task that doesn't
                                // exist at all; it can never succeed.
                                any_dep_failed = true;
                            }
                        }
                    }
                }

                if !all_deps_ready {
                    still_remaining.push(task_id);
                    continue;
                }

                let status = if any_dep_failed { TaskStatus::Skipped } else { TaskStatus::Succeeded };
                resolved.insert(task_id, status);
                if let Some(mut entry) = self.tasks.get_mut(&task_id) {
                    entry.status = status;
                }
                progressed = true;
            }

            if !progressed && !still_remaining.is_empty() {
                return Err(PipelineError::CyclicDependency);
            }
            remaining = still_remaining;
        }

        Ok(resolved.into_iter().collect())
    }

    fn execution_status_from_results(task_results: &[(Uuid, TaskStatus)]) -> ExecutionStatus {
        if task_results.is_empty() {
            return ExecutionStatus::Succeeded;
        }
        let succeeded = task_results.iter().filter(|(_, s)| *s == TaskStatus::Succeeded).count();
        if succeeded == task_results.len() {
            ExecutionStatus::Succeeded
        } else if succeeded == 0 {
            ExecutionStatus::Failed
        } else {
            ExecutionStatus::PartiallyFailed
        }
    }

    pub async fn execute_pipeline(&self, pipeline_id: Uuid) -> PipelineResult<PipelineExecution> {
        if self.pipelines.get(&pipeline_id).is_none() {
            return Err(PipelineError::PipelineNotFound);
        }

        let start_time = Utc::now();
        let task_results = self.resolve_tasks(pipeline_id)?;
        let execution_status = Self::execution_status_from_results(&task_results);

        let execution = PipelineExecution {
            execution_id: Uuid::new_v4(),
            pipeline_id,
            start_time,
            end_time: Some(Utc::now()),
            execution_status,
            task_results,
        };

        self.executions.insert(execution.execution_id, execution.clone());

        if let Some(mut pipeline) = self.pipelines.get_mut(&pipeline_id) {
            pipeline.status = match execution_status {
                ExecutionStatus::Succeeded => PipelineStatus::Completed,
                ExecutionStatus::Failed => PipelineStatus::Failed,
                _ => PipelineStatus::Active,
            };
        }

        Ok(execution)
    }

    /// Finalize an execution by re-deriving its status from the real
    /// current state of its pipeline's tasks, rather than unconditionally
    /// marking it Succeeded regardless of whether any task actually failed.
    pub async fn complete_execution(&self, execution_id: Uuid) -> PipelineResult<()> {
        let pipeline_id = self
            .executions
            .get(&execution_id)
            .map(|e| e.value().pipeline_id)
            .ok_or(PipelineError::ExecutionFailed)?;

        let task_results: Vec<(Uuid, TaskStatus)> = self
            .tasks
            .iter()
            .filter(|e| e.value().pipeline_id == pipeline_id)
            .map(|e| (e.value().task_id, e.value().status))
            .collect();
        let execution_status = Self::execution_status_from_results(&task_results);

        if let Some(mut entry) = self.executions.get_mut(&execution_id) {
            entry.end_time = Some(Utc::now());
            entry.execution_status = execution_status;
            entry.task_results = task_results;
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
    use crate::{ScheduleType, TaskType};

    #[tokio::test]
    async fn test_create_pipeline() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("data_pipeline", "End-to-end ML pipeline").await.unwrap();

        assert_eq!(pipeline.name, "data_pipeline");
        assert_eq!(pipeline.status, PipelineStatus::Draft);
    }

    #[tokio::test]
    async fn test_add_task_with_dependencies() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("ml_pipe", "ML workflow").await.unwrap();

        let preprocess = orchestrator
            .add_task(pipeline.pipeline_id, "preprocess", TaskType::DataPreprocessing, vec![])
            .await
            .unwrap();
        let train = orchestrator
            .add_task(pipeline.pipeline_id, "train", TaskType::ModelTraining, vec![preprocess.task_id])
            .await
            .unwrap();

        assert_eq!(preprocess.task_name, "preprocess");
        assert_eq!(train.dependencies, vec![preprocess.task_id]);
    }

    #[tokio::test]
    async fn test_execute_pipeline_runs_tasks_in_dependency_order() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("train_pipe", "Training pipeline").await.unwrap();

        let preprocess = orchestrator
            .add_task(pipeline.pipeline_id, "preprocess", TaskType::DataPreprocessing, vec![])
            .await
            .unwrap();
        let train = orchestrator
            .add_task(pipeline.pipeline_id, "train", TaskType::ModelTraining, vec![preprocess.task_id])
            .await
            .unwrap();
        let _evaluate = orchestrator
            .add_task(pipeline.pipeline_id, "evaluate", TaskType::Evaluation, vec![train.task_id])
            .await
            .unwrap();

        let execution = orchestrator.execute_pipeline(pipeline.pipeline_id).await.unwrap();
        assert_eq!(execution.execution_status, ExecutionStatus::Succeeded);
        assert_eq!(execution.task_results.len(), 3);
        assert!(execution.task_results.iter().all(|(_, s)| *s == TaskStatus::Succeeded));
        assert_eq!(orchestrator.execution_count(), 1);

        let pipeline_after = orchestrator.pipelines.get(&pipeline.pipeline_id).unwrap();
        assert_eq!(pipeline_after.status, PipelineStatus::Completed);
    }

    #[tokio::test]
    async fn test_execute_pipeline_cascades_failure_from_missing_dependency() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("broken_pipe", "").await.unwrap();

        // Depends on a task id that was never added -- must not silently
        // succeed.
        let dangling_dep = Uuid::new_v4();
        orchestrator
            .add_task(pipeline.pipeline_id, "train", TaskType::ModelTraining, vec![dangling_dep])
            .await
            .unwrap();

        let execution = orchestrator.execute_pipeline(pipeline.pipeline_id).await.unwrap();
        assert_eq!(execution.execution_status, ExecutionStatus::Failed);
        assert_eq!(execution.task_results[0].1, TaskStatus::Skipped);
    }

    #[tokio::test]
    async fn test_execute_pipeline_detects_cyclic_dependency() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("cyclic_pipe", "").await.unwrap();

        // Can't construct a true cycle through add_task's borrow-checked
        // API in one shot (ids don't exist yet), so build one directly by
        // adding two tasks that depend on each other's *future* ids: add
        // task A with no deps, then task B depending on A, then mutate A's
        // dependencies via a fresh add_task call referencing B to fake a
        // cycle at the data level.
        let a = orchestrator.add_task(pipeline.pipeline_id, "a", TaskType::Custom("a".into()), vec![]).await.unwrap();
        let b = orchestrator
            .add_task(pipeline.pipeline_id, "b", TaskType::Custom("b".into()), vec![a.task_id])
            .await
            .unwrap();
        // Force a cycle: a now also depends on b.
        if let Some(mut entry) = orchestrator.tasks.get_mut(&a.task_id) {
            entry.dependencies.push(b.task_id);
        }

        let result = orchestrator.execute_pipeline(pipeline.pipeline_id).await;
        assert!(matches!(result, Err(PipelineError::CyclicDependency)));
    }

    #[tokio::test]
    async fn test_complete_execution_reflects_real_task_state() {
        let orchestrator = MLPipelineOrchestrator::new();
        let pipeline = orchestrator.create_pipeline("scheduled_pipe", "").await.unwrap();
        orchestrator
            .add_task(pipeline.pipeline_id, "only_task", TaskType::Evaluation, vec![])
            .await
            .unwrap();

        let execution = orchestrator.execute_pipeline(pipeline.pipeline_id).await.unwrap();
        orchestrator.complete_execution(execution.execution_id).await.unwrap();

        let stored = orchestrator.executions.get(&execution.execution_id).unwrap();
        assert_eq!(stored.execution_status, ExecutionStatus::Succeeded);
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
