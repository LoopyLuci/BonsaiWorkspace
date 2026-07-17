//! bedf-enhancements: the feature-flag catalog for the bedf (bug-hunter
//! evolution/development framework) fuzzing suite, listing the optional
//! enhancement modules (resource budgeting, flaky-test detection, supply
//! chain scanning, etc.) and whether each is enabled per [`EnhancementsConfig`].

mod config;
mod enhancements;

pub use config::{Config, EnhancementsConfig};
pub use enhancements::{Enhancement, EnhancementEngine};
