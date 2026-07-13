//! FreeLLMAPI Core - shared domain types, traits, and service registry used by every
//! crate in the FreeLLMAPI gateway system (storage, auth, router, billing, ratelimit,
//! dashboard, events, and provider adapters).

pub mod errors;
pub mod models;
pub mod services;

pub use errors::{FreeLLMAPIError, Result};
pub use models::*;
pub use services::*;
