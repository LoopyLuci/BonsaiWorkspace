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
    /// Keyed by (run_id, metric_name) so repeated log_metric calls for the
    /// same metric accumulate into one time series.
    metrics_history: Arc<DashMap<(Uuid, String), MetricHistory>>,
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

        // run_number must increment per-experiment so repeated runs of the
        // same experiment are distinguishable (a fixed 1 for every run
        // would make every run of an experiment collide at "run 1").
        let run_number = self
            .runs
            .iter()
            .filter(|e| e.value().experiment_id == experiment_id)
            .map(|e| e.value().run_number)
            .max()
            .map(|n| n + 1)
            .unwrap_or(1);

        let run = ExperimentRun {
            run_id: Uuid::new_v4(),
            experiment_id,
            run_number,
            start_time: Utc::now(),
            end_time: None,
            status: RunStatus::Running,
            metrics: HashMap::new(),
        };

        self.runs.insert(run.run_id, run.clone());
        Ok(run)
    }

    /// Log a metric value for a run. Updates the run's latest-value map
    /// and appends to that metric's full time series so history isn't
    /// lost when the same metric is logged repeatedly (e.g. loss curves).
    pub async fn log_metric(&self, run_id: Uuid, metric_name: &str, value: f64) -> ExperimentResult<()> {
        let now = Utc::now();
        if let Some(mut entry) = self.runs.get_mut(&run_id) {
            entry.metrics.insert(metric_name.to_string(), value);
        } else {
            return Err(ExperimentError::RunNotFound);
        }

        let history_key = (run_id, metric_name.to_string());
        let mut history_entry = self
            .metrics_history
            .entry(history_key.clone())
            .or_insert_with(|| MetricHistory {
                history_id: Uuid::new_v4(),
                run_id,
                metric_name: metric_name.to_string(),
                values: Vec::new(),
            });
        history_entry.values.push((now, value));

        Ok(())
    }

    /// Retrieve the full logged time series for a run's metric.
    pub fn get_metric_history(&self, run_id: Uuid, metric_name: &str) -> Option<MetricHistory> {
        self.metrics_history.get(&(run_id, metric_name.to_string())).map(|h| h.value().clone())
    }

    /// Infer a param's type from its string representation instead of
    /// hardcoding every hyperparameter as "string" regardless of what was
    /// actually passed in.
    fn infer_param_type(value: &str) -> &'static str {
        if value.parse::<i64>().is_ok() {
            "int"
        } else if value.parse::<f64>().is_ok() {
            "float"
        } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
            "bool"
        } else {
            "string"
        }
    }

    pub async fn log_hyperparameter(&self, run_id: Uuid, param_name: &str, param_value: &str) -> ExperimentResult<Hyperparameter> {
        let hyperparam = Hyperparameter {
            param_id: Uuid::new_v4(),
            run_id,
            param_name: param_name.to_string(),
            param_value: param_value.to_string(),
            param_type: Self::infer_param_type(param_value).to_string(),
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
    async fn test_log_metric_preserves_full_history() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("ml_test", "Test metric history").await.unwrap();
        let run = tracker.start_run(exp.experiment_id).await.unwrap();

        tracker.log_metric(run.run_id, "loss", 1.0).await.unwrap();
        tracker.log_metric(run.run_id, "loss", 0.5).await.unwrap();
        tracker.log_metric(run.run_id, "loss", 0.1).await.unwrap();

        let history = tracker.get_metric_history(run.run_id, "loss").unwrap();
        assert_eq!(history.values.len(), 3, "each log_metric call must be preserved, not overwrite the previous value");
        let values: Vec<f64> = history.values.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, vec![1.0, 0.5, 0.1]);
    }

    #[tokio::test]
    async fn test_start_run_increments_run_number_per_experiment() {
        let tracker = ExperimentTracker::new();
        let exp_a = tracker.create_experiment("exp_a", "").await.unwrap();
        let exp_b = tracker.create_experiment("exp_b", "").await.unwrap();

        let a1 = tracker.start_run(exp_a.experiment_id).await.unwrap();
        let a2 = tracker.start_run(exp_a.experiment_id).await.unwrap();
        let a3 = tracker.start_run(exp_a.experiment_id).await.unwrap();
        // A different experiment's run numbering must be independent.
        let b1 = tracker.start_run(exp_b.experiment_id).await.unwrap();

        assert_eq!(a1.run_number, 1);
        assert_eq!(a2.run_number, 2);
        assert_eq!(a3.run_number, 3);
        assert_eq!(b1.run_number, 1);
    }

    #[tokio::test]
    async fn test_log_hyperparameter() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("hp_tuning", "Tune parameters").await.unwrap();
        let run = tracker.start_run(exp.experiment_id).await.unwrap();

        let param = tracker.log_hyperparameter(run.run_id, "learning_rate", "0.001").await.unwrap();
        assert_eq!(param.param_value, "0.001");
    }

    #[tokio::test]
    async fn test_log_hyperparameter_infers_real_type() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("hp_tuning", "").await.unwrap();
        let run = tracker.start_run(exp.experiment_id).await.unwrap();

        let int_param = tracker.log_hyperparameter(run.run_id, "batch_size", "32").await.unwrap();
        assert_eq!(int_param.param_type, "int");

        let float_param = tracker.log_hyperparameter(run.run_id, "learning_rate", "0.001").await.unwrap();
        assert_eq!(float_param.param_type, "float");

        let bool_param = tracker.log_hyperparameter(run.run_id, "use_dropout", "true").await.unwrap();
        assert_eq!(bool_param.param_type, "bool");

        let string_param = tracker.log_hyperparameter(run.run_id, "optimizer", "adam").await.unwrap();
        assert_eq!(string_param.param_type, "string");
    }

    #[tokio::test]
    async fn test_get_best_run() {
        let tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment("best_run_test", "").await.unwrap();

        let run1 = tracker.start_run(exp.experiment_id).await.unwrap();
        tracker.log_metric(run1.run_id, "accuracy", 0.80).await.unwrap();
        let run2 = tracker.start_run(exp.experiment_id).await.unwrap();
        tracker.log_metric(run2.run_id, "accuracy", 0.93).await.unwrap();

        let best = tracker.get_best_run(exp.experiment_id, "accuracy").await.unwrap();
        assert_eq!(best.run_id, run2.run_id);
    }
}
