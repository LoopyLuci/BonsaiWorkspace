/// Rule voting and community proposal system for Omnisystem
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleProposal {
    pub proposal_id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub created_at: i64,
    pub status: ProposalStatus,
    pub votes_for: usize,
    pub votes_against: usize,
    pub total_voters: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Open,
    Closed,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter_id: String,
    pub proposal_id: String,
    pub in_favor: bool,
    pub timestamp: i64,
}

pub struct VotingSystem {
    proposals: Arc<RwLock<HashMap<String, RuleProposal>>>,
    votes: Arc<RwLock<Vec<Vote>>>,
}

impl VotingSystem {
    pub fn new() -> Self {
        Self {
            proposals: Arc::new(RwLock::new(HashMap::new())),
            votes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_proposal(
        &self,
        rule_id: String,
        title: String,
        description: String,
        author: String,
    ) -> Result<String> {
        let proposal_id = uuid::Uuid::new_v4().to_string();
        let proposal = RuleProposal {
            proposal_id: proposal_id.clone(),
            rule_id,
            title,
            description,
            author,
            created_at: chrono::Utc::now().timestamp(),
            status: ProposalStatus::Draft,
            votes_for: 0,
            votes_against: 0,
            total_voters: 0,
        };

        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal_id.clone(), proposal);

        tracing::info!("Created proposal: {}", proposal_id);
        Ok(proposal_id)
    }

    pub async fn submit_vote(&self, voter_id: String, proposal_id: String, in_favor: bool) -> Result<()> {
        let vote = Vote {
            voter_id,
            proposal_id: proposal_id.clone(),
            in_favor,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let mut votes = self.votes.write().await;
        votes.push(vote);

        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(&proposal_id) {
            proposal.total_voters += 1;
            if in_favor {
                proposal.votes_for += 1;
            } else {
                proposal.votes_against += 1;
            }
        }

        tracing::info!("Vote recorded for proposal: {}", proposal_id);
        Ok(())
    }

    pub async fn get_proposal(&self, proposal_id: &str) -> Result<Option<RuleProposal>> {
        let proposals = self.proposals.read().await;
        Ok(proposals.get(proposal_id).cloned())
    }

    pub async fn approve_proposal(&self, proposal_id: &str) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            proposal.status = ProposalStatus::Approved;
            tracing::info!("Proposal approved: {}", proposal_id);
        }
        Ok(())
    }

    pub async fn close_voting(&self, proposal_id: &str) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            proposal.status = ProposalStatus::Closed;
        }
        Ok(())
    }

    pub async fn get_all_proposals(&self) -> Result<Vec<RuleProposal>> {
        let proposals = self.proposals.read().await;
        Ok(proposals.values().cloned().collect())
    }

    pub async fn get_votes(&self, proposal_id: &str) -> Result<Vec<Vote>> {
        let votes = self.votes.read().await;
        Ok(votes.iter().filter(|v| v.proposal_id == proposal_id).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voting_engine_creation() {
        let tmp_dir = std::env::temp_dir().join("test_voting");
        let engine = VotingEngine::new(tmp_dir).await.unwrap();
        assert_eq!(engine.vote_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_vote_approval() {
        let tmp_dir = std::env::temp_dir().join("test_voting");
        let engine = VotingEngine::new(tmp_dir).await.unwrap();

        engine
            .vote(
                "prop-1",
                "voter-1",
                VoteType::Approve {
                    reason: "Good".to_string(),
                },
                "org-1".to_string(),
            )
            .await
            .unwrap();

        engine
            .vote(
                "prop-1",
                "voter-2",
                VoteType::Approve {
                    reason: "Looks good".to_string(),
                },
                "org-1".to_string(),
            )
            .await
            .unwrap();

        let summary = engine.get_summary("prop-1").await.unwrap();
        assert_eq!(summary.approvals, 2);
        assert_eq!(summary.rejections, 0);
        assert_eq!(summary.approval_rate(), 1.0);
    }

    #[test]
    fn test_vote_summary_approval_rate() {
        let summary = VoteSummary {
            proposal_id: "prop-1".to_string(),
            rule_id: "rule-1".to_string(),
            approvals: 2,
            rejections: 1,
            abstentions: 0,
        };

        assert!(summary.approval_rate() > 0.66);
    }
}
