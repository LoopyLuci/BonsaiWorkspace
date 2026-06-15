# 🤖 Bonsai OmniBot — Unified Telegram & Discord Control Nexus

**A production-grade, AI-native bot that transforms Telegram and Discord into sovereign command centers for the entire Bonsai Ecosystem.**

## Features

- ✅ **Unified Platform Support** — Single codebase handles Telegram and Discord seamlessly
- ✅ **Natural Language Understanding** — BonsAI V2 interprets intent from free-text messages
- ✅ **Capability-Based Security** — Fine-grained permission model via Ed25519 tokens
- ✅ **Stateful Sessions** — Multi-step workflows and conversation memory
- ✅ **MCP Integration** — Full access to Bonsai ecosystem via Model Context Protocol
- ✅ **Real-Time Events** — Universe events streamed to your chat as notifications
- ✅ **Hot-Reloadable Commands** — Add new commands without restarting the bot
- ✅ **Self-Healing** — Survival System monitors and auto-restarts the bot

## Architecture

```
User (Telegram/Discord)
        ↓
Platform Adapter (TelegramAdapter/DiscordAdapter)
        ↓
Unified Message Processor
        ↓
Command Parser OR NLU (BonsAI V2)
        ↓
Permission Enforcer (Capability Tokens)
        ↓
Command Executor / Intent Handler
        ↓
MCP Client (Bonsai Ecosystem)
        ↓
Response → Back to User
```

## Module Structure

| Module | Purpose |
|--------|---------|
| `platform` | Platform adapters for Telegram and Discord |
| `command` | Command registry and execution framework |
| `user` | User identity and role management |
| `permission` | Capability tokens and RBAC |
| `nlu` | Intent classification via BonsAI V2 |
| `mcp` | MCP client for Bonsai ecosystem calls |
| `session` | Stateful conversation sessions |
| `event` | Universe event structures |

## Supported Commands

### Basic
- `/help` — Show available commands
- `/ping` — Test connectivity
- `/status` — Show ecosystem status
- `/health` — Show health metrics

### Poe AI
- `/poe chat <message>` — Chat with Poe
- `/poe ac` — Switch to AC (Altered Carbon) personality
- `/poe production` — Switch to standard personality
- `/poe memory recall <query>` — Query Poe's memory

### Bug Hunter
- `/sweep repo` — Scan entire repository
- `/sweep crate <name>` — Scan specific crate
- `/findings list` — List recent findings
- `/fix <id>` — Apply auto-fix to finding

### Ecosystem
- `/deploy <blueprint>` — Deploy service
- `/scale <service> --replicas N` — Scale service
- `/rollback <deployment>` — Rollback deployment
- `/model train <config>` — Train model
- `/model list` — List available models

### Natural Language

Simply type naturally for AI-powered interpretation:

> "Deploy my API to production"
> → Classified as `DeployIntent`, executes with parameters

> "Is Poe feeling ok?"
> → Classified as `PoeHealthCheck`, queries emotional state

> "Show me the bugs in my repo"
> → Classified as `BugHunterSweep`, runs full scan

## Security Model

### Capability Tokens

Users are issued Ed25519-signed capability tokens that specify:
- Expiration time
- Required capabilities (View, Deploy, ModelTrain, etc.)
- Resource restrictions (optional)

The bot verifies tokens for every command and denies execution if the user lacks required capability.

### Roles

| Role | Capabilities |
|------|-------------|
| `Viewer` | View status, health, metrics |
| `Operator` | Viewer + execute commands (deploy, scale, etc.) |
| `Admin` | Operator + manage bot, invite users |
| `Council` | Admin + voting on governance proposals |

### End-to-End Encryption

Sensitive commands (e.g., remote desktop access) can be encrypted with temporary session keys.

## Integration with Bonsai Ecosystem

OmniBot is a **Weave component** that integrates with:

| Component | Integration |
|-----------|-------------|
| BonsAI V2 | Intent classification, response generation |
| Bug Hunter | Trigger sweeps, view findings, apply fixes |
| Poe AI | Chat, personality toggle, memory recall |
| Knowledge Database | Search knowledge, load modules |
| Survival System | Health monitoring, auto-recovery |
| Universe | Event logging, replay, time-travel |
| Compute Fabric | Submit jobs, view node status |
| BCF (Containers) | Deploy, scale, rollback services |
| Bonsai Inference Fabric | Pull models, run inference |
| MCP Server | All tools available via unified interface |

