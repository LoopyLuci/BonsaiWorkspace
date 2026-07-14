/// Leader Election Module
///
/// Distributed leader election:
/// - Election state machine (Follower/Candidate/Leader)
/// - Election timeout
/// - Vote solicitation
/// - Leader heartbeat

use crate::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use tracing::info;

/// Election state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElectionState {
    Follower,
    Candidate,
    Leader,
}

/// Leader election manager
pub struct LeaderElectionManager {
    state: ElectionState,
    current_leader: Option<String>,
    last_heartbeat: SystemTime,
    election_timeout: Duration,
    node_id: String,
}

impl LeaderElectionManager {
    /// Create leader election manager
    pub fn new(node_id: String) -> Result<Self> {
        info!("Initializing Leader Election Manager for node: {}", node_id);
        Ok(Self {
            state: ElectionState::Follower,
            current_leader: None,
            last_heartbeat: SystemTime::now(),
            election_timeout: Duration::from_millis(1500),
            node_id,
        })
    }

    /// Get current state
    pub fn state(&self) -> ElectionState {
        self.state
    }

    /// Get current leader
    pub fn current_leader(&self) -> Option<&str> {
        self.current_leader.as_deref()
    }

    /// Check if election timeout expired
    pub fn election_timeout_expired(&self) -> bool {
        match self.last_heartbeat.elapsed() {
            Ok(elapsed) => elapsed > self.election_timeout,
            Err(_) => false,
        }
    }

    /// Start election (Follower → Candidate)
    pub async fn start_election(&mut self) -> Result<()> {
        info!("Starting election from node: {}", self.node_id);
        self.state = ElectionState::Candidate;
        Ok(())
    }

    /// Become leader (Candidate → Leader)
    pub async fn become_leader(&mut self) -> Result<()> {
        info!("Node {} becoming leader", self.node_id);
        self.state = ElectionState::Leader;
        self.current_leader = Some(self.node_id.clone());
        self.last_heartbeat = SystemTime::now();
        Ok(())
    }

    /// Revert to follower (any state → Follower)
    pub fn revert_to_follower(&mut self, leader_id: Option<String>) -> Result<()> {
        info!("Node {} reverting to follower", self.node_id);
        self.state = ElectionState::Follower;
        self.current_leader = leader_id;
        self.last_heartbeat = SystemTime::now();
        Ok(())
    }

    /// Send heartbeat (leader only)
    pub fn send_heartbeat(&mut self) -> Result<()> {
        if self.state != ElectionState::Leader {
            return Err(crate::ClusterError::Consensus(
                "Only leader can send heartbeat".to_string(),
            ));
        }
        info!("Leader {} sending heartbeat", self.node_id);
        self.last_heartbeat = SystemTime::now();
        Ok(())
    }

    /// Record heartbeat receipt
    pub fn record_heartbeat(&mut self) -> Result<()> {
        if self.state == ElectionState::Follower {
            self.last_heartbeat = SystemTime::now();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_leader_election() {
        let mut mgr = LeaderElectionManager::new("node1".to_string()).unwrap();

        assert_eq!(mgr.state(), ElectionState::Follower);
        assert_eq!(mgr.current_leader(), None);

        mgr.start_election().await.unwrap();
        assert_eq!(mgr.state(), ElectionState::Candidate);

        mgr.become_leader().await.unwrap();
        assert_eq!(mgr.state(), ElectionState::Leader);
        assert_eq!(mgr.current_leader(), Some("node1"));
    }

    #[test]
    fn test_election_timeout() {
        let mgr = LeaderElectionManager::new("node1".to_string()).unwrap();
        assert!(!mgr.election_timeout_expired());
    }

    #[tokio::test]
    async fn test_revert_to_follower() {
        let mut mgr = LeaderElectionManager::new("node1".to_string()).unwrap();
        mgr.become_leader().await.unwrap();
        assert_eq!(mgr.state(), ElectionState::Leader);

        mgr.revert_to_follower(Some("node2".to_string()))
            .unwrap();
        assert_eq!(mgr.state(), ElectionState::Follower);
        assert_eq!(mgr.current_leader(), Some("node2"));
    }
}
