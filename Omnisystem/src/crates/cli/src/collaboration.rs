/// Collaboration command handlers for Omnisystem CLI
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamProfile {
    pub team_id: String,
    pub name: String,
    pub members: Vec<String>,
    pub created_at: i64,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub proposal_id: String,
    pub team_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub status: ProposalStatus,
    pub votes_for: usize,
    pub votes_against: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Open,
    Voting,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedRuleReference {
    pub rule_id: String,
    pub rule_name: String,
    pub version: String,
    pub shared_by: String,
    pub shared_at: i64,
}

pub struct CollaborationCommand {
    teams: Arc<RwLock<HashMap<String, TeamProfile>>>,
    proposals: Arc<RwLock<HashMap<String, Proposal>>>,
    shared_rules: Arc<RwLock<Vec<SharedRuleReference>>>,
}

impl CollaborationCommand {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            shared_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_team(&self, team_id: String, name: String) -> Result<String> {
        let team = TeamProfile {
            team_id: team_id.clone(),
            name,
            members: vec![],
            created_at: chrono::Utc::now().timestamp(),
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        let mut teams = self.teams.write().await;
        teams.insert(team_id.clone(), team);

        tracing::info!("Created team: {}", team_id);
        Ok(team_id)
    }

    pub async fn add_team_member(&self, team_id: &str, member_id: String) -> Result<()> {
        let mut teams = self.teams.write().await;
        if let Some(team) = teams.get_mut(team_id) {
            if !team.members.contains(&member_id) {
                team.members.push(member_id);
            }
        }

        tracing::info!("Added member to team: {}", team_id);
        Ok(())
    }

    pub async fn remove_team_member(&self, team_id: &str, member_id: &str) -> Result<()> {
        let mut teams = self.teams.write().await;
        if let Some(team) = teams.get_mut(team_id) {
            team.members.retain(|m| m != member_id);
        }

        tracing::info!("Removed member from team: {}", team_id);
        Ok(())
    }

    pub async fn list_teams(&self) -> Result<Vec<TeamProfile>> {
        let teams = self.teams.read().await;
        Ok(teams.values().cloned().collect())
    }

    pub async fn get_team(&self, team_id: &str) -> Result<Option<TeamProfile>> {
        let teams = self.teams.read().await;
        Ok(teams.get(team_id).cloned())
    }

    pub async fn delete_team(&self, team_id: &str) -> Result<()> {
        let mut teams = self.teams.write().await;
        teams.remove(team_id);

        tracing::info!("Deleted team: {}", team_id);
        Ok(())
    }

    pub async fn submit_proposal(&self, team_id: String, title: String, description: String, author: String) -> Result<String> {
        let proposal_id = uuid::Uuid::new_v4().to_string();
        let proposal = Proposal {
            proposal_id: proposal_id.clone(),
            team_id,
            title,
            description,
            author,
            status: ProposalStatus::Draft,
            votes_for: 0,
            votes_against: 0,
            created_at: chrono::Utc::now().timestamp(),
        };

        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal_id.clone(), proposal);

        tracing::info!("Submitted proposal: {}", proposal_id);
        Ok(proposal_id)
    }

    pub async fn open_voting(&self, proposal_id: &str) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            proposal.status = ProposalStatus::Voting;
        }

        tracing::info!("Opened voting for proposal: {}", proposal_id);
        Ok(())
    }

    pub async fn vote_on_proposal(&self, proposal_id: &str, in_favor: bool) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            if in_favor {
                proposal.votes_for += 1;
            } else {
                proposal.votes_against += 1;
            }
        }

        tracing::info!("Vote recorded for proposal: {}", proposal_id);
        Ok(())
    }

    pub async fn approve_proposal(&self, proposal_id: &str) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            proposal.status = ProposalStatus::Approved;
        }

        tracing::info!("Approved proposal: {}", proposal_id);
        Ok(())
    }

    pub async fn reject_proposal(&self, proposal_id: &str) -> Result<()> {
        let mut proposals = self.proposals.write().await;
        if let Some(proposal) = proposals.get_mut(proposal_id) {
            proposal.status = ProposalStatus::Rejected;
        }

        tracing::info!("Rejected proposal: {}", proposal_id);
        Ok(())
    }

    pub async fn get_proposal(&self, proposal_id: &str) -> Result<Option<Proposal>> {
        let proposals = self.proposals.read().await;
        Ok(proposals.get(proposal_id).cloned())
    }

    pub async fn list_proposals(&self, team_id: &str) -> Result<Vec<Proposal>> {
        let proposals = self.proposals.read().await;
        let team_proposals: Vec<Proposal> = proposals
            .values()
            .filter(|p| p.team_id == team_id)
            .cloned()
            .collect();
        Ok(team_proposals)
    }

    pub async fn share_rule(&self, rule_id: String, rule_name: String, version: String, shared_by: String) -> Result<()> {
        let reference = SharedRuleReference {
            rule_id,
            rule_name,
            version,
            shared_by,
            shared_at: chrono::Utc::now().timestamp(),
        };

        let mut rules = self.shared_rules.write().await;
        rules.push(reference);

        tracing::info!("Shared rule across teams");
        Ok(())
    }

    pub async fn list_shared_rules(&self) -> Result<Vec<SharedRuleReference>> {
        let rules = self.shared_rules.read().await;
        Ok(rules.clone())
    }

    pub async fn get_shared_rule(&self, rule_id: &str) -> Result<Option<SharedRuleReference>> {
        let rules = self.shared_rules.read().await;
        Ok(rules.iter().find(|r| r.rule_id == rule_id).cloned())
    }

    pub async fn get_team_members(&self, team_id: &str) -> Result<Vec<String>> {
        let teams = self.teams.read().await;
        if let Some(team) = teams.get(team_id) {
            Ok(team.members.clone())
        } else {
            Err(anyhow::anyhow!("Team not found: {}", team_id))
        }
    }

    pub async fn check_permission(&self, team_id: &str, permission: &str) -> Result<bool> {
        let teams = self.teams.read().await;
        if let Some(team) = teams.get(team_id) {
            Ok(team.permissions.contains(&permission.to_string()))
        } else {
            Err(anyhow::anyhow!("Team not found: {}", team_id))
        }
    }

    pub async fn get_audit_log(&self, team_id: &str) -> Result<Vec<String>> {
        let teams = self.teams.read().await;
        if let Some(_team) = teams.get(team_id) {
            let log = vec![
                format!("Team created at {}", chrono::Utc::now()),
                "Member added".to_string(),
            ];
            Ok(log)
        } else {
            Err(anyhow::anyhow!("Team not found: {}", team_id))
        }
    }
}

