//! Governance Council for AHF Policy Management
//!
//! This module defines the governance council structure for approving policy updates
//! and making critical decisions about AHF configuration.

use crate::error::{VerificationError, VerificationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A member of the governance council
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilMember {
    /// Member ID
    pub id: Uuid,
    /// Member name
    pub name: String,
    /// Role in council
    pub role: String,
    /// Can approve policies?
    pub can_approve: bool,
    /// Can revoke capabilities?
    pub can_revoke: bool,
    /// Joined at
    pub joined_at: DateTime<Utc>,
}

/// A vote on a proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Vote {
    /// Vote for approval
    Approve,
    /// Vote against
    Reject,
    /// Abstain from voting
    Abstain,
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Proposal ID
    pub id: Uuid,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Proposed policy changes
    pub changes: serde_json::Value,
    /// Proposer
    pub proposer: Uuid,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Votes received
    pub votes: Vec<(Uuid, Vote)>,
    /// Approval threshold (0.0 to 1.0)
    pub approval_threshold: f64,
    /// Status
    pub status: ProposalStatus,
}

/// Status of a proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Proposal is open for voting
    Open,
    /// Proposal has been approved
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
}

impl Proposal {
    /// Create a new proposal
    pub fn new(
        title: String,
        description: String,
        changes: serde_json::Value,
        proposer: Uuid,
        approval_threshold: f64,
    ) -> Self {
        Proposal {
            id: Uuid::new_v4(),
            title,
            description,
            changes,
            proposer,
            created_at: Utc::now(),
            votes: Vec::new(),
            approval_threshold,
            status: ProposalStatus::Open,
        }
    }

    /// Add a vote
    pub fn add_vote(&mut self, voter: Uuid, vote: Vote) -> VerificationResult<()> {
        // Check if already voted
        if self.votes.iter().any(|(v, _)| v == &voter) {
            return Err(VerificationError::GovernanceError(
                "Member has already voted".to_string(),
            ));
        }

        self.votes.push((voter, vote));
        Ok(())
    }

    /// Calculate approval rate
    pub fn approval_rate(&self) -> f64 {
        if self.votes.is_empty() {
            return 0.0;
        }

        let approvals = self.votes.iter().filter(|(_, v)| *v == Vote::Approve).count();
        approvals as f64 / self.votes.len() as f64
    }

    /// Check if proposal is approved
    pub fn is_approved(&self) -> bool {
        self.approval_rate() >= self.approval_threshold
    }

    /// Finalize the proposal
    pub fn finalize(&mut self) -> VerificationResult<()> {
        if self.status != ProposalStatus::Open {
            return Err(VerificationError::GovernanceError(
                "Proposal is not open".to_string(),
            ));
        }

        if self.is_approved() {
            self.status = ProposalStatus::Approved;
            Ok(())
        } else {
            self.status = ProposalStatus::Rejected;
            Ok(())
        }
    }
}

/// Governance council
pub struct GovernanceCouncil {
    members: Vec<CouncilMember>,
    proposals: Vec<Proposal>,
}

impl GovernanceCouncil {
    /// Create a new governance council
    pub fn new() -> Self {
        GovernanceCouncil {
            members: Vec::new(),
            proposals: Vec::new(),
        }
    }

    /// Add a council member
    pub fn add_member(&mut self, member: CouncilMember) -> VerificationResult<()> {
        if self.members.iter().any(|m| m.id == member.id) {
            return Err(VerificationError::GovernanceError(
                "Member already exists".to_string(),
            ));
        }
        self.members.push(member);
        Ok(())
    }

    /// Get a council member
    pub fn get_member(&self, id: Uuid) -> Option<&CouncilMember> {
        self.members.iter().find(|m| m.id == id)
    }

    /// Submit a proposal
    pub fn submit_proposal(&mut self, proposal: Proposal) -> VerificationResult<()> {
        // Verify proposer is a council member
        if !self.members.iter().any(|m| m.id == proposal.proposer) {
            return Err(VerificationError::GovernanceError(
                "Proposer is not a council member".to_string(),
            ));
        }

        self.proposals.push(proposal);
        Ok(())
    }

    /// Vote on a proposal
    pub fn vote_on_proposal(
        &mut self,
        proposal_id: Uuid,
        voter_id: Uuid,
        vote: Vote,
    ) -> VerificationResult<()> {
        // Verify voter is a council member
        if !self.members.iter().any(|m| m.id == voter_id) {
            return Err(VerificationError::GovernanceError(
                "Voter is not a council member".to_string(),
            ));
        }

        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| {
                VerificationError::GovernanceError("Proposal not found".to_string())
            })?;

