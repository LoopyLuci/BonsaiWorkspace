//! CLI demo for omnisystem-cluster: exercises membership, consensus,
//! leader election, and replication against real (in-process) managers.

use omnisystem_cluster::{ConsensusEngine, LeaderElectionManager, MembershipManager, ReplicationManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let membership = MembershipManager::new().await?;
    membership.add_node("node-1").await?;
    membership.add_node("node-2").await?;
    membership.add_node("node-3").await?;
    let nodes = membership.get_nodes().await?;
    println!("Cluster membership: {:?}", nodes);

    let mut consensus = ConsensusEngine::new()?;
    consensus.start_election().await?;
    println!("Consensus term after election: {}", consensus.get_term());

    let mut election = LeaderElectionManager::new("node-1".to_string())?;
    election.start_election().await?;
    election.become_leader().await?;
    println!(
        "Leader election state: {:?}, leader: {:?}",
        election.state(),
        election.current_leader()
    );

    let replication = ReplicationManager::new()?;
    for node in &nodes {
        if node != "node-1" {
            replication
                .replicate_to_node(node, b"cluster-state-snapshot")
                .await?;
        }
    }
    let status = replication.get_replication_status().await?;
    println!(
        "Replication status: {} replicas, {}ms lag",
        status.replicas, status.lag_ms
    );

    Ok(())
}
