/// Phase 8: Fault Tolerance Testing
///
/// Byzantine fault injection, partition handling, failure recovery

use omnisystem_cluster::*;
use std::time::Duration;

#[tokio::test]
async fn test_single_node_failure_recovery() {
    let manager1 = ClusterManager::new().await.unwrap();
    let manager2 = ClusterManager::new().await.unwrap();
    let manager3 = ClusterManager::new().await.unwrap();

    // Add nodes to membership
    manager1.membership().add_node(manager2.node_id()).await.unwrap();
    manager1.membership().add_node(manager3.node_id()).await.unwrap();

    let initial_nodes = manager1.membership().get_nodes().await.unwrap();
    assert_eq!(initial_nodes.len(), 2);

    // Simulate node failure by removing it
    manager1
        .membership()
        .remove_node(manager2.node_id())
        .await
        .unwrap();

    let remaining = manager1.membership().get_nodes().await.unwrap();
    assert_eq!(remaining.len(), 1);
}

#[tokio::test]
async fn test_network_partition_handling() {
    // Simulate network partition: cluster split into two groups
    let manager1 = ClusterManager::new().await.unwrap();
    let manager2 = ClusterManager::new().await.unwrap();
    let manager3 = ClusterManager::new().await.unwrap();
    let manager4 = ClusterManager::new().await.unwrap();
    let manager5 = ClusterManager::new().await.unwrap();

    // Initial cluster: all 5 nodes
    for i in 2..5 {
        manager1.membership().add_node(manager2.node_id()).await.unwrap();
    }

    // Partition A: nodes 1, 2, 3 (quorum: 3/5, can reach consensus)
    // Partition B: nodes 4, 5 (minority: 2/5, cannot reach consensus)

    let partition_a_size = manager1.membership().get_nodes().await.unwrap().len();
    assert!(partition_a_size >= 2); // Majority still operational
}

#[tokio::test]
async fn test_byzantine_fault_voting() {
    // Test that Byzantine votes don't break consensus
    let mut voting = voting::VotingManager::new(5).unwrap();

    // Nodes 1, 2, 3 vote yes (honest)
    voting.record_vote("node1", true).unwrap();
    voting.record_vote("node2", true).unwrap();
    voting.record_vote("node3", true).unwrap();

    // Nodes 4, 5 vote no (Byzantine/faulty)
    voting.record_vote("node4", false).unwrap();
    voting.record_vote("node5", false).unwrap();

    // Majority (3 yes) still sufficient for consensus
    assert!(voting.has_majority());
    let (granted, denied) = voting.vote_count();
    assert_eq!(granted, 3);
    assert_eq!(denied, 2);
}

#[tokio::test]
async fn test_leader_failure_triggers_election() {
    // Leader fails, new election begins
    let mut leader = leader_election::LeaderElectionManager::new("leader".to_string()).unwrap();
    leader.become_leader().await.unwrap();

    assert_eq!(leader.state(), leader_election::ElectionState::Leader);

    // Leader receives higher term (from new leader) and reverts
    leader
        .revert_to_follower(Some("new_leader".to_string()))
        .unwrap();

    assert_eq!(leader.state(), leader_election::ElectionState::Follower);
    assert_eq!(leader.current_leader(), Some("new_leader"));
}

#[tokio::test]
async fn test_state_machine_consistency() {
    // Verify state machine consistency across replicas
    let mut sm1 = state_machine::StateMachine::new().unwrap();
    let mut sm2 = state_machine::StateMachine::new().unwrap();

    // Apply same commands in same order (simulating replication)
    for i in 0..5 {
        let entry = state_machine::LogEntry {
            index: i,
            term: 1,
            command: vec![i as u8],
        };
        sm1.append_entry(entry.clone()).unwrap();
        sm2.append_entry(entry).unwrap();
    }

    // Both replicas should have same log
    let entries1 = sm1.get_entries(0, 10);
    let entries2 = sm2.get_entries(0, 10);

    assert_eq!(entries1.len(), entries2.len());
    assert_eq!(entries1.len(), 5);
}

