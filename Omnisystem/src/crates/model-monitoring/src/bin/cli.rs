//! CLI: record performance for a model, check its health, and detect drift.

use chrono::Utc;
use model_monitoring::{ModelMonitor, ModelPerformance};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitor = ModelMonitor::new();
    let model_id = Uuid::new_v4();

    let perf = ModelPerformance {
        perf_id: Uuid::new_v4(),
        model_id,
        timestamp: Utc::now(),
        accuracy: 0.94,
        precision: 0.93,
        recall: 0.92,
        f1_score: 0.925,
    };
    monitor.record_performance(&perf).await?;

    let health = monitor.perform_health_check(model_id).await?;
    println!("model health: {:?}", health.status);

    let drift = monitor.detect_data_drift(model_id, "age", 0.45).await?;
    println!("data drift on 'age': score={:.2} detected={}", drift.drift_score, drift.drift_detected);

    let anomaly = monitor.detect_anomaly(model_id, "input-hash-abc123", 0.82).await?;
    println!("anomaly score: {:.2}", anomaly.anomaly_score);

    println!("total performance records: {}", monitor.metric_count());

    Ok(())
}
