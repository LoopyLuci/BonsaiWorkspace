//! CommandParser Actor - Converts natural language and structured input to Actions
//!
//! Responsibilities:
//! - Parse natural language commands (with optional AI assistance)
//! - Parse structured command syntax
//! - Extract parameters and intent
//! - Fall back to deterministic parsing if AI unavailable
//! - Route to ActionExecutor
//! - Track parsing success/failure metrics

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use omni_bot_core::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed command intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    ServiceStart,
    ServiceStop,
    ServiceRestart,
    ServiceStatus,
    EnvironmentCreate,
    EnvironmentStart,
    EnvironmentStop,
    EnvironmentSnapshot,
    EnvironmentRestore,
    ModuleInstall,
    ModuleUpdate,
    AssetGenerate,
    AssetPublish,
    ValidationRun,
    ToggleAI,
    Unknown,
}

impl Intent {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "start" | "launch" | "begin" => Intent::ServiceStart,
            "stop" | "terminate" | "end" | "kill" => Intent::ServiceStop,
            "restart" | "reload" => Intent::ServiceRestart,
            "status" | "info" | "check" => Intent::ServiceStatus,
            "create" | "new" | "make" => Intent::EnvironmentCreate,
            "snapshot" | "snap" | "save" => Intent::EnvironmentSnapshot,
            "restore" | "recover" | "revert" => Intent::EnvironmentRestore,
            "install" | "add" => Intent::ModuleInstall,
            "update" | "upgrade" => Intent::ModuleUpdate,
            "generate" | "create_asset" => Intent::AssetGenerate,
            "publish" | "deploy" => Intent::AssetPublish,
            "validate" | "test" | "check_validation" => Intent::ValidationRun,
            "toggle" | "enable" | "disable" => Intent::ToggleAI,
            _ => Intent::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Intent::ServiceStart => "service_start",
            Intent::ServiceStop => "service_stop",
            Intent::ServiceRestart => "service_restart",
            Intent::ServiceStatus => "service_status",
            Intent::EnvironmentCreate => "environment_create",
            Intent::EnvironmentStart => "environment_start",
            Intent::EnvironmentStop => "environment_stop",
            Intent::EnvironmentSnapshot => "environment_snapshot",
            Intent::EnvironmentRestore => "environment_restore",
            Intent::ModuleInstall => "module_install",
            Intent::ModuleUpdate => "module_update",
            Intent::AssetGenerate => "asset_generate",
            Intent::AssetPublish => "asset_publish",
            Intent::ValidationRun => "validation_run",
            Intent::ToggleAI => "toggle_ai",
            Intent::Unknown => "unknown",
        }
    }
}

/// Parsed command representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub intent: Intent,
    pub parameters: HashMap<String, String>,
    pub raw_input: String,
    pub confidence: f32,
    pub parsed_at: chrono::DateTime<chrono::Utc>,
}

impl ParsedCommand {
    pub fn new(intent: Intent, raw_input: String) -> Self {
        Self {
            intent,
            parameters: HashMap::new(),
            raw_input,
            confidence: 1.0,
            parsed_at: chrono::Utc::now(),
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

/// Messages for CommandParser
#[derive(Debug, Clone)]
pub enum CommandParserMessage {
    /// Parse a raw input string
    Parse { input: String },
    /// Parse with AI assistance (if available)
    ParseWithAI { input: String },
    /// Get parsing metrics
    GetMetrics,
    /// Clear parsed command cache
    ClearCache,
    /// Stop the actor
    Stop,
}

/// Parsing metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsingMetrics {
    pub total_parsed: u64,
    pub successful_parses: u64,
    pub failed_parses: u64,
    pub avg_confidence: f32,
    pub cache_size: usize,
}

/// CommandParser actor
pub struct CommandParser {
    id: ActorId,
    metrics: ParsingMetrics,
    cache: HashMap<String, ParsedCommand>,
    max_cache_size: usize,
    ai_enabled: bool,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            metrics: ParsingMetrics::default(),
            cache: HashMap::new(),
            max_cache_size: 1000,
            ai_enabled: false,
        }
    }

