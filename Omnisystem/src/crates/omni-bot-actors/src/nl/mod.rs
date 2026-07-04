//! Natural Language Processing Module
//!
//! Handles intent classification, entity extraction, and command templates
//! for converting natural language input to structured actions.

pub mod intent_classifier;
pub mod entity_extraction;
pub mod templates;

pub use intent_classifier::IntentClassifier;
pub use entity_extraction::EntityExtractor;
pub use templates::CommandTemplates;
