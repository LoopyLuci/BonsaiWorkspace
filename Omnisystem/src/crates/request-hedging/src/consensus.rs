use crate::{ConsensusDecision, ConsensusType, HedgingError, HedgingResult, ServiceVote};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConsensusManager {
    decisions: Arc<DashMap<String, ConsensusDecision>>,
}

impl ConsensusManager {
    pub fn new() -> Self {
        Self {
            decisions: Arc::new(DashMap::new()),
        }
    }

    pub async fn submit_vote(&self, decision_id: &str, vote: &ServiceVote) -> HedgingResult<()> {
        let mut decision = self
            .decisions
            .entry(decision_id.to_string())
            .or_insert_with(|| ConsensusDecision {
                decision_id: decision_id.to_string(),
                consensus_type: ConsensusType::Majority,
                votes: Vec::new(),
                final_decision: false,
                confidence_level: 0.0,
            });

        decision.votes.push(vote.clone());
        Ok(())
    }

    pub async fn reach_consensus(
        &self,
        decision_id: &str,
        consensus_type: ConsensusType,
        total_services: usize,
    ) -> HedgingResult<bool> {
        if let Some(mut decision) = self.decisions.get_mut(decision_id) {
            decision.consensus_type = consensus_type;

            let votes_for = decision.votes.iter().filter(|v| v.decision).count();
            let total_votes = decision.votes.len();

            let (consensus_reached, final_decision) = match consensus_type {
                ConsensusType::Majority => {
                    (total_votes > 0, votes_for > total_votes / 2)
                }
                ConsensusType::Unanimous => {
                    let unanimous = votes_for == total_votes;
                    (unanimous && total_votes == total_services, unanimous)
                }
                ConsensusType::Quorum(quorum_size) => {
                    let has_quorum = total_votes >= quorum_size as usize;
                    (has_quorum, votes_for > (quorum_size as usize) / 2)
                }
            };

            if consensus_reached {
                decision.final_decision = final_decision;
                decision.confidence_level = (votes_for as f64 / total_votes as f64) * 100.0;
                Ok(decision.final_decision)
            } else {
                Err(HedgingError::ConsensusNotReached)
            }
        } else {
            Err(HedgingError::Internal("Decision not found".to_string()))
        }
    }

    pub async fn get_decision(&self, decision_id: &str) -> HedgingResult<ConsensusDecision> {
        self.decisions
            .get(decision_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| HedgingError::Internal("Decision not found".to_string()))
    }

    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }
}

impl Default for ConsensusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_vote() {
        let manager = ConsensusManager::new();
        let vote = ServiceVote {
            service_id: "service-1".to_string(),
            timestamp: Utc::now(),
            decision: true,
            confidence: 0.95,
        };

        manager.submit_vote("decision-1", &vote).await.unwrap();
        assert_eq!(manager.decision_count(), 1);
    }

    #[tokio::test]
    async fn test_majority_consensus() {
        let manager = ConsensusManager::new();

        let vote1 = ServiceVote {
            service_id: "service-1".to_string(),
            timestamp: Utc::now(),
            decision: true,
            confidence: 0.95,
        };

        let vote2 = ServiceVote {
            service_id: "service-2".to_string(),
            timestamp: Utc::now(),
            decision: true,
            confidence: 0.90,
        };

        let vote3 = ServiceVote {
            service_id: "service-3".to_string(),
            timestamp: Utc::now(),
            decision: false,
            confidence: 0.85,
        };

        manager.submit_vote("decision-1", &vote1).await.unwrap();
        manager.submit_vote("decision-1", &vote2).await.unwrap();
        manager.submit_vote("decision-1", &vote3).await.unwrap();

        let result = manager
            .reach_consensus("decision-1", ConsensusType::Majority, 3)
            .await
            .unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_quorum_consensus() {
        let manager = ConsensusManager::new();

        let vote1 = ServiceVote {
            service_id: "service-1".to_string(),
            timestamp: Utc::now(),
            decision: true,
            confidence: 0.95,
        };

        manager.submit_vote("decision-1", &vote1).await.unwrap();

        let result = manager
            .reach_consensus("decision-1", ConsensusType::Quorum(2), 4)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_decision() {
        let manager = ConsensusManager::new();
        let vote = ServiceVote {
            service_id: "service-1".to_string(),
            timestamp: Utc::now(),
            decision: true,
            confidence: 0.95,
        };

        manager.submit_vote("decision-1", &vote).await.unwrap();
        let decision = manager.get_decision("decision-1").await.unwrap();

        assert_eq!(decision.votes.len(), 1);
    }
}
