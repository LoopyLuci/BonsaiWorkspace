//! transfer-ai: an advisory congestion-control layer for data transfer.
//!
//! [`advisor::AiCongestionAdvisor`] is designed to eventually wrap a trained
//! model that suggests congestion-window / pacing-rate adjustments; today it
//! honestly reports "no advice" (`None`) rather than fabricating a
//! suggestion, since no model backend is wired up yet. Any advice it does
//! produce in the future must first pass through
//! [`safety::SafetyEnvelope`], which clamps AI output to
//! provably-safe bounds so a bad or malicious suggestion can never cause a
//! congestion collapse or buffer overflow.

pub mod advisor;
pub mod error;
pub mod safety;
pub mod types;

pub use advisor::{AiAdvice, AiCongestionAdvisor};
pub use error::{Error, Result};
pub use safety::SafetyEnvelope;
pub use types::State;
