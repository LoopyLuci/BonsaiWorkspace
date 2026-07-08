use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConsensusEngine {
    votes: Arc<DashMap<String, bool>>,
    quorum_size: usize,
}

impl ConsensusEngine {
    pub fn new(quorum_size: usize) -> Self {
        Self {
            votes: Arc::new(DashMap::new()),
            quorum_size,
        }
    }

    pub fn vote(&self, voter_id: String, vote: bool) -> Result<()> {
        self.votes.insert(voter_id, vote);
        tracing::info!("Vote recorded");
        Ok(())
    }

    pub fn check_consensus(&self) -> bool {
        let total_votes = self.votes.len();
        let positive_votes = self.votes.iter().filter(|v| *v.value()).count();
        
        total_votes >= self.quorum_size && positive_votes > (total_votes / 2)
    }

    pub fn reset(&self) {
        self.votes.clear();
    }

    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus() {
        let engine = ConsensusEngine::new(3);
        engine.vote("a1".to_string(), true).unwrap();
        engine.vote("a2".to_string(), true).unwrap();
        engine.vote("a3".to_string(), true).unwrap();
        assert_eq!(engine.vote_count(), 3);
    }
}
