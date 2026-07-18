//! Buddy: a simple in-memory conversational assistant orchestrator --
//! capability registry, per-conversation context/session storage, and a
//! pluggable interaction handler trait.

pub mod assistant;
pub mod capabilities;
pub mod context;
pub mod error;
pub mod interaction;

pub use assistant::{Buddy, Message};
pub use capabilities::CapabilityRegistry;
pub use context::ConversationContext;
pub use error::{BuddyError, Error, Result};
pub use interaction::{DefaultInteractionHandler, InteractionHandler};