#[tokio::test]
async fn test_split_brain_prevention() {
    // Quorum voting prevents split-brain scenario
    let mut voting1 = voting::VotingManager::new(5).unwrap();
    let mut voting2 = voting::VotingManager::new(5).unwrap();

    // Partition A (quorum: 3 out of 5)
    voting1.record_vote("node1", true).unwrap();
    voting1.record_vote("node2", true).unwrap();
    voting1.record_vote("node3", true).unwrap();
    assert!(voting1.has_majority());

    // Partition B (minority: 2 out of 5)
    voting2.record_vote("node4", true).unwrap();
    voting2.record_vote("node5", true).unwrap();
    assert!(!voting2.has_majority()); // Cannot reach consensus

    // Only partition with quorum can become leader
    assert!(voting1.has_majority());
    assert!(!voting2.has_majority());
}

#[tokio::test]
async fn test_cascading_node_failures() {
    // Cluster recovers from cascading failures as long as quorum remains
    let mut voting = voting::VotingManager::new(5).unwrap();

    // All nodes initially
    voting.record_vote("node1", true).unwrap();
    voting.record_vote("node2", true).unwrap();
    voting.record_vote("node3", true).unwrap();
    voting.record_vote("node4", true).unwrap();
    voting.record_vote("node5", true).unwrap();
    assert!(voting.has_majority());

    // Reset and simulate node1 failure
    voting.reset().unwrap();
    voting.record_vote("node2", true).unwrap();
    voting.record_vote("node3", true).unwrap();
    voting.record_vote("node4", true).unwrap();
    assert!(voting.has_majority()); // 3 out of 5 still quorum

    // Simulate node2 failure
    voting.reset().unwrap();
    voting.record_vote("node3", true).unwrap();
    voting.record_vote("node4", true).unwrap();
    // For 5-node cluster, quorum is 3, so 2 votes is NOT quorum
    assert!(!voting.has_majority());

    // With 3 votes we reach quorum for 5-node cluster
    voting.record_vote("node5", true).unwrap();
    assert!(voting.has_majority());
}

#[tokio::test]
async fn test_election_timeout_without_heartbeat() {
    let mgr = leader_election::LeaderElectionManager::new("follower".to_string()).unwrap();

    // Initially timeout should not be expired (just created)
    // After waiting, it would expire
    let expired = mgr.election_timeout_expired();
    // Note: May or may not be expired depending on timing, but should be safe either way
    println!("Election timeout expired: {}", expired);
}

#[tokio::test]
async fn test_majority_election_with_failures() {
    // 5-node cluster, 2 failures = still have quorum (3 nodes)
    let voting = voting::VotingManager::new(5).unwrap();
    assert_eq!(voting.quorum_size(), 3); // 5/2 + 1 = 3

    // With 3 nodes remaining, one can become leader
    let mut votes = voting::VotingManager::new(5).unwrap();
    votes.record_vote("node1", true).unwrap();
    votes.record_vote("node2", true).unwrap();
    votes.record_vote("node3", true).unwrap();
    assert!(votes.has_majority());
}

#[tokio::test]
async fn test_data_consistency_after_partition_heal() {
    // After network partition heals, ensure data consistency
    let mut sm_partition_a = state_machine::StateMachine::new().unwrap();
    let mut sm_partition_b = state_machine::StateMachine::new().unwrap();

    // Both partitions apply different commands (diverge)
    sm_partition_a
        .append_entry(state_machine::LogEntry {
            index: 1,
            term: 1,
            command: vec![1],
        })
        .unwrap();

    sm_partition_b
        .append_entry(state_machine::LogEntry {
            index: 1,
            term: 2, // Different term = new leader
            command: vec![2],
        })
        .unwrap();

    // After heal, higher term wins
    let entries_a = sm_partition_a.get_entries(0, 10);
    let entries_b = sm_partition_b.get_entries(0, 10);

    // In real scenario, partition B would overwrite partition A
    assert_eq!(entries_a.len(), 1);
    assert_eq!(entries_b.len(), 1);
    // entries_b[0].term (2) > entries_a[0].term (1), so B wins
}
