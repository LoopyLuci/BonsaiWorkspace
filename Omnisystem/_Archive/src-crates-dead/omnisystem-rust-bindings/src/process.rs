/// Process and thread management bindings

use std::sync::Arc;
use omnisystem_kernel::process::{ProcessId, ThreadId};

pub struct ProcessHandle {
    process: Arc<omnisystem_kernel::process::Process>,
}

impl ProcessHandle {
    pub fn new(process: Arc<omnisystem_kernel::process::Process>) -> Self {
        ProcessHandle { process }
    }

    pub fn id(&self) -> ProcessId {
        self.process.id
    }

    pub fn thread_count(&self) -> usize {
        self.process.thread_count()
    }

    pub fn create_thread(&self) -> Result<ThreadHandle, String> {
        let tid = 1; // Simplified - would get from thread manager
        match self.process.create_thread(tid) {
            Ok(thread) => Ok(ThreadHandle::new(thread)),
            Err(e) => Err(format!("Failed to create thread: {:?}", e)),
        }
    }
}

pub struct ThreadHandle {
    thread: Arc<omnisystem_kernel::process::Thread>,
}

impl ThreadHandle {
    pub fn new(thread: Arc<omnisystem_kernel::process::Thread>) -> Self {
        ThreadHandle { thread }
    }

    pub fn id(&self) -> ThreadId {
        self.thread.id
    }

    pub fn process_id(&self) -> ProcessId {
        self.thread.process_id
    }
}
