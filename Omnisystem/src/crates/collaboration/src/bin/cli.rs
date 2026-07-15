//! Small demo CLI: creates a team rule profile, submits a rule proposal and
//! votes on it, and publishes/searches a shared rule library.

use collaboration::{SharedLibrary, SharedRule, TeamProfileManager, VotingSystem};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_dir = std::env::temp_dir().join("collaboration_cli_demo");
    let profile_manager = TeamProfileManager::new(db_dir).await?;

    let profile = profile_manager
        .create_profile(
            "team-platform".to_string(),
            "org-omnisystem".to_string(),
            "Platform Team Rules".to_string(),
            None,
        )
        .await?;
    println!("Created team profile: {} ({})", profile.name, profile.profile_id);

    let voting = VotingSystem::new();
    let proposal_id = voting
        .create_proposal(
            "unused-import".to_string(),
            "Flag unused imports".to_string(),
            "Warn when an import is never referenced in the file".to_string(),
            "author-1".to_string(),
        )
        .await?;
    voting.submit_vote("voter-1".to_string(), proposal_id.clone(), true).await?;
    voting.submit_vote("voter-2".to_string(), proposal_id.clone(), true).await?;
    voting.submit_vote("voter-3".to_string(), proposal_id.clone(), false).await?;

    let proposal = voting.get_proposal(&proposal_id).await?.expect("proposal exists");
    println!(
        "Proposal '{}': {} for / {} against ({} total voters)",
        proposal.title, proposal.votes_for, proposal.votes_against, proposal.total_voters
    );

    let library = SharedLibrary::new("collaboration_demo_library".to_string());
    library
        .publish_rule(SharedRule {
            rule_id: "unused-import".to_string(),
            name: "Unused Import".to_string(),
            pattern: "Detects unused import statements".to_string(),
            severity: "warning".to_string(),
            author: "author-1".to_string(),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            downloads: 0,
            rating: 0.0,
        })
        .await?;

    let results = library.search_rules("unused").await?;
    println!("Shared library search for 'unused': {} match(es)", results.len());
    for rule in &results {
        println!("  - {} ({})", rule.name, rule.rule_id);
    }

    Ok(())
}
