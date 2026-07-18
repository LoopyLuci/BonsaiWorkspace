//! CLI: register a policy, feed it a few metric snapshots, and print the
//! resulting scaling decision and demand prediction.

use auto_scaler::{AutoScaler, MetricSnapshot, ScalingPolicy};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scaler = AutoScaler::new();

    let policy = ScalingPolicy {
        policy_id: Uuid::new_v4(),
        service_name: "api-service".to_string(),
        min_replicas: 2,
        max_replicas: 10,
        target_cpu_percent: 70,
        target_memory_percent: 75,
    };
    scaler.register_policy(&policy).await?;

    for (cpu, memory, requests) in [(85u32, 90u32, 12_000u64), (88, 92, 13_500)] {
        let metric = MetricSnapshot {
            snapshot_id: Uuid::new_v4(),
            service_name: "api-service".to_string(),
            cpu_percent: cpu,
            memory_percent: memory,
            request_count: requests,
            response_time_ms: 210,
        };
        scaler.record_metrics(&metric).await?;
    }

    let decision = scaler.evaluate_scaling("api-service", 2).await?;
    println!(
        "decision: {:?} {} -> {} ({})",
        decision.action, decision.current_replicas, decision.desired_replicas, decision.reason
    );

    let prediction = scaler.predict_demand("api-service").await?;
    println!(
        "prediction: {} replicas ({}% confidence, load={:.1})",
        prediction.predicted_replicas, prediction.confidence_percent, prediction.predicted_load
    );

    println!("total decisions recorded: {}", scaler.decision_count());

    Ok(())
}
