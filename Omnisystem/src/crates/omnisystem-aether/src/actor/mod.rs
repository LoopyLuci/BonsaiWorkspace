//! Aether Actor Runtime
//!
//! Actors with supervision trees, database-aware persistence, and location transparency.

pub mod runtime;
pub mod mailbox;

pub use runtime::{Actor, ActorRef, ActorRuntime};
pub use mailbox::Mailbox;
