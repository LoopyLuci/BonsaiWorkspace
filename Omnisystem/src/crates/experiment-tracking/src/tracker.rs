use crate::{Experiment, ExperimentStatus, ExperimentRun, RunStatus, Hyperparameter, MetricHistory, ExperimentError, ExperimentResult};
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

pub struct ExperimentTracker {
    experiments: Arc<DashMap<Uuid, Experiment>>,
    runs: Arc<DashMap<Uuid, ExperimentRun>>,
    hyperparams: Arc<DashMap<Uuid, Hyperparameter>>,
    metrics_history: Arc<DashMap<Uuid, MetricHistory>>,
}

impl ExperimentTracker {
    pub fn new() -> Self {
        Self {
            experiments: Arc::new(DashMap::new()),
            runs: Arc::new(DashMap::new()),
            hyperparams: Arc::new(DashMap::new()),
            metrics_history: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_experiment(&self, name: &str, description: &str) -> ExperimentResult<Experiment> {
        let experiment = Experiment {
            experiment_id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            created_at: Utc::now(),
            status: ExperimentStatus::Planning,
        };

        self.experiments.insert(experiment.experiment_id, experiment.clone());
        Ok(experiment)
    }

    pub async fn start_run(&self, experiment_id: Uuid) -> ExperimentResult<ExperimentRun> {
        if self.experiments.get(&experiment_id).is_none() {
            return Err(ExperimentError::ExperimentNotFound);
        }

        let run = ExperimentRun {
            run_id: Uuid::new_v4(),
            experiment_id,
            run_number: 1,
            start_time: Utc::now(),
            end_time: None,
            status: RunStatus::Running,
            metrics: HashMap::new(),
        };

        self.runs.insert(run.run_id, run.clone());
        Ok(run)
    }

    pub async fn log_metric(&self, run_id: Uuid, metric_name: &str, value: f64) -> ExperimentResult<()> {
        if let Some(mut entry) = self.runs.get_mut(&run_id) {
            entry.metrics.insert(metric_name.to_string(), value);
        } else {
            return Err(ExperimentError::RunNotFound);
        }

        Ok(())
    }

    pub async fn log_hyperparameter(&self, run_id: Uuid, param_name: &str, param_value: &str) -> ExperimentResult<Hyperparameter> {
        let hyperparam = Hyperparameter {
            param_id: Uuid::new_v4(),
            run_id,
            param_name: param_name.to_string(),
            param_value: param_value.to_string(),
            param_type: "string".to_string(),
        };

        self.hyperparams.insert(hyperparam.param_id, hyperparam.clone());
        Ok(hyperparam)
    }

    pub async fn end_run(&self, run_id: Uuid, success: bool) -> ExperimentResult<()> {
        if let Some(mut entry) = self.runs.get_mut(&run_id) {
            entry.end_time = Some(Utc::now());
            entry.status = if success { RunStatus::Succeeded } else { RunStatus::Failed };
        } else {
            return Err(ExperimentError::RunNotFound);
        }

        Ok(())
    }

    pub async fn get_best_run(&self, experiment_id: Uuid, metric_name: &str) -> ExperimentResult<ExperimentRun> {
        let mut best_run = None;
        let mut best_value = f64::NEG_INFINITY;

        for entry in self.runs.iter() {
            if entry.value().experiment_id == experiment_id {
                if let Some(value) = entry.value().metrics.get(metric_name) {
                    if *value > best_value {
                        best_value = *value;
                        best_run = Some(entry.value().clone());
                    }
                }
            }
        }

        best_run.ok_or(ExperimentError::RunNotFound)
    }

    pub fn run_count(&self) -> usize {
        self.runs.len()
    }
}

impl Default for ExperimentTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_experiment() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("neural_net_v1", "Testing NN architectures").await.unwrap();

        assert_eq!(exp.name, "neural_net_v1");
        assert_eq!(exp.status, ExperimentStatus::Planning);
    }

    #[tokio::test]
    async fn test_start_run() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("optimization", "Hyperparameter tuning").await.unwrap();

        let run = tracker.start_run(exp.experiment_id).await.unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(tracker.run_count(), 1);
    }

    #[tokio::test]
    async fn test_log_metric() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("ml_test", "Test metrics").await.unwrap();
        let run = tracker.start_run(exp.experiment_id).await.unwrap();

        tracker.log_metric(run.run_id, "accuracy", 0.95).await.unwrap();
        tracker.log_metric(run.run_id, "loss", 0.05).await.unwrap();
    }

    #[tokio::test]
    async fn test_log_hyperparameter() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("hp_tuning", "Tune parameters").await.unwrap();
        let run = tracker.start_run(exp.experiment_id).await.unwrap();

        let param = tracker.log_hyperparameter(run.run_id, "learning_rate", "0.001").await.unwrap();
        assert_eq!(param.param_value, "0.001");
    }
}
