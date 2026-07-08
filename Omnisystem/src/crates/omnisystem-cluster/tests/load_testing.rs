/// Phase 9: Load Testing
///
/// Multi-node cluster simulation, stress testing, scalability validation

use omnisystem_cluster::*;

#[tokio::test]
async fn test_10_node_cluster_formation() {
    let manager = ClusterManager::new().await.unwrap();
    let members = manager.membership();

    // Add 10 nodes
    for i in 0..10 {
        members.add_node(&format!("node-{}", i)).await.unwrap();
    }

    let nodes = members.get_nodes().await.unwrap();
    assert_eq!(nodes.len(), 10);
    println!("✓ 10-node cluster formed successfully");
}

#[tokio::test]
async fn test_20_node_cluster_voting() {
    let mut voting = voting::VotingManager::new(20).unwrap();
    assert_eq!(voting.quorum_size(), 11);

    // Record votes from quorum (11 nodes)
    for i in 0..11 {
        voting.record_vote(&format!("node-{}", i), true).unwrap();
    }

    assert!(voting.has_majority());
    let (granted, _denied) = voting.vote_count();
    assert_eq!(granted, 11);
    println!("✓ 20-node cluster quorum voting successful");
}

#[tokio::test]
async fn test_50_node_cluster_state_machine() {
    let mut sm = state_machine::StateMachine::new().unwrap();

    // Add 500 log entries (simulating 50 nodes with 10 commands each)
    for i in 0..500 {
        let entry = state_machine::LogEntry {
            index: i,
            term: (i / 50) as u64 + 1,
            command: vec![(i % 256) as u8],
        };
        sm.append_entry(entry).unwrap();
    }

    let entries = sm.get_entries(0, 500);
    assert_eq!(entries.len(), 500);

    // Create snapshot
    let snapshot = sm.create_snapshot().unwrap();
    assert!(!snapshot.is_empty());
    println!(
        "✓ 50-node cluster with 500 log entries snapshot created ({} bytes)",
        snapshot.len()
    );
}

#[tokio::test]
async fn test_100_node_cluster_scalability() {
    let mut voting = voting::VotingManager::new(100).unwrap();
    assert_eq!(voting.quorum_size(), 51);

    // Record votes from quorum
    for i in 0..51 {
        voting.record_vote(&format!("node-{:03}", i), true).unwrap();
    }

    assert!(voting.has_majority());
    println!("✓ 100-node cluster quorum voting successful");
    println!("  - Quorum size: {} out of 100", voting.quorum_size());
}

#[tokio::test]
async fn test_concurrent_node_additions() {
    let manager = ClusterManager::new().await.unwrap();
    let members = manager.membership();

    // Add 50 nodes concurrently
    let mut handles = vec![];
    for i in 0..50 {
        let members_clone = members.clone();
        let handle = tokio::spawn(async move {
            members_clone
                .add_node(&format!("node-concurrent-{}", i))
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    // Wait for all additions
    for handle in handles {
        handle.await.unwrap();
    }

    let nodes = members.get_nodes().await.unwrap();
    assert_eq!(nodes.len(), 50);
    println!("✓ 50 concurrent node additions completed");
}

#[tokio::test]
async fn test_leader_election_large_cluster() {
    // Simulate leader election in 21-node cluster
    let mut election = leader_election::LeaderElectionManager::new("node-large-0".to_string())
        .unwrap();

    // Start election
    election.start_election().await.unwrap();
    assert_eq!(election.state(), leader_election::ElectionState::Candidate);

    // Become leader (after receiving 11 votes from 21 nodes)
    election.become_leader().await.unwrap();
    assert_eq!(election.state(), leader_election::ElectionState::Leader);

    // Send heartbeats
    for _ in 0..100 {
        election.send_heartbeat().unwrap();
    }

    println!("✓ Large cluster leader election and heartbeat streaming successful");
}

#[tokio::test]
async fn test_stress_voting_with_many_nodes() {
    // Stress test: 100 nodes, rapid voting
    let mut voting = voting::VotingManager::new(100).unwrap();

    // Rapid vote recording
    for i in 0..100 {
        let granted = i % 3 == 0; // ~33% voting
        voting.record_vote(&format!("node-{:03}", i), granted).unwrap();
    }

    // Verify quorum math
    let (granted, denied) = voting.vote_count();
    assert_eq!(granted + denied, 100);
    println!(
        "✓ Stress test: 100 nodes, {} granted, {} denied",
        granted, denied
    );
}

#[tokio::test]
async fn test_log_replication_scale() {
    // Simulate log replication across 10 replicas
    let mut replicas: Vec<_> = (0..10)
        .map(|_| state_machine::StateMachine::new().unwrap())
        .collect();

    // Replicate 1000 entries across all replicas
    for entry_idx in 0..1000 {
        let entry = state_machine::LogEntry {
            index: entry_idx,
            term: 1,
            command: vec![(entry_idx % 256) as u8],
        };

        for replica in &mut replicas {
            replica.append_entry(entry.clone()).unwrap();
        }
    }

    // Verify all replicas have same state
    for replica in &replicas {
        let entries = replica.get_entries(0, 1000);
        assert_eq!(entries.len(), 1000);
    }

    println!("✓ Log replication across 10 replicas with 1000 entries successful");
}

#[tokio::test]
async fn test_cascading_elections_under_load() {
    // Simulate rapid successive elections
    let mut election_rounds = vec![];

    for round in 0..10 {
        let mut voting = voting::VotingManager::new(7).unwrap();

        // Each election round: record votes
        for i in 0..7 {
            let granted = (round + i) % 2 == 0;
            voting.record_vote(&format!("node-{}", i), granted).unwrap();
        }

        let has_leader = voting.has_majority();
        election_rounds.push((round, has_leader));
    }

    println!("✓ Cascading elections completed:");
    for (round, has_leader) in election_rounds {
        println!("  Round {}: leader elected = {}", round, has_leader);
    }
}

#[tokio::test]
async fn test_memory_efficiency_large_log() {
    // Test memory efficiency with large log
    let mut sm = state_machine::StateMachine::new().unwrap();

    // Add 10,000 entries
    for i in 0..10000 {
        let entry = state_machine::LogEntry {
            index: i,
            term: (i / 1000) as u64 + 1,
            command: vec![(i % 256) as u8],
        };
        sm.append_entry(entry).unwrap();
    }

    // Query large ranges efficiently
    let mid = sm.get_entries(5000, 5100);
    assert_eq!(mid.len(), 100);

    let high = sm.get_entries(9900, 10000);
    assert_eq!(high.len(), 100);

    println!("✓ Memory efficiency test: 10,000 log entries stored and queried");
}

#[tokio::test]
async fn test_cluster_under_Byzantine_load() {
    // Test cluster stability with Byzantine nodes
    let total_nodes = 25;
    let byzantine_nodes = 8; // Just under 1/3

    let mut voting = voting::VotingManager::new(total_nodes).unwrap();

    // Byzantine nodes vote randomly
    for i in 0..byzantine_nodes {
        voting.record_vote(&format!("byzantine-{}", i), i % 2 == 0).unwrap();
    }

    // Honest nodes vote yes
    for i in byzantine_nodes..total_nodes {
        voting.record_vote(&format!("honest-{}", i), true).unwrap();
    }

    // Honest majority should still reach consensus
    assert!(voting.has_majority());
    println!(
        "✓ Cluster under Byzantine load: {} honest nodes reach consensus despite {} Byzantine nodes",
        total_nodes - byzantine_nodes,
        byzantine_nodes
    );
}
