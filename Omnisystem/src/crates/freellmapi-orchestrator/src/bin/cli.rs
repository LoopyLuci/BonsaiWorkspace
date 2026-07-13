//! Demo CLI: registers two nodes, marks one unhealthy, and exercises the real
//! ClusterManager and LoadBalancer selection logic.

use freellmapi_orchestrator::{BalancingStrategy, ClusterManager, LoadBalancer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cluster = ClusterManager::new();
    cluster.register_node("node-a", "127.0.0.1:9000", 100).await?;
    cluster.register_node("node-b", "127.0.0.1:9001", 100).await?;

    let selected = cluster.select_node().await?;
    println!("Selected node: {:?}", selected.map(|n| n.id));

    cluster.mark_unhealthy("node-a").await?;
    let healthy = cluster.get_healthy_nodes().await?;
    println!("Healthy nodes after marking node-a unhealthy: {:?}", healthy.iter().map(|n| &n.id).collect::<Vec<_>>());

    let nodes = cluster.get_all_nodes().await?;
    let mut lb = LoadBalancer::new(BalancingStrategy::RoundRobin);
    for _ in 0..3 {
        println!("Round-robin pick: {:?}", lb.select_node(&nodes).map(|n| n.id));
    }

    Ok(())
}
