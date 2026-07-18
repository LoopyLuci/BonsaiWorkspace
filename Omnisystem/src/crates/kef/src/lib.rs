//! kef: Knowledge Extraction Fabric.
//!
//! Scans models, extracts candidate knowledge via synthetic generation,
//! activation clustering, attention-triplet mining, and membership
//! inference, then curates (dedup/PII/quality), embeds, indexes, and
//! packages the result into a KDB module.

pub mod activation_extractor;
pub mod attention_extractor;
pub mod curator;
pub mod error;
pub mod ingestion;
pub mod kef_service;
pub mod membership_inference;
pub mod model_scanner;
pub mod quality_scorer;
pub mod redaction;
pub mod synthetic_generator;
pub mod types;

pub use error::{KefError, Result};
pub use kef_service::KefService;
pub use types::*;
