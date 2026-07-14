//! CLI demo: acquire and release a distributed lock.

use service_coordination::LockManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let locks = LockManager::new();

    let lock = locks
        .acquire_lock("lock-1", "resource-42", "worker-a", 5000)
        .await?;
    println!("Acquired lock {} on {} for {}", lock.lock_id, lock.resource_id, lock.owner);

    locks.release_lock("resource-42", "worker-a").await?;
    println!("Released lock on resource-42");

    Ok(())
}
