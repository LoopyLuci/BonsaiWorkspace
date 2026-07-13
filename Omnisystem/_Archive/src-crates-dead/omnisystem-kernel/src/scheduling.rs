use parking_lot::RwLock;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use crate::KernelError;
use crate::process::{ProcessId, ThreadId, Thread, ThreadState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulingPolicy {
    FIFO,          // First-In-First-Out
    RoundRobin,    // Round-robin with time quantum
    PriorityBased, // Priority-based with preemption
    EDF,           // Earliest Deadline First
}

pub struct SchedulingQueue {
    ready_threads: VecDeque<Arc<Thread>>,
    policy: SchedulingPolicy,
}

impl SchedulingQueue {
    pub fn new(policy: SchedulingPolicy) -> Self {
        SchedulingQueue {
            ready_threads: VecDeque::new(),
            policy,
        }
    }

    pub fn enqueue(&mut self, thread: Arc<Thread>) {
        self.ready_threads.push_back(thread);
    }

    pub fn dequeue(&mut self) -> Option<Arc<Thread>> {
        self.ready_threads.pop_front()
    }

    pub fn peek(&self) -> Option<Arc<Thread>> {
        self.ready_threads.front().cloned()
    }

    pub fn size(&self) -> usize {
        self.ready_threads.len()
    }

    pub fn policy(&self) -> SchedulingPolicy {
        self.policy
    }
}

pub struct Scheduler {
    queues: RwLock<BTreeMap<u8, SchedulingQueue>>, // Priority -> Queue
    current_thread: RwLock<Option<Arc<Thread>>>,
    time_quantum: u32,                             // milliseconds
}

impl Scheduler {
    pub fn new() -> Self {
        let mut queues = BTreeMap::new();

        // Create 256 priority levels (0-255)
        for priority in 0..=255u8 {
            queues.insert(
                priority,
                SchedulingQueue::new(SchedulingPolicy::RoundRobin),
            );
        }

        Scheduler {
            queues: RwLock::new(queues),
            current_thread: RwLock::new(None),
            time_quantum: 10, // 10ms default
        }
    }

    pub fn add_thread(&self, thread: Arc<Thread>) -> Result<(), KernelError> {
        let priority = *thread.priority.read();
        let mut queues = self.queues.write();

        match queues.get_mut(&priority) {
            Some(queue) => {
                queue.enqueue(thread);
                Ok(())
            }
            None => Err(KernelError::Unknown("Invalid priority".to_string())),
        }
    }

    pub fn remove_thread(&self, thread_id: ThreadId) -> Result<(), KernelError> {
        let mut queues = self.queues.write();

        for (_priority, queue) in queues.iter_mut() {
            queue.ready_threads.retain(|t| t.id != thread_id);
        }

        Ok(())
    }

    pub fn schedule_next(&self) -> Option<Arc<Thread>> {
        let mut queues = self.queues.write();

        // Find highest priority queue with ready threads
        for priority in (0u8..=255).rev() {
            if let Some(queue) = queues.get_mut(&priority) {
                if let Some(thread) = queue.dequeue() {
                    *self.current_thread.write() = Some(Arc::clone(&thread));
                    return Some(thread);
                }
            }
        }

        None
    }

    pub fn yield_thread(&self, thread: Arc<Thread>) -> Result<(), KernelError> {
        let priority = *thread.priority.read();
        let mut queues = self.queues.write();

        match queues.get_mut(&priority) {
            Some(queue) => {
                queue.enqueue(thread);
                Ok(())
            }
            None => Err(KernelError::Unknown("Invalid priority".to_string())),
        }
    }

    pub fn get_current_thread(&self) -> Option<Arc<Thread>> {
        self.current_thread.read().clone()
    }

    pub fn set_thread_priority(
        &self,
        thread: Arc<Thread>,
        new_priority: u8,
    ) -> Result<(), KernelError> {
        *thread.priority.write() = new_priority;
        Ok(())
    }

    pub async fn run(&self) -> Result<(), KernelError> {
        loop {
            if let Some(thread) = self.schedule_next() {
                *thread.state.write() = ThreadState::Running;

                // Simulate thread execution
                tokio::time::sleep(std::time::Duration::from_millis(self.time_quantum as u64))
                    .await;

                // Yield back to scheduler
                *thread.state.write() = ThreadState::Ready;
                self.yield_thread(thread)?;
            } else {
                // No threads ready, sleep briefly
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }

    pub fn get_queue_depth(&self, priority: u8) -> usize {
        self.queues
            .read()
            .get(&priority)
            .map(|q| q.size())
            .unwrap_or(0)
    }

    pub fn total_ready_threads(&self) -> usize {
        self.queues
            .read()
            .values()
            .map(|q| q.size())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessManager;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new();
        assert_eq!(scheduler.total_ready_threads(), 0);
    }

    #[test]
    fn test_thread_scheduling() {
        let scheduler = Scheduler::new();
        let thread = Arc::new(Thread::new(1, 1));

        let result = scheduler.add_thread(thread.clone());
        assert!(result.is_ok());
        assert_eq!(scheduler.total_ready_threads(), 1);
    }

    #[tokio::test]
    async fn test_schedule_next() {
        let scheduler = Scheduler::new();
        let thread = Arc::new(Thread::new(1, 1));

        scheduler.add_thread(thread.clone()).unwrap();
        let next = scheduler.schedule_next();

        assert!(next.is_some());
        assert_eq!(next.unwrap().id, 1);
    }
}
