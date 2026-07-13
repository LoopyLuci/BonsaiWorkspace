//! CLI demo: register a cluster and run a rolling-update rollout through completion.

use deployment_lifecycle::{Cluster, ClusterFederation, ClusterId, ClusterStatus, RolloutManager, RolloutStrategy};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let federation = ClusterFederation::new();
    let rollouts = RolloutManager::new();

    let cluster_id = federation
        .register_cluster(&Cluster {
            id: ClusterId("us-east-1".to_string()),
            name: "primary".to_string(),
            api_url: "https://us-east-1.example.internal".to_string(),
            status: ClusterStatus::Ready,
            capacity_replicas: 10,
            available_replicas: 10,
            region: "us-east-1".to_string(),
            created_at: Utc::now(),
            last_heartbeat: Utc::now(),
        })
        .await?;
    println!("Registered cluster: {}", cluster_id.0);

    let rollout_id = rollouts
        .start_rollout("checkout-service", RolloutStrategy::RollingUpdate)
        .await?;
    println!("Started rollout: {}", rollout_id.0);

    rollouts.update_progress(&rollout_id, 50).await?;
    rollouts.update_progress(&rollout_id, 100).await?;

    let rollout = rollouts.get_rollout(&rollout_id).await?;
    println!("Rollout status: {:?}, progress: {}%", rollout.status, rollout.progress_percent);

    let events = rollouts.get_rollout_events(&rollout_id).await?;
    println!("Events: {}", events.len());

    Ok(())
}
