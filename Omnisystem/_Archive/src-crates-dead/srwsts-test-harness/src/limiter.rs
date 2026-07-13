//! Resource limiting for vault environments

use crate::errors::{HarnessError, HarnessResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Resource limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory: u64,
    /// Maximum CPU time in milliseconds
    pub max_cpu_time_ms: u64,
    /// Maximum I/O throughput in MB/s
    pub max_io_mbps: u32,
    /// Maximum network bandwidth in Mbps
    pub max_network_mbps: u32,
    /// Maximum number of file descriptors
    pub max_fds: u32,
    /// Maximum processes/threads
    pub max_processes: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 512 * 1024 * 1024,    // 512 MB
            max_cpu_time_ms: 60 * 1000,       // 60 seconds
            max_io_mbps: 500,
            max_network_mbps: 1000,
            max_fds: 256,
            max_processes: 10,
        }
    }
}

impl ResourceLimits {
    /// Create strict limits for sandboxing
    pub fn strict() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,     // 64 MB
            max_cpu_time_ms: 10 * 1000,       // 10 seconds
            max_io_mbps: 50,
            max_network_mbps: 100,
            max_fds: 32,
            max_processes: 1,
        }
    }

    /// Create relaxed limits for normal testing
    pub fn relaxed() -> Self {
        Self {
            max_memory: 2 * 1024 * 1024 * 1024, // 2 GB
            max_cpu_time_ms: 300 * 1000,        // 5 minutes
            max_io_mbps: 2000,
            max_network_mbps: 10000,
            max_fds: 1024,
            max_processes: 100,
        }
    }
}

/// Resource limiter
pub struct ResourceLimiter {
    /// Limits configuration
    pub limits: ResourceLimits,
    /// Current memory usage
    memory_used: Arc<AtomicU64>,
    /// Current CPU time used
    cpu_time_used_ms: Arc<AtomicU64>,
    /// Current I/O operations
    io_ops_count: Arc<AtomicU64>,
    /// Current I/O bytes
    io_bytes: Arc<AtomicU64>,
    /// Open file descriptors count
    fds_open: Arc<AtomicU64>,
    /// Active process count
    processes_active: Arc<AtomicU64>,
}

impl ResourceLimiter {
    /// Create a new resource limiter
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            memory_used: Arc::new(AtomicU64::new(0)),
            cpu_time_used_ms: Arc::new(AtomicU64::new(0)),
            io_ops_count: Arc::new(AtomicU64::new(0)),
            io_bytes: Arc::new(AtomicU64::new(0)),
            fds_open: Arc::new(AtomicU64::new(0)),
            processes_active: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check and allocate memory
    pub fn allocate_memory(&self, size: u64) -> HarnessResult<()> {
        let current = self.memory_used.load(Ordering::SeqCst);
        if current + size > self.limits.max_memory {
            return Err(HarnessError::ResourceLimitExceeded(
                format!(
                    "memory limit exceeded: {} + {} > {}",
                    current, size, self.limits.max_memory
                ),
            ));
        }

        self.memory_used.fetch_add(size, Ordering::SeqCst);
        Ok(())
    }

    /// Release memory
    pub fn release_memory(&self, size: u64) {
        let current = self.memory_used.load(Ordering::SeqCst);
        if size <= current {
            self.memory_used.fetch_sub(size, Ordering::SeqCst);
        }
    }

    /// Record CPU time
    pub fn record_cpu_time(&self, time_ms: u64) -> HarnessResult<()> {
        let current = self.cpu_time_used_ms.load(Ordering::SeqCst);
        if current + time_ms > self.limits.max_cpu_time_ms {
            return Err(HarnessError::ResourceLimitExceeded(
                format!("CPU time limit exceeded: {} ms", current + time_ms),
            ));
        }

        self.cpu_time_used_ms.fetch_add(time_ms, Ordering::SeqCst);
        Ok(())
    }

    /// Record I/O operation
    pub fn record_io(&self, bytes: u64) -> HarnessResult<()> {
        self.io_ops_count.fetch_add(1, Ordering::SeqCst);
        self.io_bytes.fetch_add(bytes, Ordering::SeqCst);
        Ok(())
    }

