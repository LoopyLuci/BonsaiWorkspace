use crate::{Task, Priority, Result};
use std::collections::BinaryHeap;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct Scheduler {
    queue: Arc<Mutex<BinaryHeap<(Priority, String)>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
        }
    }

    pub fn schedule(&self, task: &Task) -> Result<()> {
        let mut queue = self.queue.lock();
        queue.push((task.priority, task.id.clone()));
        tracing::info!("Scheduled task: {}", task.id);
        Ok(())
    }

    pub fn next_task(&self) -> Option<String> {
        let mut queue = self.queue.lock();
        queue.pop().map(|(_, id)| id)
    }

    pub fn queue_size(&self) -> usize {
        self.queue.lock().len()
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
    use crate::Task;

    #[test]
    fn test_scheduler_priority_order() {
        let scheduler = Scheduler::new();
        let t1 = Task::new("t1".to_string(), Priority::Low);
        let t2 = Task::new("t2".to_string(), Priority::High);

        scheduler.schedule(&t1).unwrap();
        scheduler.schedule(&t2).unwrap();

        assert_eq!(scheduler.next_task(), Some("t2".to_string()));
    }

    #[test]
    fn test_queue_size() {
        let scheduler = Scheduler::new();
        assert_eq!(scheduler.queue_size(), 0);
    }
}
