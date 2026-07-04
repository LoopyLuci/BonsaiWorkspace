//! Deterministic Keyword Parser
//!
//! Parses structured command syntax without any AI/HDE dependency.
//! Always available, highly predictable, suitable for automation.
//!
//! Syntax:
//! - `<command> <target> [--flag value] [--name value]`
//! - Example: `deploy vm --cpus 2 --memory 4096`

use omni_bot_core::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of keyword parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordParseResult {
    /// The command that was parsed
    pub command: String,
    /// The target of the command
    pub target: Option<String>,
    /// Extracted parameters
    pub params: HashMap<String, serde_json::Value>,
    /// The action to execute
    pub action: Action,
    /// Diagnostic notes
    pub notes: Vec<String>,
}

/// Deterministic keyword parser
pub struct KeywordParser {
    /// Known commands with their syntax
    commands: HashMap<String, CommandSpec>,
}

/// Specification for a command
#[derive(Debug, Clone)]
struct CommandSpec {
    /// Command name
    name: String,
    /// Whether this command requires a target
    requires_target: bool,
    /// Valid flags for this command
    valid_flags: Vec<String>,
}

impl KeywordParser {
    /// Create a new keyword parser with all known commands
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        // Service commands
        commands.insert("start".to_string(), CommandSpec {
            name: "start".to_string(),
            requires_target: true,
            valid_flags: vec!["--config".to_string(), "--force".to_string(), "-f".to_string()],
        });
        commands.insert("stop".to_string(), CommandSpec {
            name: "stop".to_string(),
            requires_target: true,
            valid_flags: vec!["--force".to_string(), "-f".to_string()],
        });
        commands.insert("restart".to_string(), CommandSpec {
            name: "restart".to_string(),
            requires_target: true,
            valid_flags: vec![],
        });
        commands.insert("status".to_string(), CommandSpec {
            name: "status".to_string(),
            requires_target: true,
            valid_flags: vec![],
        });
        commands.insert("configure".to_string(), CommandSpec {
            name: "configure".to_string(),
            requires_target: true,
            valid_flags: vec!["--key".to_string(), "--value".to_string()],
        });

        // Environment commands
        commands.insert("create".to_string(), CommandSpec {
            name: "create".to_string(),
            requires_target: true,
            valid_flags: vec!["--cpus".to_string(), "--memory".to_string(), "--name".to_string(), "--type".to_string()],
        });
        commands.insert("deploy".to_string(), CommandSpec {
            name: "deploy".to_string(),
            requires_target: true,
            valid_flags: vec!["--cpus".to_string(), "--memory".to_string(), "--config".to_string()],
        });
        commands.insert("snapshot".to_string(), CommandSpec {
            name: "snapshot".to_string(),
            requires_target: true,
            valid_flags: vec!["--name".to_string()],
        });
        commands.insert("restore".to_string(), CommandSpec {
            name: "restore".to_string(),
            requires_target: true,
            valid_flags: vec!["--from".to_string(), "--snapshot".to_string()],
        });
        commands.insert("migrate".to_string(), CommandSpec {
            name: "migrate".to_string(),
            requires_target: true,
            valid_flags: vec!["--to".to_string(), "--target".to_string()],
        });
        commands.insert("delete".to_string(), CommandSpec {
            name: "delete".to_string(),
            requires_target: true,
            valid_flags: vec!["--force".to_string()],
        });

        // Module commands
        commands.insert("install".to_string(), CommandSpec {
            name: "install".to_string(),
            requires_target: true,
            valid_flags: vec!["--version".to_string(), "-v".to_string()],
        });
        commands.insert("update".to_string(), CommandSpec {
            name: "update".to_string(),
            requires_target: true,
            valid_flags: vec!["--version".to_string(), "-v".to_string()],
        });
        commands.insert("remove".to_string(), CommandSpec {
            name: "remove".to_string(),
            requires_target: true,
            valid_flags: vec![],
        });

