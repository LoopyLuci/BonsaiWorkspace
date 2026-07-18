//! CLI: exercise cache set/get/stats and cluster node registration.

use distributed_cache::{CacheConfig, DistributedCache, ReplicationManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = DistributedCache::new(&CacheConfig {
        max_size_bytes: 1024 * 1024,
        max_entries: 1000,
        eviction_policy: "lru".to_string(),
        ttl_seconds: 300,
    });

    cache.set("session:abc123", "user-42".to_string()).await?;
    let value = cache.get("session:abc123").await?;
    println!("session:abc123 = {}", value);

    let _ = cache.get("session:missing").await;
    let stats = cache.stats();
    println!(
        "stats: hits={} misses={} entries={} hit_rate={:.2}",
        stats.hits, stats.misses, stats.entries, stats.hit_rate
    );

    let replication = ReplicationManager::new();
    replication.add_node("node-1", "10.0.0.1:6379").await?;
    replication.add_node("node-2", "10.0.0.2:6379").await?;
    println!("replica nodes: {}", replication.node_count());

    Ok(())
}
