//! Remote-access-support: an in-memory registry backing a remote-support
//! session (think screen-sharing / remote-assistance tooling) -- sessions,
//! per-user security policies, multiplexed channels, and an executed-
//! command log.

pub mod channel;
pub mod command;
pub mod error;
pub mod security;
pub mod session;

pub use channel::{Channel, ChannelManager, ChannelType};
pub use command::{Command, CommandExecutor};
pub use error::{Error, Result};
pub use security::{SecurityManager, SecurityPolicy};
pub use session::{RemoteSession, SessionManager};