        // Asset commands
        commands.insert("generate".to_string(), CommandSpec {
            name: "generate".to_string(),
            requires_target: true,
            valid_flags: vec!["--type".to_string(), "--description".to_string(), "-d".to_string()],
        });
        commands.insert("publish".to_string(), CommandSpec {
            name: "publish".to_string(),
            requires_target: true,
            valid_flags: vec![],
        });

        // Validation commands
        commands.insert("validate".to_string(), CommandSpec {
            name: "validate".to_string(),
            requires_target: false,
            valid_flags: vec!["--suite".to_string(), "--matrix".to_string()],
        });
        commands.insert("test".to_string(), CommandSpec {
            name: "test".to_string(),
            requires_target: false,
            valid_flags: vec!["--suite".to_string()],
        });

        Self { commands }
    }

    /// Parse a command string
    pub fn parse(&self, input: &str) -> Result<KeywordParseResult, crate::ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(crate::ParseError::EmptyInput);
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(crate::ParseError::EmptyInput);
        }

        let command_name = parts[0];
        let spec = self
            .commands
            .get(command_name)
            .ok_or(crate::ParseError::UnrecognizedCommand {
                input: input.to_string(),
            })?;

        let mut params: HashMap<String, serde_json::Value> = HashMap::new();
        let mut target: Option<String> = None;
        let mut notes = Vec::new();

        // Extract target and flags
        let mut i = 1;
        let mut context = "target"; // "target" or "flag_value"

        while i < parts.len() {
            let part = parts[i];

            if part.starts_with("--") || part.starts_with("-") {
                // Flag encountered
                let flag_name = part.to_string();
                let flag_short = if part.starts_with("--") {
                    part[2..].to_string()
                } else {
                    part[1..].to_string()
                };

                if spec.valid_flags.contains(&flag_name) || spec.valid_flags.iter().any(|f| f.ends_with(&flag_short)) {
                    // Look for value
                    if i + 1 < parts.len() && !parts[i + 1].starts_with("-") {
                        let value = parts[i + 1];
                        // Try to parse as number
                        if let Ok(n) = value.parse::<i64>() {
                            params.insert(flag_short, serde_json::json!(n));
                        } else {
                            params.insert(flag_short, serde_json::json!(value));
                        }
                        i += 2;
                    } else {
                        // Flag without value (boolean flag)
                        params.insert(flag_short, serde_json::json!(true));
                        i += 1;
                    }
                } else {
                    notes.push(format!("Unknown flag: {}", flag_name));
                    i += 1;
                }
                context = "flag";
            } else {
                // Regular argument
                if context == "target" && target.is_none() {
                    target = Some(part.to_string());
                } else {
                    notes.push(format!("Ignored positional argument: {}", part));
                }
                i += 1;
            }
        }

        // Validate required target
        if spec.requires_target && target.is_none() {
            return Err(crate::ParseError::SyntaxError {
                message: format!("Command '{}' requires a target", command_name),
            });
        }

        // Store target in params if provided
        if let Some(ref t) = target {
            params.insert("target".to_string(), serde_json::json!(t));
        }

        // Build action based on command
        let action = self.build_action(command_name, target.as_deref(), &params)?;

        Ok(KeywordParseResult {
            command: command_name.to_string(),
            target,
            params,
            action,
            notes,
        })
    }

    /// Build an action from parsed command
    fn build_action(
        &self,
        command: &str,
        target: Option<&str>,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<Action, crate::ParseError> {
        let target = target.unwrap_or("unknown");

        match command {
            "start" => {
                Ok(Action::StartService {
                    name: target.to_string(),
                    config: None,
                })
            }
            "stop" => {
                let force = params
                    .get("force")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(Action::StopService {
                    name: target.to_string(),
                    force,
                })
            }
            "restart" => {
                Ok(Action::RestartService {
                    name: target.to_string(),
                })
            }
            "status" => {
                Ok(Action::GetServiceStatus {
                    name: target.to_string(),
                })
            }
            "configure" => {
                Ok(Action::ConfigureService {
                    name: target.to_string(),
                    updates: params.clone(),
                })
            }
            "create" => {
                let mut spec: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                if let Some(cpus) = params.get("cpus") {
                    spec.insert("cpus".to_string(), cpus.clone());
                }
                if let Some(mem) = params.get("memory") {
                    spec.insert("memory".to_string(), mem.clone());
                }
                Ok(Action::CreateEnvironment {
                    name: target.to_string(),
                    spec,
                })
            }
            "deploy" => {
                let mut spec: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                if let Some(cpus) = params.get("cpus") {
                    spec.insert("cpus".to_string(), cpus.clone());
                }
                if let Some(mem) = params.get("memory") {
                    spec.insert("memory".to_string(), mem.clone());
                }
                Ok(Action::CreateEnvironment {
                    name: target.to_string(),
                    spec,
                })
            }
            "snapshot" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("snapshot")
                    .to_string();
                Ok(Action::SnapshotEnvironment {
                    id: target.to_string(),
                    name,
                })
            }
            "restore" => {
                let snapshot_id = params
                    .get("from")
                    .or_else(|| params.get("snapshot"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                Ok(Action::RestoreEnvironment {
                    id: target.to_string(),
                    snapshot_id,
                })
            }
            "migrate" => {
                let target_dest = params
                    .get("to")
                    .or_else(|| params.get("target"))
                    .and_then(|v| v.as_str())
                    .ok_or(crate::ParseError::MissingParameter(
                        "target".to_string(),
                    ))?
                    .to_string();
                Ok(Action::MigrateEnvironment {
                    id: target.to_string(),
                    target: target_dest,
                })
            }
            "delete" => {
                Ok(Action::DeleteEnvironment {
                    id: target.to_string(),
                })
            }
            "install" => {
                let version = params
                    .get("version")
                    .or_else(|| params.get("v"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("latest")
                    .to_string();
                Ok(Action::InstallModule {
                    name: target.to_string(),
                    version,
                })
            }
            "update" => {
                let version = params
                    .get("version")
                    .or_else(|| params.get("v"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("latest")
                    .to_string();
                Ok(Action::UpdateModule {
                    name: target.to_string(),
                    version,
                })
            }
            "remove" => {
                Ok(Action::RemoveModule {
                    name: target.to_string(),
                })
            }
            "generate" => {
                let asset_type = params
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or(target)
                    .to_string();
                let description = params
                    .get("description")
                    .or_else(|| params.get("d"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto-generated")
                    .to_string();
                Ok(Action::GenerateAsset {
                    asset_type,
                    description,
                })
            }
            "publish" => {
                Ok(Action::PublishAsset {
                    id: target.to_string(),
                })
            }
            "validate" | "test" => {
                let suite = params
                    .get("suite")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                Ok(Action::RunValidation {
                    suite,
                    matrix: std::collections::HashMap::new(),
                })
            }
            _ => {
                Ok(Action::Custom {
                    name: command.to_string(),
                    params: params.clone(),
                })
            }
        }
    }
}

impl Default for KeywordParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_start_service() {
        let parser = KeywordParser::new();
        let result = parser.parse("start nginx").unwrap();
        assert_eq!(result.command, "start");
        assert_eq!(result.target, Some("nginx".to_string()));
    }

    #[test]
    fn test_parse_with_flags() {
        let parser = KeywordParser::new();
        let result = parser.parse("create env-01 --cpus 4 --memory 8192").unwrap();
        assert_eq!(result.command, "create");
        assert_eq!(result.target, Some("env-01".to_string()));
        assert_eq!(result.params.get("cpus").and_then(|v| v.as_i64()), Some(4));
        assert_eq!(
            result.params.get("memory").and_then(|v| v.as_i64()),
            Some(8192)
        );
    }

    #[test]
    fn test_unknown_command() {
        let parser = KeywordParser::new();
        let result = parser.parse("foobar nginx");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_target() {
        let parser = KeywordParser::new();
        let result = parser.parse("start");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_input() {
        let parser = KeywordParser::new();
        let result = parser.parse("");
        assert!(result.is_err());
    }
}
