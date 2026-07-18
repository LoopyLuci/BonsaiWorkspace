//! bedf-sanitizers: a simulated memory-sanitizer toolkit -- track
//! allocation/deallocation/access events and detect use-after-free and
//! buffer-overflow issues, then roll them up into an ASAN/MSAN/TSAN/LSAN-
//! style [`SanitizerReport`].

pub mod config;
pub mod error;
pub mod interfaces;
pub mod memory_tracker;
pub mod sanitizer_report;

pub use config::{Config, SanitizerConfig};
pub use error::{Error, Result};
pub use interfaces::Component;
pub use memory_tracker::{AccessRecord, AllocationRecord, MemoryTracker};
pub use sanitizer_report::{IssueType, MemoryIssue, SanitizerReport};