    /// Open a file descriptor
    pub fn open_fd(&self) -> HarnessResult<()> {
        let current = self.fds_open.load(Ordering::SeqCst);
        if current + 1 > self.limits.max_fds as u64 {
            return Err(HarnessError::ResourceLimitExceeded(
                "file descriptor limit exceeded".to_string(),
            ));
        }

        self.fds_open.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Close a file descriptor
    pub fn close_fd(&self) {
        let current = self.fds_open.load(Ordering::SeqCst);
        if current > 0 {
            self.fds_open.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Start a new process
    pub fn spawn_process(&self) -> HarnessResult<()> {
        let current = self.processes_active.load(Ordering::SeqCst);
        if current + 1 > self.limits.max_processes as u64 {
            return Err(HarnessError::ResourceLimitExceeded(
                "process limit exceeded".to_string(),
            ));
        }

        self.processes_active.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Exit a process
    pub fn exit_process(&self) {
        let current = self.processes_active.load(Ordering::SeqCst);
        if current > 0 {
            self.processes_active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Get current resource usage
    pub fn get_usage(&self) -> ResourceUsage {
        ResourceUsage {
            memory_bytes: self.memory_used.load(Ordering::SeqCst),
            cpu_time_ms: self.cpu_time_used_ms.load(Ordering::SeqCst),
            io_ops: self.io_ops_count.load(Ordering::SeqCst),
            io_bytes: self.io_bytes.load(Ordering::SeqCst),
            fds_open: self.fds_open.load(Ordering::SeqCst),
            processes_active: self.processes_active.load(Ordering::SeqCst),
        }
    }

    /// Check if resources are available
    pub fn has_resources(&self) -> bool {
        let usage = self.get_usage();
        usage.memory_bytes < self.limits.max_memory
            && usage.cpu_time_ms < self.limits.max_cpu_time_ms
            && usage.fds_open < self.limits.max_fds as u64
            && usage.processes_active < self.limits.max_processes as u64
    }

    /// Reset usage counters
    pub fn reset(&self) {
        self.memory_used.store(0, Ordering::SeqCst);
        self.cpu_time_used_ms.store(0, Ordering::SeqCst);
        self.io_ops_count.store(0, Ordering::SeqCst);
        self.io_bytes.store(0, Ordering::SeqCst);
        self.fds_open.store(0, Ordering::SeqCst);
        self.processes_active.store(0, Ordering::SeqCst);
    }
}

/// Resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory used in bytes
    pub memory_bytes: u64,
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
    /// I/O operations count
    pub io_ops: u64,
    /// I/O bytes transferred
    pub io_bytes: u64,
    /// Open file descriptors
    pub fds_open: u64,
    /// Active processes
    pub processes_active: u64,
}

impl ResourceUsage {
    /// Get memory usage percentage
    pub fn memory_percent(&self, limit: u64) -> f64 {
        if limit == 0 {
            return 0.0;
        }
        (self.memory_bytes as f64 / limit as f64) * 100.0
    }

    /// Get CPU time percentage
    pub fn cpu_time_percent(&self, limit: u64) -> f64 {
        if limit == 0 {
            return 0.0;
        }
        (self.cpu_time_ms as f64 / limit as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory, 512 * 1024 * 1024);
    }

    #[test]
    fn test_resource_limits_strict() {
        let limits = ResourceLimits::strict();
        assert!(limits.max_memory < ResourceLimits::default().max_memory);
    }

    #[test]
    fn test_resource_limits_relaxed() {
        let limits = ResourceLimits::relaxed();
        assert!(limits.max_memory > ResourceLimits::default().max_memory);
    }

    #[test]
    fn test_resource_limiter_memory() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits);

        let result = limiter.allocate_memory(100);
        assert!(result.is_ok());

        let usage = limiter.get_usage();
        assert_eq!(usage.memory_bytes, 100);
    }

    #[test]
    fn test_resource_limiter_memory_exceeded() {
        let mut limits = ResourceLimits::default();
        limits.max_memory = 100;

        let limiter = ResourceLimiter::new(limits);
        let result = limiter.allocate_memory(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_limiter_fds() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits);

        for _ in 0..10 {
            let result = limiter.open_fd();
            assert!(result.is_ok());
        }

        let usage = limiter.get_usage();
        assert_eq!(usage.fds_open, 10);
    }

    #[test]
    fn test_resource_limiter_processes() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits);

        let result = limiter.spawn_process();
        assert!(result.is_ok());

        let usage = limiter.get_usage();
        assert_eq!(usage.processes_active, 1);
    }

    #[test]
    fn test_resource_usage_percentages() {
        let usage = ResourceUsage {
            memory_bytes: 256 * 1024 * 1024,
            cpu_time_ms: 50,
            io_ops: 100,
            io_bytes: 1024,
            fds_open: 10,
            processes_active: 2,
        };

        let mem_percent = usage.memory_percent(512 * 1024 * 1024);
        assert_eq!(mem_percent, 50.0);
    }
}