        proposal.add_vote(voter_id, vote)?;
        Ok(())
    }

    /// Finalize a proposal
    pub fn finalize_proposal(&mut self, proposal_id: Uuid) -> VerificationResult<()> {
        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| {
                VerificationError::GovernanceError("Proposal not found".to_string())
            })?;

        proposal.finalize()?;
        Ok(())
    }

    /// Get proposal
    pub fn get_proposal(&self, id: Uuid) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.id == id)
    }

    /// Get all proposals
    pub fn all_proposals(&self) -> &[Proposal] {
        &self.proposals
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

impl Default for GovernanceCouncil {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_council_creation() {
        let council = GovernanceCouncil::new();
        assert_eq!(council.member_count(), 0);
    }

    #[test]
    fn test_add_member() {
        let mut council = GovernanceCouncil::new();
        let member = CouncilMember {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            role: "validator".to_string(),
            can_approve: true,
            can_revoke: true,
            joined_at: Utc::now(),
        };

        assert!(council.add_member(member.clone()).is_ok());
        assert_eq!(council.member_count(), 1);
        assert!(council.add_member(member).is_err()); // Duplicate
    }

    #[test]
    fn test_proposal_creation() {
        let changes = serde_json::json!({"threshold": 0.95});
        let proposer = Uuid::new_v4();
        let proposal = Proposal::new(
            "Update threshold".to_string(),
            "Increase threshold to 0.95".to_string(),
            changes,
            proposer,
            0.75,
        );

        assert_eq!(proposal.status, ProposalStatus::Open);
        assert_eq!(proposal.approval_rate(), 0.0);
    }

    #[test]
    fn test_voting() {
        let changes = serde_json::json!({"threshold": 0.95});
        let proposer = Uuid::new_v4();
        let mut proposal = Proposal::new(
            "Update threshold".to_string(),
            "Increase threshold to 0.95".to_string(),
            changes,
            proposer,
            0.5,
        );

        let voter1 = Uuid::new_v4();
        let voter2 = Uuid::new_v4();

        assert!(proposal.add_vote(voter1, Vote::Approve).is_ok());
        assert!(proposal.add_vote(voter2, Vote::Reject).is_ok());
        assert!(proposal.add_vote(voter1, Vote::Approve).is_err()); // Already voted

        assert_eq!(proposal.approval_rate(), 0.5);
    }

    #[test]
    fn test_proposal_approval() {
        let changes = serde_json::json!({"threshold": 0.95});
        let proposer = Uuid::new_v4();
        let mut proposal = Proposal::new(
            "Update".to_string(),
            "Update".to_string(),
            changes,
            proposer,
            0.66,
        );

        let voter1 = Uuid::new_v4();
        let voter2 = Uuid::new_v4();
        let voter3 = Uuid::new_v4();

        proposal.add_vote(voter1, Vote::Approve).unwrap();
        proposal.add_vote(voter2, Vote::Approve).unwrap();
        proposal.add_vote(voter3, Vote::Reject).unwrap();

        assert!(proposal.is_approved());
        proposal.finalize().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }

    #[test]
    fn test_council_proposal_workflow() {
        let mut council = GovernanceCouncil::new();

        let member1 = CouncilMember {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
            role: "validator".to_string(),
            can_approve: true,
            can_revoke: true,
            joined_at: Utc::now(),
        };

        let member2 = CouncilMember {
            id: Uuid::new_v4(),
            name: "Bob".to_string(),
            role: "validator".to_string(),
            can_approve: true,
            can_revoke: true,
            joined_at: Utc::now(),
        };

        council.add_member(member1.clone()).unwrap();
        council.add_member(member2.clone()).unwrap();

        let changes = serde_json::json!({"threshold": 0.95});
        let proposal = Proposal::new(
            "Update".to_string(),
            "Update".to_string(),
            changes,
            member1.id,
            0.5,
        );

        let proposal_id = proposal.id;
        council.submit_proposal(proposal).unwrap();

        council.vote_on_proposal(proposal_id, member1.id, Vote::Approve).unwrap();
        council.vote_on_proposal(proposal_id, member2.id, Vote::Approve).unwrap();

        council.finalize_proposal(proposal_id).unwrap();

        let final_proposal = council.get_proposal(proposal_id).unwrap();
        assert_eq!(final_proposal.status, ProposalStatus::Approved);
    }
}
