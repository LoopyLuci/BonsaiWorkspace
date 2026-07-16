//! Data pipeline: a small in-memory ETL toolkit (extract/transform/load)
//! plus a staged pipeline runner and a cron-style scheduler.

pub mod error;
pub mod extractor;
pub mod loader;
pub mod pipeline;
pub mod scheduler;
pub mod transformer;
pub mod types;

pub use error::{Error, PipelineError, Result};
pub use extractor::Extractor;
pub use loader::{DestinationType, LoadDestination, Loader};
pub use pipeline::{ExecutionRecord, Pipeline, PipelineStage, StageStatus, StageType};
pub use scheduler::{PipelineScheduler, Schedule, ScheduleFrequency};
pub use transformer::Transformer;
pub use types::State;
