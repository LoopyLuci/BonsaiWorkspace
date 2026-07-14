//! CLI demo for mesh-network: registers two nodes on a MeshPlatform,
//! computes routes, and prints real network stats.

use mesh_network::coordination::MeshNode;
use mesh_network::{MeshConfig, MeshPlatform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = MeshPlatform::new(MeshConfig::default());

    let mut node_a = MeshNode::new(vec![1, 2, 3, 4], "node-a".to_string());
    node_a.mark_online();
    let mut node_b = MeshNode::new(vec![5, 6, 7, 8], "node-b".to_string());
    node_b.mark_online();

    platform.register_node(node_a)?;
    platform.register_node(node_b)?;

    println!("Registered nodes: {}", platform.list_nodes().len());

    platform.compute_routes();

    let stats = platform.get_network_stats();
    println!(
        "Nodes: {} total, {} online. ACL rules: {}",
        stats.total_nodes, stats.online_nodes, stats.acl_rules
    );

    let health = platform.network_health();
    println!(
        "Network healthy: {} (online ratio {:.2})",
        health.is_healthy, health.online_ratio
    );

    Ok(())
}
