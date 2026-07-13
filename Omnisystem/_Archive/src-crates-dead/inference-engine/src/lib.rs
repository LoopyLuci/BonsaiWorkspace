mod error;
mod types;
mod engine;

pub use error::{InferenceError, InferenceResult};
pub use types::{Rule, Fact, InferenceOutcome, KnowledgeBase, DeductiveStep};
pub use engine::InferenceEngine;
