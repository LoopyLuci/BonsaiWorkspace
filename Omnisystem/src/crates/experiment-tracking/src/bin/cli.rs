//! CLI for exercising the experiment-tracking crate.

use experiment_tracking::ExperimentTracker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = ExperimentTracker::new();

    let experiment = tracker.create_experiment("lr_sweep", "Learning rate sweep").await?;
    println!("Created experiment '{}'", experiment.name);

    for lr in ["0.1", "0.01", "0.001"] {
        let run = tracker.start_run(experiment.experiment_id).await?;
        tracker.log_hyperparameter(run.run_id, "learning_rate", lr).await?;
        tracker.log_metric(run.run_id, "accuracy", 0.7 + lr.parse::<f64>().unwrap_or(0.0) * 10.0).await?;
        tracker.log_metric(run.run_id, "accuracy", 0.8 + lr.parse::<f64>().unwrap_or(0.0) * 5.0).await?;
        tracker.end_run(run.run_id, true).await?;
        println!("Run #{} (lr={}) completed", run.run_number, lr);
    }

    let best = tracker.get_best_run(experiment.experiment_id, "accuracy").await?;
    println!("Best run: #{} with accuracy {:?}", best.run_number, best.metrics.get("accuracy"));

    if let Some(history) = tracker.get_metric_history(best.run_id, "accuracy") {
        println!("Accuracy history for best run: {:?}", history.values.iter().map(|(_, v)| v).collect::<Vec<_>>());
    }

    println!("Total runs tracked: {}", tracker.run_count());
    Ok(())
}