impl Default for CollaborationCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_team() {
        let cmd = CollaborationCommand::new();
        let result = cmd.create_team("team1".to_string(), "Team One".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_member() {
        let cmd = CollaborationCommand::new();
        cmd.create_team("team1".to_string(), "Team One".to_string()).await.unwrap();
        let result = cmd.add_team_member("team1", "user1".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_submit_proposal() {
        let cmd = CollaborationCommand::new();
        let result = cmd
            .submit_proposal(
                "team1".to_string(),
                "New Rule".to_string(),
                "Description".to_string(),
                "author".to_string(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_voting() {
        let cmd = CollaborationCommand::new();
        let proposal_id = cmd
            .submit_proposal(
                "team1".to_string(),
                "Test".to_string(),
                "Desc".to_string(),
                "author".to_string(),
            )
            .await
            .unwrap();

        let _ = cmd.open_voting(&proposal_id).await;
        let _ = cmd.vote_on_proposal(&proposal_id, true).await;

        let proposal = cmd.get_proposal(&proposal_id).await.unwrap().unwrap();
        assert_eq!(proposal.votes_for, 1);
    }

    #[tokio::test]
    async fn test_share_rule() {
        let cmd = CollaborationCommand::new();
        let result = cmd
            .share_rule(
                "rule1".to_string(),
                "Test Rule".to_string(),
                "1.0".to_string(),
                "user1".to_string(),
            )
            .await;
        assert!(result.is_ok());
    }
}
