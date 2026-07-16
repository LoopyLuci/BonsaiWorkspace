//! bedf-fuzzing: a small AFL-style, coverage-guided mutation fuzzer.
//!
//! - [`mutation::Mutator`]: real byte-level mutation strategies (bit flip,
//!   byte flip, "interesting" boundary values, dictionary-token splicing,
//!   and havoc which chains several of the above).
//! - [`corpus::Corpus`]: a bounded seed corpus plus a crash-inputs list,
//!   with random-input generation and mutation-based sampling.
//! - [`fuzzer::CoverageGuidedFuzzer`]: tracks which "edges" (arbitrary u64
//!   identifiers a target instruments and reports back) have been seen,
//!   and reports coverage as a percentage of `FuzzerConfig::max_coverage`.
//! - [`config`]: [`config::FuzzerConfig`] tunables (iteration/timeout/
//!   input-size/corpus-size limits).
//! - [`interfaces::Component`]: a small async init/name trait for wiring a
//!   fuzz target into a larger host system.

pub mod config;
pub mod corpus;
pub mod fuzzer;
pub mod interfaces;
pub mod mutation;

pub use config::{Config, FuzzerConfig};
pub use corpus::Corpus;
pub use fuzzer::CoverageGuidedFuzzer;
pub use interfaces::Component;
pub use mutation::{MutationStrategy, Mutator};
