/// Team rule profiles - allow teams to override global rule configurations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Team-specific rule profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRuleProfile {
    pub profile_id: String,
    pub team_id: String,
    pub organization_id: String,
    pub name: String,

    /// Rules and their team-specific configuration
    pub rules: HashMap<String, TeamRuleConfig>,

    /// Inherit from parent profile (org-level)
    pub inherit_from: Option<String>,

    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

/// Team-specific configuration for a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRuleConfig {
    pub rule_id: String,
    pub enabled: bool,
    pub severity: String,  // Error, Warning, Hint, Note
    pub confidence_threshold: f32,
    pub metadata: HashMap<String, String>,
}

impl TeamRuleProfile {
    pub fn new(
        team_id: String,
        organization_id: String,
        name: String,
        inherit_from: Option<String>,
    ) -> Self {
        let profile_id = format!("profile-{}-{}", team_id, uuid::Uuid::new_v4());

        Self {
            profile_id,
            team_id,
            organization_id,
            name,
            rules: HashMap::new(),
            inherit_from,
            created_at: Utc::now(),
            last_modified: Utc::now(),
        }
    }

    /// Add or update a rule configuration in this profile.
    pub fn set_rule_config(&mut self, config: TeamRuleConfig) {
        self.rules.insert(config.rule_id.clone(), config);
        self.last_modified = Utc::now();
    }

    /// Get a rule configuration (or None if not overridden).
    pub fn get_rule_config(&self, rule_id: &str) -> Option<&TeamRuleConfig> {
        self.rules.get(rule_id)
    }

    /// Remove a rule from this profile (revert to defaults).
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.remove(rule_id);
        self.last_modified = Utc::now();
    }
}

/// Manager for team rule profiles (persistence + retrieval).
pub struct TeamProfileManager {
    db_path: PathBuf,
    // In-memory cache
    profiles_cache: dashmap::DashMap<String, TeamRuleProfile>,
}

impl TeamProfileManager {
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Create database if needed
        std::fs::create_dir_all(&db_path)?;

        Ok(Self {
            db_path,
            profiles_cache: dashmap::DashMap::new(),
        })
    }

    /// Create a new profile.
    pub async fn create_profile(
        &self,
        team_id: String,
        organization_id: String,
        name: String,
        inherit_from: Option<String>,
    ) -> Result<TeamRuleProfile> {
        let profile = TeamRuleProfile::new(team_id, organization_id, name, inherit_from);

        // Store in cache
        self.profiles_cache.insert(profile.profile_id.clone(), profile.clone());

        // Persist to database
        self.persist_profile(&profile).await?;

        tracing::info!("Created team profile: {}", profile.profile_id);
        Ok(profile)
    }

    /// Get a profile by ID.
    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<TeamRuleProfile>> {
        // Check cache first
        if let Some(profile) = self.profiles_cache.get(profile_id) {
            return Ok(Some(profile.clone()));
        }

        // Load from database
        if let Some(profile) = self.load_profile(profile_id).await? {
            self.profiles_cache.insert(profile_id.to_string(), profile.clone());
            return Ok(Some(profile));
        }

        Ok(None)
    }

    /// Get all profiles for a team.
    pub async fn get_team_profiles(&self, team_id: &str) -> Result<Vec<TeamRuleProfile>> {
        let profiles = self
            .profiles_cache
            .iter()
            .filter(|entry| entry.value().team_id == team_id)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(profiles)
    }

    /// Update a profile.
    pub async fn update_profile(&self, profile: TeamRuleProfile) -> Result<()> {
        self.profiles_cache
            .insert(profile.profile_id.clone(), profile.clone());

        // Persist to database
        self.persist_profile(&profile).await?;

        tracing::info!("Updated profile: {}", profile.profile_id);
        Ok(())
    }

    /// Delete a profile.
    pub async fn delete_profile(&self, profile_id: &str) -> Result<()> {
        self.profiles_cache.remove(profile_id);

        // Remove from database
        self.delete_from_db(profile_id).await?;

        tracing::info!("Deleted profile: {}", profile_id);
        Ok(())
    }

    /// Persist a profile to database
    async fn persist_profile(&self, profile: &TeamRuleProfile) -> Result<()> {
        let json = serde_json::to_string(profile)?;
        let file_path = self.db_path.join(format!("{}.json", profile.profile_id));
        tokio::fs::write(file_path, json).await?;
        tracing::debug!("Persisted profile: {}", profile.profile_id);
        Ok(())
    }

    /// Load a profile from database
    async fn load_profile(&self, profile_id: &str) -> Result<Option<TeamRuleProfile>> {
        let file_path = self.db_path.join(format!("{}.json", profile_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let json = tokio::fs::read_to_string(file_path).await?;
        let profile = serde_json::from_str(&json)?;
        Ok(Some(profile))
    }

    /// Delete a profile from database
    async fn delete_from_db(&self, profile_id: &str) -> Result<()> {
        let file_path = self.db_path.join(format!("{}.json", profile_id));

        if file_path.exists() {
            tokio::fs::remove_file(file_path).await?;
            tracing::debug!("Deleted profile from db: {}", profile_id);
        }

        Ok(())
    }

    /// Get number of profiles.
    pub async fn profile_count(&self) -> Result<usize> {
        Ok(self.profiles_cache.len())
    }

    /// List all profiles.
    pub async fn list_profiles(&self) -> Result<Vec<TeamRuleProfile>> {
        let profiles = self
            .profiles_cache
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        Ok(profiles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_profile_creation() {
        let profile = TeamRuleProfile::new(
            "team-1".to_string(),
            "org-1".to_string(),
            "Production Rules".to_string(),
            None,
        );

        assert_eq!(profile.team_id, "team-1");
        assert_eq!(profile.rules.len(), 0);
    }

    #[test]
    fn test_set_rule_config() {
        let mut profile = TeamRuleProfile::new(
            "team-1".to_string(),
            "org-1".to_string(),
            "Production Rules".to_string(),
            None,
        );

        let config = TeamRuleConfig {
            rule_id: "unused-import".to_string(),
            enabled: false,
            severity: "warning".to_string(),
            confidence_threshold: 0.8,
            metadata: HashMap::new(),
        };

        profile.set_rule_config(config);

        assert_eq!(profile.rules.len(), 1);
        assert!(profile.get_rule_config("unused-import").is_some());
    }

    #[tokio::test]
    async fn test_profile_manager() {
        let tmp_dir = std::env::temp_dir().join("test_profiles");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let manager = TeamProfileManager::new(tmp_dir).await.unwrap();

        let profile = manager
            .create_profile(
                "team-1".to_string(),
                "org-1".to_string(),
                "Test Profile".to_string(),
                None,
            )
            .await
            .unwrap();

        let retrieved = manager.get_profile(&profile.profile_id).await.unwrap();
        assert!(retrieved.is_some());
    }
}
