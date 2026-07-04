//! Omnisystem Async Runtime (OAR)
//!
//! Enterprise-grade async runtime with zero external dependencies.
//! Provides work-stealing thread pool, async task scheduling, and I/O multiplexing.
//!
//! # No External Dependencies
//!
//! This runtime is completely self-contained with no external crate dependencies,
//! providing immunity from supply-chain attacks and full auditability.
//!
//! # Key Features
//!
//! - **Work-stealing scheduler**: Optimal load balancing across CPU cores
//! - **Platform-specific I/O**: epoll (Linux), IOCP (Windows), kqueue (macOS)
//! - **Lock-free structures**: Minimal contention in hot paths
//! - **Fair mutexes**: FIFO wakeup order prevents starvation
//! - **Zero-copy async**: Minimal allocations in async operations
//! - **Integrated metrics**: Built-in observability
//!
//! # Architecture
//!
//! The runtime consists of:
//!
//! - **Executor**: Work-stealing thread pool managing task execution
//! - **Scheduler**: Fair async task scheduler with priority support
//! - **I/O Reactor**: Platform-specific I/O multiplexing
//! - **Synchronization**: Lock-free primitives and fair locks
//! - **Task Runtime**: Global runtime managing all threads

pub mod executor;
pub mod io;
pub mod scheduler;
pub mod synchronization;
pub mod task;

pub use executor::{Executor, Runtime};
pub use scheduler::Scheduler;
pub use task::Task;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Global Omnisystem Async Runtime instance
static RUNTIME: std::sync::OnceLock<Arc<Runtime>> = std::sync::OnceLock::new();

/// Initialize the global async runtime with specified number of worker threads.
///
/// # Panics
///
/// Panics if called more than once.
pub fn initialize_runtime(num_workers: usize) -> Arc<Runtime> {
    let num_workers = if num_workers == 0 {
        num_cpus()
    } else {
        num_workers
    };

    let runtime = Arc::new(Runtime::new(num_workers));
    RUNTIME.set(runtime.clone()).expect("Runtime already initialized");
    runtime
}

/// Get the global async runtime instance.
///
/// # Panics
///
/// Panics if runtime not initialized via `initialize_runtime`.
pub fn global_runtime() -> Arc<Runtime> {
    RUNTIME
        .get()
        .cloned()
        .expect("Runtime not initialized. Call initialize_runtime() first.")
}

/// Spawn a task on the global runtime.
///
/// # Example
///
/// ```ignore
/// oar::spawn(async {
///     println!("Running on OAR!");
/// }).await;
/// ```
pub async fn spawn<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let runtime = global_runtime();
    runtime.spawn(future).await
}

/// Block on an async task using the global runtime.
///
/// # Example
///
/// ```ignore
/// oar::block_on(async {
///     println!("Running synchronously!");
/// });
/// ```
pub fn block_on<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    let runtime = global_runtime();
    runtime.block_on(future)
}

/// Get the number of logical CPU cores.
fn num_cpus() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Runtime statistics for observability.
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Total tasks scheduled
    pub total_tasks: usize,
    /// Tasks currently running
    pub active_tasks: usize,
    /// Number of worker threads
    pub num_workers: usize,
    /// Total work-stealing attempts
    pub work_steal_attempts: usize,
    /// Successful work steals
    pub work_steal_successes: usize,
    /// Total context switches
    pub context_switches: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_initialization() {
        let runtime = initialize_runtime(2);
        assert_eq!(runtime.num_workers(), 2);
    }

    #[test]
    fn test_spawn_simple_task() {
        initialize_runtime(2);
        let result = block_on(async {
            let value = 42;
            value + 1
        });
        assert_eq!(result, 43);
    }

    #[test]
    fn test_cpu_count() {
        let cpus = num_cpus();
        assert!(cpus > 0);
    }
}
