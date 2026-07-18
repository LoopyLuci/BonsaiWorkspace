//! omnisystem-async: tokio-backed async task management, spawning helpers,
//! and synchronization wrappers.

pub mod error;
pub mod executor;
pub mod spawn;
pub mod sync;
pub mod types;

pub use error::{Error, Result};
pub use executor::{AsyncTask, TaskExecutor, TaskId, TaskResult, TaskStatus};
pub use spawn::{join_all, sleep, spawn, spawn_blocking, timeout, SpawnFuture};
pub use sync::{
    broadcast, create_broadcast, create_channel, mpsc, AsyncBarrier, AsyncLock, AsyncRwLock,
    AsyncSemaphore,
};
pub use types::State;

use std::sync::OnceLock;

/// A lazily-initialized global tokio runtime shared by [`spawn::spawn`] and
/// [`spawn::spawn_blocking`], so callers don't need to thread a `Runtime`
/// handle through every layer of the crate.
pub struct GlobalRuntime(OnceLock<tokio::runtime::Runtime>);

impl GlobalRuntime {
    const fn new() -> Self {
        Self(OnceLock::new())
    }

    /// Get (initializing on first use) the underlying tokio runtime.
    pub fn tokio_runtime(&self) -> &tokio::runtime::Runtime {
        self.0.get_or_init(|| {
            tokio::runtime::Runtime::new().expect("failed to create global omnisystem-async runtime")
        })
    }

    /// Spawn a future onto the global runtime.
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tokio_runtime().spawn(future)
    }
}

/// The process-wide async runtime used by [`spawn`]'s free functions.
pub static GLOBAL_RUNTIME: GlobalRuntime = GlobalRuntime::new();
