//! CLI

use resource_quotas::{QuotaManager, ResourceQuota};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = QuotaManager::new();
    let quota = ResourceQuota {
        quota_id: Uuid::new_v4(),
        tenant_id: "tenant1".to_string(),
        cpu_cores: 4,
        memory_mb: 8192,
        storage_gb: 100,
        network_mbps: 1000,
        active: true,
    };
    manager.set_quota(&quota).await?;

    let retrieved = manager.get_quota("tenant1").await?;
    println!("Quota for {}: {} cores", retrieved.tenant_id, retrieved.cpu_cores);
    println!("Total quotas: {}", manager.quota_count());
    Ok(())
}
