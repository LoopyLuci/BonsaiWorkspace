//! Agent-to-agent secure mailbox for Bonsai swarm communication.
//!
//! Each agent registers with its `BonsaiIdentity`. Messages are encrypted
//! to the recipient's public key using a fresh session handshake, then
//! delivered over the best available transport (relay or direct).
//!
//! Design:
//! - Local delivery: zero-copy `mpsc` channel.
//! - Remote delivery: serialized + encrypted over a `RelayClient`.
//! - Inbox: bounded buffer — callers `recv()` asynchronously.

pub mod error;
pub mod mailbox;
pub mod envelope;

pub use error::{MailboxError, MailboxResult};
pub use mailbox::AgentMailbox;
pub use envelope::{MailEnvelope, AgentId};