    pub fn with_ai_enabled(mut self, enabled: bool) -> Self {
        self.ai_enabled = enabled;
        self
    }

    /// Deterministic parsing using keyword matching and pattern analysis
    fn parse_deterministic(&self, input: &str) -> ParsedCommand {
        let lower = input.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        // Simple pattern matching for intents
        let intent = if words.is_empty() {
            Intent::Unknown
        } else {
            // Look for action verbs
            let action_word = words[0];
            Intent::from_str(action_word)
        };

        let mut cmd = ParsedCommand::new(intent, input.to_string());

        // Extract parameters using simple heuristics
        for i in 0..words.len() {
            match words[i] {
                "--name" | "-n" if i + 1 < words.len() => {
                    cmd = cmd.with_parameter("name".to_string(), words[i + 1].to_string());
                }
                "--type" | "-t" if i + 1 < words.len() => {
                    cmd = cmd.with_parameter("type".to_string(), words[i + 1].to_string());
                }
                "--version" | "-v" if i + 1 < words.len() => {
                    cmd = cmd.with_parameter("version".to_string(), words[i + 1].to_string());
                }
                "--force" | "-f" => {
                    cmd = cmd.with_parameter("force".to_string(), "true".to_string());
                }
                _ => {}
            }
        }

        // Extract positional arguments (words after the verb)
        if words.len() > 1 && intent != Intent::Unknown {
            if !words[1].starts_with('-') {
                cmd = cmd.with_parameter("target".to_string(), words[1].to_string());
            }
        }

        cmd.with_confidence(0.8) // Deterministic parsing confidence
    }

    /// Parse with AI assistance (optional)
    fn parse_with_ai(&self, input: &str) -> ParsedCommand {
        // Placeholder for AI-powered parsing
        // In production, this would call an LLM API
        log::info!("[CommandParser] AI parsing requested for: {}", input);

        // Fall back to deterministic if AI unavailable
        let mut cmd = self.parse_deterministic(input);
        cmd.confidence = 0.95; // Higher confidence with AI
        cmd
    }

