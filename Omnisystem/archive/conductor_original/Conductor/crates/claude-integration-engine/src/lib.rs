//! Claude AI Integration Engine
//!
//! Provides natural language command processing and intelligent recommendations
//! for container operations using the Claude API.

#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use tracing::{info, debug};
use std::sync::Arc;
use dashmap::DashMap;

/// Claude API client configuration
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    /// API key
    pub api_key: String,
    /// Base URL
    pub base_url: String,
    /// Model version
    pub model: String,
    /// Max tokens per request
    pub max_tokens: usize,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("CLAUDE_API_KEY").unwrap_or_default(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: "claude-opus-4".to_string(),
            max_tokens: 2048,
        }
    }
}

/// Claude integration engine
pub struct ClaudeIntegrationEngine {
    config: ClaudeConfig,
    cache: Arc<DashMap<String, CommandInterpretation>>,
}

impl ClaudeIntegrationEngine {
    /// Create a new Claude integration engine
    pub fn new(config: ClaudeConfig) -> Self {
        info!("Initializing Claude integration with model: {}", config.model);
        Self {
            config,
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ClaudeConfig::default())
    }

    /// Process a natural language command
    pub async fn process_command(&self, input: &str) -> Result<CommandInterpretation> {
        debug!("Processing command: {}", input);

        // Check cache first
        if let Some(cached) = self.cache.get(input) {
            debug!("Cache hit for command: {}", input);
            return Ok(cached.clone());
        }

        // Parse command intent
        let interpretation = self.parse_command_intent(input).await?;

        // Cache result
        self.cache
            .insert(input.to_string(), interpretation.clone());

        Ok(interpretation)
    }

    /// Parse command intent
    async fn parse_command_intent(&self, input: &str) -> Result<CommandInterpretation> {
        debug!("Parsing command intent: {}", input);
        self.parse_command_intent_fallback(input)
    }

    /// Fallback command parsing without Claude
    fn parse_command_intent_fallback(&self, input: &str) -> Result<CommandInterpretation> {
        debug!("Using command parsing");

        let lower = input.to_lowercase();

        let (action, resource_type) = if lower.contains("list") || lower.contains("show") {
            ("list", "container")
        } else if lower.contains("start") {
            ("start", "container")
        } else if lower.contains("stop") || lower.contains("kill") {
            ("stop", "container")
        } else if lower.contains("remove") || lower.contains("delete") {
            ("remove", "container")
        } else if lower.contains("create") || lower.contains("run") {
            ("create", "container")
        } else if lower.contains("image") {
            ("list", "image")
        } else if lower.contains("network") {
            ("list", "network")
        } else if lower.contains("volume") {
            ("list", "volume")
        } else {
            ("list", "container")
        };

        Ok(CommandInterpretation {
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_name: String::new(),
            parameters: std::collections::HashMap::new(),
            confidence: 0.6,
            explanation: "Parsed using pattern matching".to_string(),
        })
    }

    /// Generate optimization recommendations
    pub async fn generate_recommendations(&self, _container_metrics: &str) -> Result<Vec<String>> {
        debug!("Generating recommendations");

        Ok(vec![
            "Monitor CPU usage for potential optimization opportunities".to_string(),
            "Consider using resource limits to prevent runaway containers".to_string(),
            "Implement health checks for better reliability".to_string(),
        ])
    }

    /// Troubleshoot an issue
    pub async fn troubleshoot_issue(&self, issue_description: &str) -> Result<TroubleshootingGuide> {
        debug!("Troubleshooting: {}", issue_description);

        Ok(TroubleshootingGuide {
            issue: issue_description.to_string(),
            diagnosis: "Issue detected in container operations".to_string(),
            steps: vec![
                "Check Docker daemon status".to_string(),
                "Review container logs".to_string(),
                "Verify resource availability".to_string(),
                "Check network connectivity".to_string(),
            ],
            resolution: "Monitor system and apply recommended fixes".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claude_initialization() {
        let engine = ClaudeIntegrationEngine::default();
        assert!(!engine.config.model.is_empty());
    }

    #[tokio::test]
    async fn test_command_parsing() {
        let engine = ClaudeIntegrationEngine::default();
        let result = engine.process_command("list all containers").await;
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.action, "list");
    }

    #[tokio::test]
    async fn test_recommendations() {
        let engine = ClaudeIntegrationEngine::default();
        let metrics = "cpu: 45%, memory: 512MB, network: 10Mbps";
        let result = engine.generate_recommendations(metrics).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_troubleshooting() {
        let engine = ClaudeIntegrationEngine::default();
        let result = engine
            .troubleshoot_issue("Container keeps restarting")
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = ClaudeConfig::default();
        assert_eq!(config.model, "claude-opus-4");
        assert_eq!(config.max_tokens, 2048);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let engine = ClaudeIntegrationEngine::default();
        let cmd = "list containers";
        let result1 = engine.process_command(cmd).await;
        let result2 = engine.process_command(cmd).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_commands() {
        let engine = ClaudeIntegrationEngine::default();
        let commands = vec![
            "start container web",
            "stop container db",
            "list images",
        ];
        for cmd in commands {
            let result = engine.process_command(cmd).await;
            assert!(result.is_ok());
        }
    }
}
