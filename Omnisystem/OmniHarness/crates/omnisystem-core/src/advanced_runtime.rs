//! Advanced Multi-threaded Runtime with Event-Sourcing and GPU Support
//!
//! This module provides:
//! - Event-sourcing state management
//! - Multi-threaded actor architecture
//! - GPU acceleration framework
//! - Structured logging
//! - Resource pooling and efficiency
//! - Work-stealing scheduler
//! - Zero-copy data structures

use std::sync::Arc;
use tokio::sync::RwLock;

pub mod event_sourcing;
pub mod actor_system;
pub mod work_scheduler;
pub mod gpu_runtime;
pub mod structured_logging;
pub mod resource_pool;

pub use event_sourcing::{Event, EventStore, EventSourced};
pub use actor_system::{Actor, ActorRef, ActorSystem, ActorMessage};
pub use gpu_runtime::{GPURuntime, GPUKernel, GPUMemoryPool};
pub use structured_logging::{StructuredLogger, LogContext, LogEvent};
pub use resource_pool::{MemoryPool, BufferPool};
pub use work_scheduler::{WorkScheduler, WorkStealer, Task, TaskPriority};

/// Global omnisystem runtime configuration
#[derive(Clone, Debug)]
pub struct OmnisystemConfig {
    /// Number of worker threads (0 = auto-detect cores)
    pub num_workers: usize,

    /// Number of GPU devices to use (0 = auto-detect)
    pub num_gpu_devices: usize,

    /// Max GPU memory per device in GB
    pub gpu_memory_limit_gb: u32,

    /// Max system RAM for buffer pools in GB
    pub memory_limit_gb: u32,

    /// Enable event sourcing
    pub enable_event_sourcing: bool,

    /// Enable structured logging
    pub enable_structured_logging: bool,

    /// Logging level
    pub log_level: LogLevel,

    /// Event log retention (number of events)
    pub event_log_retention: usize,

    /// State snapshot interval (events between snapshots)
    pub snapshot_interval: usize,

    /// Enable zero-copy transfers
    pub enable_zero_copy: bool,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

impl LogLevel {
    pub fn to_level_value(self) -> usize {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
            LogLevel::Critical => 5,
        }
    }

    pub fn from_level_value(value: usize) -> Self {
        match value {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Critical,
            _ => LogLevel::Info,
        }
    }
}

impl Default for OmnisystemConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus::get(),
            num_gpu_devices: 0, // Auto-detect
            gpu_memory_limit_gb: 8,
            memory_limit_gb: 2,
            enable_event_sourcing: true,
            enable_structured_logging: true,
            log_level: LogLevel::Info,
            event_log_retention: 1_000_000,
            snapshot_interval: 10_000,
            enable_zero_copy: true,
        }
    }
}

/// Main Omnisystem runtime orchestrator
pub struct OmnisystemRuntime {
    config: OmnisystemConfig,

    /// Work-stealing scheduler for distributed tasks
    scheduler: Arc<WorkScheduler>,

    /// Actor system for message-passing concurrency
    actor_system: Arc<ActorSystem>,

    /// GPU runtime for heterogeneous computing
    gpu_runtime: Arc<GPURuntime>,

    /// Structured logging system
    logger: Arc<StructuredLogger>,

    /// Memory pool for zero-copy operations
    memory_pool: Arc<MemoryPool>,

    /// Buffer pool for efficient I/O
    buffer_pool: Arc<BufferPool>,

    /// Event store for event sourcing
    event_store: Arc<EventStore>,

    /// Runtime metrics
    metrics: Arc<RwLock<RuntimeMetrics>>,
}

#[derive(Default, Clone, Debug)]
pub struct RuntimeMetrics {
    pub total_tasks_executed: u64,
    pub tasks_per_second: f64,
    pub total_gpu_tasks: u64,
    pub gpu_utilization: f32,
    pub cpu_utilization: f32,
    pub memory_used_mb: u32,
    pub gpu_memory_used_mb: u32,
    pub active_workers: usize,
    pub event_log_size: usize,
}