    #[allow(dead_code)]
    fn convert_to_action(&self, cmd: &ParsedCommand) -> Option<Action> {
        match cmd.intent {
            Intent::ServiceStart => {
                let name = cmd.parameters.get("target")?.clone();
                Some(Action::StartService {
                    name,
                    config: None,
                })
            }
            Intent::ServiceStop => {
                let name = cmd.parameters.get("target")?.clone();
                let force = cmd
                    .parameters
                    .get("force")
                    .map(|v| v == "true")
                    .unwrap_or(false);
                Some(Action::StopService { name, force })
            }
            Intent::ServiceRestart => {
                let name = cmd.parameters.get("target")?.clone();
                Some(Action::RestartService { name })
            }
            Intent::ServiceStatus => {
                let name = cmd.parameters.get("target")?.clone();
                Some(Action::GetServiceStatus { name })
            }
            Intent::ModuleInstall => {
                let name = cmd.parameters.get("target")?.clone();
                let version = cmd
                    .parameters
                    .get("version")
                    .cloned()
                    .unwrap_or_else(|| "latest".to_string());
                Some(Action::InstallModule { name, version })
            }
            Intent::ModuleUpdate => {
                let name = cmd.parameters.get("target")?.clone();
                let version = cmd
                    .parameters
                    .get("version")
                    .cloned()
                    .unwrap_or_else(|| "latest".to_string());
                Some(Action::UpdateModule { name, version })
            }
            Intent::AssetGenerate => {
                let asset_type = cmd.parameters.get("type")?.clone();
                let description = cmd
                    .parameters
                    .get("description")
                    .cloned()
                    .unwrap_or_default();
                Some(Action::GenerateAsset {
                    asset_type,
                    description,
                })
            }
            Intent::ValidationRun => {
                let suite = cmd.parameters.get("target")?.clone();
                Some(Action::RunValidation {
                    suite,
                    matrix: std::collections::HashMap::new(),
                })
            }
            _ => None,
        }
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Actor for CommandParser {
    type Message = CommandParserMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            CommandParserMessage::Parse { input } => {
                // Check cache first
                if self.cache.contains_key(&input) {
                    log::info!("[CommandParser] Cache hit for: {}", input);
                    return Ok(true);
                }

                let cmd = self.parse_deterministic(&input);
                let success = cmd.intent != Intent::Unknown;

                if success {
                    self.metrics.successful_parses += 1;
                    log::info!(
                        "[CommandParser] Parsed '{}' -> {:?}",
                        input,
                        cmd.intent
                    );

                    // Add to cache
                    if self.cache.len() >= self.max_cache_size {
                        self.cache.clear();
                    }
                    self.cache.insert(input, cmd);
                } else {
                    self.metrics.failed_parses += 1;
                    log::warn!("[CommandParser] Failed to parse: {}", input);
                }

                self.metrics.total_parsed += 1;
                self.metrics.cache_size = self.cache.len();
                Ok(true)
            }
            CommandParserMessage::ParseWithAI { input } => {
                if self.ai_enabled {
                    let _cmd = self.parse_with_ai(&input);
                    self.metrics.successful_parses += 1;
                } else {
                    log::warn!("[CommandParser] AI parsing requested but disabled");
                    let _cmd = self.parse_deterministic(&input);
                }
                self.metrics.total_parsed += 1;
                Ok(true)
            }
            CommandParserMessage::GetMetrics => {
                log::info!("[CommandParser] Metrics: {:?}", self.metrics);
                Ok(true)
            }
            CommandParserMessage::ClearCache => {
                let count = self.cache.len();
                self.cache.clear();
                log::info!("[CommandParser] Cleared {} cache entries", count);
                self.metrics.cache_size = 0;
                Ok(true)
            }
            CommandParserMessage::Stop => {
                log::info!("[CommandParser] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let state = serde_json::json!({
            "metrics": self.metrics,
            "cache_size": self.cache.len(),
            "ai_enabled": self.ai_enabled,
        });

        Ok(Snapshot::new(
            self.id,
            "CommandParser".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        log::info!("[CommandParser] Restored from snapshot");
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "CommandParser"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_from_str() {
        assert_eq!(Intent::from_str("start"), Intent::ServiceStart);
        assert_eq!(Intent::from_str("stop"), Intent::ServiceStop);
        assert_eq!(Intent::from_str("restart"), Intent::ServiceRestart);
        assert_eq!(Intent::from_str("unknown"), Intent::Unknown);
    }

    #[test]
    fn test_deterministic_parsing() {
        let parser = CommandParser::new();

        let cmd = parser.parse_deterministic("start my-service --force");
        assert_eq!(cmd.intent, Intent::ServiceStart);
        assert_eq!(
            cmd.parameters.get("target").map(|s| s.as_str()),
            Some("my-service")
        );
        assert_eq!(
            cmd.parameters.get("force").map(|s| s.as_str()),
            Some("true")
        );
    }

    #[test]
    fn test_parameter_extraction() {
        let parser = CommandParser::new();

        let cmd = parser.parse_deterministic("install mymodule --version 2.0.1");
        assert_eq!(cmd.intent, Intent::ModuleInstall);
        assert_eq!(
            cmd.parameters.get("version").map(|s| s.as_str()),
            Some("2.0.1")
        );
    }

    #[test]
    fn test_convert_to_action() {
        let parser = CommandParser::new();
        let cmd = parser.parse_deterministic("start nginx");

        let action = parser.convert_to_action(&cmd);
        match action {
            Some(Action::StartService { name, .. }) => {
                assert_eq!(name, "nginx");
            }
            _ => panic!("Expected StartService action"),
        }
    }
}
