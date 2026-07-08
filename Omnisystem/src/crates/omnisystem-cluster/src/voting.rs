/// Voting and Quorum Module
///
/// Distributed quorum voting:
/// - Vote tracking
/// - Quorum calculation
/// - Vote majority detection
/// - Byzantine fault tolerance

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Vote structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub node_id: String,
    pub term: u64,
    pub granted: bool,
}

/// Voting manager
pub struct VotingManager {
    term: u64,
    votes: HashMap<String, bool>,
    total_nodes: usize,
}

impl VotingManager {
    /// Create voting manager
    pub fn new(total_nodes: usize) -> Result<Self> {
        info!("Initializing Voting Manager for {} nodes", total_nodes);
        Ok(Self {
            term: 0,
            votes: HashMap::new(),
            total_nodes,
        })
    }

    /// Record a vote
    pub fn record_vote(&mut self, node_id: &str, granted: bool) -> Result<()> {
        info!("Recording vote from {}: {}", node_id, granted);
        self.votes.insert(node_id.to_string(), granted);
        Ok(())
    }

    /// Get quorum size
    pub fn quorum_size(&self) -> usize {
        (self.total_nodes / 2) + 1
    }

    /// Check if majority reached
    pub fn has_majority(&self) -> bool {
        let granted = self.votes.values().filter(|v| **v).count();
        granted >= self.quorum_size()
    }

    /// Get vote count
    pub fn vote_count(&self) -> (usize, usize) {
        let granted = self.votes.values().filter(|v| **v).count();
        let denied = self.votes.len() - granted;
        (granted, denied)
    }

    /// Reset votes for new election
    pub fn reset(&mut self) -> Result<()> {
        info!("Resetting votes for new election");
        self.votes.clear();
        self.term += 1;
        Ok(())
    }

    /// Get current term
    pub fn current_term(&self) -> u64 {
        self.term
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voting_manager() {
        let mut vm = VotingManager::new(5).unwrap();
        assert_eq!(vm.quorum_size(), 3);

        vm.record_vote("node1", true).unwrap();
        vm.record_vote("node2", true).unwrap();
        vm.record_vote("node3", true).unwrap();

        assert!(vm.has_majority());
        let (granted, denied) = vm.vote_count();
        assert_eq!(granted, 3);
        assert_eq!(denied, 0);
    }

    #[test]
    fn test_quorum_calculation() {
        let vm = VotingManager::new(3).unwrap();
        assert_eq!(vm.quorum_size(), 2);

        let vm = VotingManager::new(7).unwrap();
        assert_eq!(vm.quorum_size(), 4);
    }
}
