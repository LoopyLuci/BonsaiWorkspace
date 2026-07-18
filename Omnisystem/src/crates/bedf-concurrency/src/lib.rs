//! bedf-concurrency: concurrency-testing primitives for the BEDF harness.
//!
//! Provides a happens-before-free [`race_detector::RaceDetector`] that flags
//! conflicting concurrent memory accesses, and a [`scheduler::ConcurrencyScheduler`]
//! that drives deterministic, randomized, or coverage-guided thread
//! interleavings for systematic concurrency testing.

pub mod race_detector;
pub mod scheduler;

pub use race_detector::{AccessRecord, RaceDetector, RaceInfo};
pub use scheduler::{ConcurrencyConfig, ConcurrencyScheduler, ScheduleStrategy};
