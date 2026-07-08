// Command system for OmniBot

use crate::{Message, User, Capability, Session, McpClient};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;

/// Command response
#[derive(Debug, Clone)]
pub struct CommandResponse {
    pub text: String,
    pub is_error: bool,
}

impl CommandResponse {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}

/// Command context
pub struct CommandContext {
    pub user: User,
    pub message: Message,
    pub mcp_client: Arc<McpClient>,
    pub session: Arc<Session>,
}

/// Command trait
#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;
    fn required_capability(&self) -> Capability;
    async fn execute(&self, ctx: CommandContext) -> Result<CommandResponse>;
}

/// Command registry
pub struct CommandRegistry {
    commands: DashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: DashMap::new(),
        }
    }

    pub fn register(&self, command: Arc<dyn Command>) {
        self.commands.insert(command.name().into(), command);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.get(name).map(|r| r.value().clone())
    }

    pub fn list(&self) -> Vec<Arc<dyn Command>> {
        self.commands.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Help command
pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "Show available commands"
    }

    fn usage(&self) -> &str {
        "/help [command]"
    }

    fn required_capability(&self) -> Capability {
        Capability::View
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResponse> {
        let text = r#"🤖 **Bonsai OmniBot Commands**

**Basic:**
- `/help` — Show this message
- `/ping` — Test connectivity

**Ecosystem:**
- `/status` — Show Bonsai ecosystem status
- `/health` — Show health metrics
- `/metrics` — Show detailed metrics

**Poe AI:**
- `/poe chat <msg>` — Chat with Poe
- `/poe ac` — Switch to AC personality
- `/poe production` — Switch to standard mode

**Bug Hunter:**
- `/sweep repo` — Scan repository for bugs
- `/sweep crate <name>` — Scan specific crate

**Other:**
- Type naturally for AI-powered intent classification!

Use `/help <command>` for detailed help."#;
        Ok(CommandResponse::success(text))
    }
}

/// Ping command
pub struct PingCommand;

#[async_trait]
impl Command for PingCommand {
    fn name(&self) -> &str {
        "ping"
    }

    fn description(&self) -> &str {
        "Test bot connectivity"
    }

    fn usage(&self) -> &str {
        "/ping"
    }

    fn required_capability(&self) -> Capability {
        Capability::View
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResponse> {
        Ok(CommandResponse::success("🏓 Pong!"))
    }
}

/// Status command
pub struct StatusCommand;

#[async_trait]
impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }

    fn description(&self) -> &str {
        "Show Bonsai ecosystem status"
    }

    fn usage(&self) -> &str {
        "/status [component]"
    }

    fn required_capability(&self) -> Capability {
        Capability::View
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResponse> {
        // Call MCP tool to get status
        match ctx.mcp_client.call_tool("bonsai_status", &Default::default()).await {
            Ok(response) => Ok(CommandResponse::success(response)),
            Err(e) => Ok(CommandResponse::error(format!("Error: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_response() {
        let success = CommandResponse::success("OK");
        assert!(!success.is_error);
        assert_eq!(success.text, "OK");

        let error = CommandResponse::error("Failed");
        assert!(error.is_error);
        assert_eq!(error.text, "Failed");
    }

    #[test]
    fn test_command_registry() {
        let registry = CommandRegistry::new();
        let help = Arc::new(HelpCommand);
        registry.register(help.clone());

        assert!(registry.get("help").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_help_command() {
        let cmd = HelpCommand;
        assert_eq!(cmd.name(), "help");
        assert_eq!(cmd.required_capability(), Capability::View);
    }
}
