/// Process Scheduler
/// Fair scheduling with priority levels

use std::sync::Arc;
use std::collections::VecDeque;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Priority Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Idle = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    RealTime = 4,
}

/// Scheduled Task
#[derive(Debug, Clone, Copy)]
pub struct ScheduledTask {
    pub pid: u64,
    pub priority: Priority,
    pub cpu_time_allocated_ms: u64,
    pub cpu_time_used_ms: u64,
}

/// Scheduler - Fair priority-based scheduling
pub struct Scheduler {
    queues: Arc<DashMap<Priority, VecDeque<ScheduledTask>>>,
    current_task: Arc<std::sync::Mutex<Option<ScheduledTask>>>,
    total_scheduled: Arc<std::sync::atomic::AtomicU64>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            queues: Arc::new(DashMap::new()),
            current_task: Arc::new(std::sync::Mutex::new(None)),
            total_scheduled: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Add task to scheduler
    pub fn enqueue(&self, pid: u64, priority: Priority) -> anyhow::Result<()> {
        let task = ScheduledTask {
            pid,
            priority,
            cpu_time_allocated_ms: match priority {
                Priority::Idle => 10,
                Priority::Low => 20,
                Priority::Normal => 50,
                Priority::High => 100,
                Priority::RealTime => 500,
            },
            cpu_time_used_ms: 0,
        };

        let mut queue = self.queues.entry(priority).or_insert(VecDeque::new());
        queue.push_back(task);
        self.total_scheduled.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Get next task to run
    pub fn next_task(&self) -> Option<ScheduledTask> {
        // Check queues in priority order (high to low)
        let priorities = [
            Priority::RealTime,
            Priority::High,
            Priority::Normal,
            Priority::Low,
            Priority::Idle,
        ];

        for priority in &priorities {
            if let Some(mut queue) = self.queues.get_mut(priority) {
                if let Some(task) = queue.pop_front() {
                    *self.current_task.lock().unwrap() = Some(task.clone());
                    return Some(task);
                }
            }
        }

        None
    }

    /// Mark task as needing rescheduling
    pub fn reschedule(&self, mut task: ScheduledTask) -> anyhow::Result<()> {
        task.cpu_time_used_ms = 0;
        let mut queue = self.queues.entry(task.priority).or_insert(VecDeque::new());
        queue.push_back(task);
        Ok(())
    }

    /// Remove task from scheduler
    pub fn remove(&self, pid: u64) -> bool {
        for mut entry in self.queues.iter_mut() {
            let queue = entry.value_mut();
            if let Some(pos) = queue.iter().position(|t| t.pid == pid) {
                queue.remove(pos);
                self.total_scheduled.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                return true;
            }
        }
        false
    }

    /// Scheduler tick - process one task quantum
    pub async fn tick(&self) -> anyhow::Result<()> {
        if let Some(task) = self.next_task() {
            // Simulate running the task for its time quantum
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            // Reschedule if needed
            let mut updated_task = task.clone();
            updated_task.cpu_time_used_ms += 1;

            if updated_task.cpu_time_used_ms < updated_task.cpu_time_allocated_ms {
                self.reschedule(updated_task)?;
            }
        }

        Ok(())
    }

    /// Get current task
    pub fn current(&self) -> Option<ScheduledTask> {
        self.current_task.lock().unwrap().clone()
    }

    /// Get queue length for priority
    pub fn queue_length(&self, priority: Priority) -> usize {
        self.queues
            .get(&priority)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// Total scheduled tasks
    pub fn total_tasks(&self) -> u64 {
        self.total_scheduled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_enqueue() {
        let scheduler = Scheduler::new();
        scheduler.enqueue(1, Priority::Normal).unwrap();
        scheduler.enqueue(2, Priority::High).unwrap();
        scheduler.enqueue(3, Priority::Low).unwrap();

        assert_eq!(scheduler.total_tasks(), 3);
    }

    #[test]
    fn test_priority_ordering() {
        let scheduler = Scheduler::new();
        scheduler.enqueue(1, Priority::Low).unwrap();
        scheduler.enqueue(2, Priority::High).unwrap();
        scheduler.enqueue(3, Priority::Normal).unwrap();

        // High priority should be scheduled first
        let task = scheduler.next_task().unwrap();
        assert_eq!(task.pid, 2);
    }

    #[test]
    fn test_scheduler_removal() {
        let scheduler = Scheduler::new();
        scheduler.enqueue(1, Priority::Normal).unwrap();
        scheduler.enqueue(2, Priority::Normal).unwrap();

        assert!(scheduler.remove(1));
        assert_eq!(scheduler.total_tasks(), 1);
        assert!(!scheduler.remove(999));
    }
}
