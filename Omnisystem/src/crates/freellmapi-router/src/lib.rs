//! FreeLLMAPI Router - Thompson-sampling based provider selection (a Beta
//! distribution bandit) plus fastest/cheapest/most-reliable routing strategies.

pub mod bandit;
pub mod service;

pub use bandit::{sample_beta, BetaDistribution};
pub use service::{ProviderStats, RouterService};
