use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub memory_usage: u64,
    pub cpu_time: u64,
}

pub struct ProcessManager {
    processes: Arc<DashMap<u32, Process>>,
    next_pid: Arc<std::sync::Mutex<u32>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            next_pid: Arc::new(std::sync::Mutex::new(1)),
        }
    }

    pub fn create_process(&self, name: String) -> u32 {
        let mut pid = self.next_pid.lock().unwrap();
        let process_id = *pid;
        *pid += 1;

        let process = Process {
            pid: process_id,
            name,
            memory_usage: 0,
            cpu_time: 0,
        };
        self.processes.insert(process_id, process);
        process_id
    }

    pub fn get_process(&self, pid: u32) -> Option<Process> {
        self.processes.get(&pid).map(|p| p.clone())
    }

    pub fn terminate_process(&self, pid: u32) -> bool {
        self.processes.remove(&pid).is_some()
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn total_memory_usage(&self) -> u64 {
        self.processes.iter().map(|ref_| ref_.memory_usage).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_creation() {
        let pm = ProcessManager::new();
        let pid = pm.create_process("init".to_string());
        assert_eq!(pid, 1);
        assert_eq!(pm.process_count(), 1);
    }

    #[test]
    fn test_process_termination() {
        let pm = ProcessManager::new();
        let pid = pm.create_process("test".to_string());
        assert!(pm.terminate_process(pid));
        assert_eq!(pm.process_count(), 0);
    }
}
