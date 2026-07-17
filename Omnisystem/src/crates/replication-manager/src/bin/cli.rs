//! CLI for exercising the replication-manager crate.

use replication_manager::{FailoverPolicy, ReplicaStatus, ReplicationManager};
use uuid::Uuid;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ReplicationManager::new();

    let replica1 = Uuid::new_v4();
    manager
        .register_replica(&ReplicaStatus { replica_id: replica1, node_name: "replica-1".to_string(), lag_bytes: 0, is_synced: true, last_sync: Utc::now() })
        .await?;
    let replica2 = Uuid::new_v4();
    manager
        .register_replica(&ReplicaStatus { replica_id: replica2, node_name: "replica-2".to_string(), lag_bytes: 500, is_synced: false, last_sync: Utc::now() })
        .await?;

    manager
        .register_failover_policy(&FailoverPolicy { policy_id: Uuid::new_v4(), name: "default".to_string(), auto_failover_enabled: true, failover_timeout_seconds: 15 })
        .await?;

    let check_id = manager.check_consistency("primary-1").await?;
    println!("Consistency check {} recorded", check_id);

    manager.trigger_failover("primary-1", "replica-1", "primary node health check failed").await?;

    let metrics = manager.get_metrics();
    println!(
        "Metrics: avg lag {}ms, sync rate {:.0}%, failovers {}, data loss events {}",
        metrics.replication_lag_ms,
        metrics.sync_success_rate * 100.0,
        metrics.failovers_total,
        metrics.data_loss_events
    );

    println!("Total replicas tracked: {}", manager.replica_count());
    Ok(())
}