impl OmnisystemRuntime {
    /// Create new runtime with default configuration
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(OmnisystemConfig::default()).await
    }

    /// Create new runtime with custom configuration
    pub async fn with_config(config: OmnisystemConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize components
        let logger = Arc::new(StructuredLogger::new(config.log_level));
        let scheduler = Arc::new(WorkScheduler::new(config.num_workers));
        let gpu_runtime = Arc::new(GPURuntime::new(config.num_gpu_devices).await?);
        let memory_pool = Arc::new(MemoryPool::new(config.memory_limit_gb as usize * 1024));
        let buffer_pool = Arc::new(BufferPool::new());
        let event_store = Arc::new(EventStore::new(config.event_log_retention));

        let actor_system = Arc::new(ActorSystem::new(config.num_workers));

        let init_event = LogEvent::new(
            LogLevel::Info,
            format!(
                "Omnisystem Runtime initialized: {} CPU workers, {} GPU devices",
                config.num_workers, config.num_gpu_devices
            ),
        );
        logger.log(init_event);

        Ok(Self {
            config,
            scheduler,
            actor_system,
            gpu_runtime,
            logger,
            memory_pool,
            buffer_pool,
            event_store,
            metrics: Arc::new(RwLock::new(RuntimeMetrics::default())),
        })
    }

    /// Get reference to actor system
    pub fn actor_system(&self) -> Arc<ActorSystem> {
        self.actor_system.clone()
    }

    /// Get reference to GPU runtime
    pub fn gpu_runtime(&self) -> Arc<GPURuntime> {
        self.gpu_runtime.clone()
    }

    /// Get reference to logger
    pub fn logger(&self) -> Arc<StructuredLogger> {
        self.logger.clone()
    }

    /// Get reference to work scheduler
    pub fn scheduler(&self) -> Arc<WorkScheduler> {
        self.scheduler.clone()
    }

    /// Get reference to memory pool
    pub fn memory_pool(&self) -> Arc<MemoryPool> {
        self.memory_pool.clone()
    }

    /// Get current runtime metrics
    pub async fn metrics(&self) -> RuntimeMetrics {
        self.metrics.read().await.clone()
    }

    /// Update metrics
    pub async fn update_metrics<F>(&self, f: F)
    where
        F: FnOnce(&mut RuntimeMetrics),
    {
        let mut metrics = self.metrics.write().await;
        f(&mut metrics);
    }

    /// Shutdown runtime gracefully
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        let shutdown_event = LogEvent::new(LogLevel::Info, "Shutting down Omnisystem Runtime");
        self.logger.log(shutdown_event);

        self.actor_system.shutdown().await?;
        self.gpu_runtime.shutdown().await?;
        self.scheduler.shutdown().await?;

        Ok(())
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_RUNTIME: tokio::sync::RwLock<Option<Arc<OmnisystemRuntime>>> =
        tokio::sync::RwLock::new(None);
}

/// Initialize global runtime
pub async fn init_global_runtime(config: OmnisystemConfig) -> Result<Arc<OmnisystemRuntime>, Box<dyn std::error::Error>> {
    let runtime = Arc::new(OmnisystemRuntime::with_config(config).await?);
    let mut global = GLOBAL_RUNTIME.write().await;
    *global = Some(runtime.clone());
    Ok(runtime)
}

/// Get global runtime reference
pub async fn global_runtime() -> Result<Arc<OmnisystemRuntime>, &'static str> {
    let global = GLOBAL_RUNTIME.read().await;
    global.as_ref().cloned().ok_or("Runtime not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_creation() {
        let config = OmnisystemConfig {
            num_workers: 4,
            num_gpu_devices: 0,
            ..Default::default()
        };

        let runtime = OmnisystemRuntime::with_config(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_metrics() {
        let config = OmnisystemConfig::default();
        let runtime = OmnisystemRuntime::with_config(config).await.unwrap();

        let metrics = runtime.metrics().await;
        assert_eq!(metrics.total_tasks_executed, 0);
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let runtime = OmnisystemRuntime::new().await.unwrap();
        let result = runtime.shutdown().await;
        assert!(result.is_ok());
    }
}
