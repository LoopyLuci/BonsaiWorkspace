/// Integration tests for Cluster Coordination

use omnisystem_cluster::*;

#[tokio::test]
async fn test_cluster_initialization() {
    let manager = ClusterManager::new().await.unwrap();
    assert!(!manager.node_id().is_empty());

    let status = manager.get_status().await.unwrap();
    assert_eq!(status.node_id, manager.node_id());
    assert!(!status.is_leader);
    assert_eq!(status.term, 0);
}

#[tokio::test]
async fn test_membership_operations() {
    let manager = ClusterManager::new().await.unwrap();
    let membership = manager.membership();

    membership.add_node("node1").await.unwrap();
    membership.add_node("node2").await.unwrap();
    membership.add_node("node3").await.unwrap();

    let nodes = membership.get_nodes().await.unwrap();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.contains(&"node1".to_string()));
}

#[tokio::test]
async fn test_leader_election_state_machine() {
    let mut mgr = leader_election::LeaderElectionManager::new("node1".to_string()).unwrap();

    assert_eq!(mgr.state(), leader_election::ElectionState::Follower);

    mgr.start_election().await.unwrap();
    assert_eq!(mgr.state(), leader_election::ElectionState::Candidate);

    mgr.become_leader().await.unwrap();
    assert_eq!(mgr.state(), leader_election::ElectionState::Leader);

    mgr.revert_to_follower(Some("node2".to_string()))
        .unwrap();
    assert_eq!(mgr.state(), leader_election::ElectionState::Follower);
    assert_eq!(mgr.current_leader(), Some("node2"));
}

#[test]
fn test_voting_quorum() {
    let mut vm = voting::VotingManager::new(5).unwrap();
    assert_eq!(vm.quorum_size(), 3);

    vm.record_vote("node1", true).unwrap();
    vm.record_vote("node2", true).unwrap();
    assert!(!vm.has_majority());

    vm.record_vote("node3", true).unwrap();
    assert!(vm.has_majority());

    let (granted, denied) = vm.vote_count();
    assert_eq!(granted, 3);
    assert_eq!(denied, 0);
}

#[test]
fn test_state_machine_operations() {
    let mut sm = state_machine::StateMachine::new().unwrap();

    let entry1 = state_machine::LogEntry {
        index: 1,
        term: 1,
        command: vec![1, 2, 3],
    };

    sm.append_entry(entry1).unwrap();
    assert_eq!(sm.get_entries(0, 10).len(), 1);

    let entry2 = state_machine::LogEntry {
        index: 2,
        term: 1,
        command: vec![4, 5, 6],
    };

    sm.append_entry(entry2).unwrap();
    assert_eq!(sm.get_entries(0, 10).len(), 2);
    assert_eq!(sm.get_last_log_term(), 1);
}

#[tokio::test]
async fn test_distributed_consensus_simulation() {
    // Simulate a 3-node cluster reaching consensus
    let mut voting_mgr = voting::VotingManager::new(3).unwrap();
    let mut election_mgr =
        leader_election::LeaderElectionManager::new("leader".to_string()).unwrap();

    // Follower becomes candidate
    election_mgr.start_election().await.unwrap();
    assert_eq!(
        election_mgr.state(),
        leader_election::ElectionState::Candidate
    );

    // Collect votes from other nodes
    voting_mgr.record_vote("node1", true).unwrap();
    voting_mgr.record_vote("node2", true).unwrap();

    // Majority reached
    assert!(voting_mgr.has_majority());

    // Candidate becomes leader
    election_mgr.become_leader().await.unwrap();
    assert_eq!(
        election_mgr.state(),
        leader_election::ElectionState::Leader
    );

    // Leader sends heartbeat
    election_mgr.send_heartbeat().unwrap();
}

#[tokio::test]
async fn test_cluster_with_multiple_managers() {
    let manager1 = ClusterManager::new().await.unwrap();
    let manager2 = ClusterManager::new().await.unwrap();
    let manager3 = ClusterManager::new().await.unwrap();

    let node1 = manager1.node_id().to_string();
    let node2 = manager2.node_id().to_string();
    let node3 = manager3.node_id().to_string();

    // Add all nodes to membership
    manager1.membership().add_node(&node2).await.unwrap();
    manager1.membership().add_node(&node3).await.unwrap();

    let nodes = manager1.membership().get_nodes().await.unwrap();
    assert_eq!(nodes.len(), 2);
}