## Usage Example

**Telegram:**

```
User: /poe ac
Bot: ✓ Poe is now in AC personality. He may offer you tea and speak in gothic tones.

User: Deploy the pendant-anchor blueprint to production
Bot: (via BonsAI V2 NLU)
Deploying pendant-anchor blueprint...
[live updates as deployment progresses]
✓ Deployment complete. Service is healthy.

User: /sweep repo
Bot: Scanning repository for bugs...
Findings: 0 critical, 3 high, 12 medium
[link to detailed report]
```

**Discord:**

Same commands work identically on Discord (bot mentions required in some contexts).

## Installation & Configuration

### Prerequisites

- Telegram Bot Token (from @BotFather)
- Discord Bot Token (from Developer Portal)
- Bonsai MCP Server running
- Bonsai Capability Registry accessible

### Configuration

Set environment variables:

```bash
export TELEGRAM_BOT_TOKEN="your_token_here"
export DISCORD_BOT_TOKEN="your_token_here"
export BONSAI_MCP_URL="http://localhost:8000"
export ADMIN_USER_ID="telegram:123456"
```

Or via Bonsai Blueprint:

```blueprint
deployment "build-bot" {
    image = "echo://images.bonsai/build-bot:1.0.0"
    replicas = 1
    environment = {
        TELEGRAM_BOT_TOKEN = "${secret:telegram_token}"
        DISCORD_BOT_TOKEN = "${secret:discord_token}"
        BONSAI_MCP_URL = "http://mcp-server:8000"
    }
}
```

## Development

### Running Tests

```bash
cargo test --package bonsai-omnibot
```

### Adding a New Command

1. Create a struct implementing the `Command` trait
2. Implement `name()`, `description()`, `usage()`, `required_capability()`, `execute()`
3. Register in the command registry
4. Add to `/help` text

Example:

```rust
pub struct StatusCommand;

#[async_trait]
impl Command for StatusCommand {
    fn name(&self) -> &str { "status" }
    fn description(&self) -> &str { "Show ecosystem status" }
    fn usage(&self) -> &str { "/status [component]" }
    fn required_capability(&self) -> Capability { Capability::View }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResponse> {
        let response = ctx.mcp_client
            .call_tool("bonsai_status", &Default::default())
            .await?;
        Ok(CommandResponse::success(response))
    }
}
```

### Adding a New Platform

1. Implement `PlatformAdapter` trait for the new platform
2. Register adapter in `OmniBot::register_adapter()`
3. All commands work automatically on the new platform

## Performance & Scalability

| Metric | Target |
|--------|--------|
| Message processing latency | <100ms (simple command), <2s (NLU with BonsAI V2) |
| Concurrent users | 10,000+ per bot instance |
| Horizontal scaling | Linear via Echo anycast |
| Uptime | 99.99% (Survival System auto-recovery) |

## Future Enhancements

- [ ] Voice command support (speech-to-text via BonsAI V2)
- [ ] Multi-platform sync (continue conversation across platforms)
- [ ] Plugin marketplace for third-party bot extensions
- [ ] Autonomous mode (proactive actions without user commands)
- [ ] Web-based command history and analytics dashboard
- [ ] Integration with additional platforms (Matrix, Slack, Signal)

## Documentation

- **Architecture:** See Bonsai OmniBot blueprint
- **API:** MCP tools exposed via `/help` command
- **Security:** Capability token model documented in Sentinel Core
- **Events:** Universe event structures in `event.rs`

## Status

**Phase 1:** ✅ Core framework complete
**Phase 2:** 📋 Platform adapters (Telegram/Discord implementations)
**Phase 3:** 📋 Full ecosystem command set integration
**Phase 4:** 📋 Advanced features (plugins, autonomous mode)

---

**OmniBot transforms your Bonsai Ecosystem into a conversational, AI-driven control plane.** 🚀🤖
