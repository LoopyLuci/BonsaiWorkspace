use crate::memory::AddressSpace;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use crate::KernelError;

pub type ProcessId = u64;
pub type ThreadId = u64;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// Thread control block
pub struct Thread {
    pub id: ThreadId,
    pub state: RwLock<ThreadState>,
    pub process_id: ProcessId,
    pub stack_pointer: RwLock<u64>,
    pub instruction_pointer: RwLock<u64>,
    pub priority: RwLock<u8>,
}

impl Thread {
    pub fn new(id: ThreadId, process_id: ProcessId) -> Self {
        Thread {
            id,
            state: RwLock::new(ThreadState::Ready),
            process_id,
            stack_pointer: RwLock::new(0),
            instruction_pointer: RwLock::new(0),
            priority: RwLock::new(128),
        }
    }
}

/// Process control block
pub struct Process {
    pub id: ProcessId,
    pub state: RwLock<ProcessState>,
    pub address_space: Arc<AddressSpace>,
    pub threads: RwLock<BTreeMap<ThreadId, Arc<Thread>>>,
    pub priority: RwLock<u8>,
    pub parent_pid: Option<ProcessId>,
}

impl Process {
    pub fn new(id: ProcessId, parent_pid: Option<ProcessId>) -> Self {
        Process {
            id,
            state: RwLock::new(ProcessState::Ready),
            address_space: Arc::new(AddressSpace::new()),
            threads: RwLock::new(BTreeMap::new()),
            priority: RwLock::new(128),
            parent_pid,
        }
    }

    pub fn create_thread(&self, tid: ThreadId) -> Result<Arc<Thread>, KernelError> {
        let thread = Arc::new(Thread::new(tid, self.id));
        self.threads.write().insert(tid, Arc::clone(&thread));
        Ok(thread)
    }

    pub fn get_thread(&self, tid: ThreadId) -> Option<Arc<Thread>> {
        self.threads.read().get(&tid).cloned()
    }

    pub fn thread_count(&self) -> usize {
        self.threads.read().len()
    }
}

/// Process manager
pub struct ProcessManager {
    processes: RwLock<BTreeMap<ProcessId, Arc<Process>>>,
    next_pid: RwLock<ProcessId>,
    next_tid: RwLock<ThreadId>,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: RwLock::new(BTreeMap::new()),
            next_pid: RwLock::new(1),
            next_tid: RwLock::new(1),
        }
    }

    pub fn create_process(&self, parent_pid: Option<ProcessId>) -> Result<Arc<Process>, KernelError> {
        let pid = {
            let mut next_pid = self.next_pid.write();
            let pid = *next_pid;
            *next_pid += 1;
            pid
        };

        let process = Arc::new(Process::new(pid, parent_pid));
        self.processes.write().insert(pid, Arc::clone(&process));

        Ok(process)
    }

    pub fn get_process(&self, pid: ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(&pid).cloned()
    }

    pub fn create_thread(&self, pid: ProcessId) -> Result<Arc<Thread>, KernelError> {
        let process = self
            .get_process(pid)
            .ok_or(KernelError::ProcessError("Process not found".to_string()))?;

        let tid = {
            let mut next_tid = self.next_tid.write();
            let tid = *next_tid;
            *next_tid += 1;
            tid
        };

        process.create_thread(tid)
    }

    pub fn terminate_process(&self, pid: ProcessId) -> Result<(), KernelError> {
        let process = self
            .get_process(pid)
            .ok_or(KernelError::ProcessError("Process not found".to_string()))?;

        *process.state.write() = ProcessState::Terminated;
        self.processes.write().remove(&pid);

        Ok(())
    }

    pub fn get_all_processes(&self) -> Vec<Arc<Process>> {
        self.processes
            .read()
            .values()
            .cloned()
            .collect()
    }

    pub fn process_count(&self) -> usize {
        self.processes.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_creation() {
        let pm = ProcessManager::new();
        let process = pm.create_process(None);
        assert!(process.is_ok());
    }

    #[test]
    fn test_thread_creation() {
        let pm = ProcessManager::new();
        let process = pm.create_process(None).unwrap();
        let thread = pm.create_thread(process.id);
        assert!(thread.is_ok());
    }

    #[test]
    fn test_process_hierarchy() {
        let pm = ProcessManager::new();
        let parent = pm.create_process(None).unwrap();
        let child = pm.create_process(Some(parent.id)).unwrap();

        assert_eq!(child.parent_pid, Some(parent.id));
    }
}
