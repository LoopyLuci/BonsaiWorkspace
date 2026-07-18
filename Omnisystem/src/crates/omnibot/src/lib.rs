//! OmniBot - Telegram/Discord chatbot framework for the Bonsai ecosystem
//!
//! Platform-agnostic message handling ([`platform`]), capability-based
//! permissions ([`permission`], [`user`]), a command registry
//! ([`command`]), simple pattern-based intent classification ([`nlu`]),
//! per-user session state ([`session`]), an MCP tool-calling client
//! ([`mcp`]), and an event log ([`event`]).

pub mod command;
pub mod event;
pub mod mcp;
pub mod nlu;
pub mod permission;
pub mod platform;
pub mod session;
pub mod user;

pub use command::{
    Command, CommandContext, CommandRegistry, CommandResponse, HelpCommand, PingCommand,
    StatusCommand,
};
pub use event::Event;
pub use mcp::McpClient;
pub use nlu::{Intent, IntentClassifier};
pub use permission::{Capability, CapabilityToken, Permission};
pub use platform::{
    DiscordAdapter, Message, MessageId, Platform, PlatformAdapter, TelegramAdapter, UserId,
    UserInfo,
};
pub use session::{Session, SessionManager};
pub use user::{User, UserRole};
