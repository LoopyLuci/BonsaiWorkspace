//! omnisystem-runtime: a small priority-scheduled async task runtime.
//!
//! Provides a [`Task`] model with priorities, a [`Scheduler`] that orders
//! pending tasks by priority, a [`TaskExecutor`] that runs them, a
//! [`ResourcePool`] for bounded resource allocation, and [`RuntimeMetrics`]
//! for tracking completion/failure counts.

pub mod core;
pub mod error;
pub mod executor;
pub mod metrics;
pub mod pool;
pub mod scheduler;
pub mod types;

pub use core::Core;
pub use error::{Result, RuntimeError};
pub use executor::TaskExecutor;
pub use metrics::RuntimeMetrics;
pub use pool::{ResourceMetrics, ResourcePool};
pub use scheduler::Scheduler;
pub use types::{Priority, Task, TaskMetrics, TaskState};
