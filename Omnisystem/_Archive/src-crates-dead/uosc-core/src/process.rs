/// Process Management System
/// Unified process abstraction across Windows, macOS, Linux

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Process Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Created,
    Running,
    Suspended,
    Waiting,
    Terminated,
    Failed,
}

/// Process Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub status: ProcessStatus,
    pub created_at: u64,
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub priority: i32,
}

impl Process {
    pub fn new(id: u64, name: String) -> Self {
        Process {
            id,
            name,
            parent_id: None,
            status: ProcessStatus::Created,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            cpu_time_ms: 0,
            memory_bytes: 0,
            priority: 0,
        }
    }

    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Process Manager
pub struct ProcessManager {
    processes: Arc<DashMap<u64, Process>>,
    next_pid: Arc<std::sync::atomic::AtomicU64>,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: Arc::new(DashMap::new()),
            next_pid: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Create a new process
    pub fn create(&self, name: String) -> anyhow::Result<Process> {
        let pid = self.next_pid.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut process = Process::new(pid, name);
        process.status = ProcessStatus::Running;

        self.processes.insert(pid, process.clone());
        tracing::debug!("Process created: {} (pid: {})", process.name, pid);

        Ok(process)
    }

    /// Create a child process
    pub fn create_child(&self, parent_id: u64, name: String) -> anyhow::Result<Process> {
        // Verify parent exists
        if !self.processes.contains_key(&parent_id) {
            return Err(anyhow::anyhow!("Parent process not found: {}", parent_id));
        }

        let pid = self.next_pid.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut process = Process::new(pid, name);
        process.parent_id = Some(parent_id);
        process.status = ProcessStatus::Running;

        self.processes.insert(pid, process.clone());
        tracing::debug!("Child process created: {} (pid: {}, parent: {})", process.name, pid, parent_id);

        Ok(process)
    }

    /// Get process by ID
    pub fn get(&self, pid: u64) -> Option<Process> {
        self.processes.get(&pid).map(|p| p.clone())
    }

    /// Get all processes
    pub fn list_all(&self) -> Vec<Process> {
        self.processes.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Suspend a process
    pub fn suspend(&self, pid: u64) -> anyhow::Result<()> {
        match self.processes.get_mut(&pid) {
            Some(mut process) => {
                if process.status == ProcessStatus::Running {
                    process.status = ProcessStatus::Suspended;
                    tracing::debug!("Process suspended: {} (pid: {})", process.name, pid);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Cannot suspend non-running process"))
                }
            }
            None => Err(anyhow::anyhow!("Process not found: {}", pid)),
        }
    }

    /// Resume a suspended process
    pub fn resume(&self, pid: u64) -> anyhow::Result<()> {
        match self.processes.get_mut(&pid) {
            Some(mut process) => {
                if process.status == ProcessStatus::Suspended {
                    process.status = ProcessStatus::Running;
                    tracing::debug!("Process resumed: {} (pid: {})", process.name, pid);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Cannot resume non-suspended process"))
                }
            }
            None => Err(anyhow::anyhow!("Process not found: {}", pid)),
        }
    }

    /// Terminate a process
    pub fn terminate(&self, pid: u64) -> anyhow::Result<()> {
        match self.processes.get_mut(&pid) {
            Some(mut process) => {
                process.status = ProcessStatus::Terminated;
                tracing::debug!("Process terminated: {} (pid: {})", process.name, pid);
                Ok(())
            }
            None => Err(anyhow::anyhow!("Process not found: {}", pid)),
        }
    }

    /// Terminate all processes
    pub async fn terminate_all(&self) -> anyhow::Result<()> {
        let pids: Vec<u64> = self.processes.iter().map(|entry| entry.key().clone()).collect();
        for pid in pids {
            let _ = self.terminate(pid);
        }
        tracing::info!("All processes terminated");
        Ok(())
    }

    /// Get child processes
    pub fn get_children(&self, parent_id: u64) -> Vec<Process> {
        self.processes
            .iter()
            .filter(|entry| entry.value().parent_id == Some(parent_id))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Update process statistics
    pub fn update_stats(&self, pid: u64, cpu_time_ms: u64, memory_bytes: u64) -> anyhow::Result<()> {
        match self.processes.get_mut(&pid) {
            Some(mut process) => {
                process.cpu_time_ms = cpu_time_ms;
                process.memory_bytes = memory_bytes;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Process not found: {}", pid)),
        }
    }

    /// Process count
    pub fn count(&self) -> usize {
        self.processes.len()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_creation() {
        let manager = ProcessManager::new();
        let proc = manager.create("test_process".to_string()).unwrap();
        assert_eq!(proc.name, "test_process");
        assert_eq!(proc.status, ProcessStatus::Running);
    }

    #[test]
    fn test_child_process_creation() {
        let manager = ProcessManager::new();
        let parent = manager.create("parent".to_string()).unwrap();
        let child = manager.create_child(parent.id, "child".to_string()).unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
        assert_eq!(manager.get_children(parent.id).len(), 1);
    }

    #[test]
    fn test_process_status_changes() {
        let manager = ProcessManager::new();
        let proc = manager.create("test".to_string()).unwrap();

        manager.suspend(proc.id).unwrap();
        assert_eq!(manager.get(proc.id).unwrap().status, ProcessStatus::Suspended);

        manager.resume(proc.id).unwrap();
        assert_eq!(manager.get(proc.id).unwrap().status, ProcessStatus::Running);

        manager.terminate(proc.id).unwrap();
        assert_eq!(manager.get(proc.id).unwrap().status, ProcessStatus::Terminated);
    }

    #[test]
    fn test_process_listing() {
        let manager = ProcessManager::new();
        manager.create("proc1".to_string()).unwrap();
        manager.create("proc2".to_string()).unwrap();
        manager.create("proc3".to_string()).unwrap();

        assert_eq!(manager.count(), 3);
        assert_eq!(manager.list_all().len(), 3);
    }
}
