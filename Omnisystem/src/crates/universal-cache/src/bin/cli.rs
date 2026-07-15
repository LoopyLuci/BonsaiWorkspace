//! Universal Cache CLI - exercises the cache, eviction policies, and cluster layer

use universal_cache::cache::Cache;
use universal_cache::cluster::{ClusterManager, ClusterNode};
use universal_cache::Policy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = Cache::<String, String>::builder()
        .capacity(3)
        .policy(Policy::Lru)
        .build();

    cache.insert("a".to_string(), "1".to_string());
    cache.insert("b".to_string(), "2".to_string());
    cache.insert("c".to_string(), "3".to_string());
    cache.insert("d".to_string(), "4".to_string()); // evicts "a"

    println!("cache size: {}", cache.len());
    println!("get(a) = {:?}", cache.get(&"a".to_string()));
    println!("get(d) = {:?}", cache.get(&"d".to_string()));

    let stats = cache.stats();
    println!("hit rate: {:.2}", stats.hit_rate());

    let manager = ClusterManager::new(2);
    let addr: std::net::SocketAddr = "127.0.0.1:6379".parse()?;
    manager.join(ClusterNode::new("node1".to_string(), addr));
    println!("cluster nodes: {}", manager.node_count());

    Ok(())
}
