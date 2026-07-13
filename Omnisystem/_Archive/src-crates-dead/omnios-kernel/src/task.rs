use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub name: String,
    pub state: TaskState,
    pub priority: u8,
}

pub struct TaskScheduler {
    tasks: Arc<DashMap<u32, Task>>,
    next_id: Arc<std::sync::Mutex<u32>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            next_id: Arc::new(std::sync::Mutex::new(1)),
        }
    }

    pub fn create_task(&self, name: String, priority: u8) -> u32 {
        let mut id = self.next_id.lock().unwrap();
        let task_id = *id;
        *id += 1;

        let task = Task {
            id: task_id,
            name,
            state: TaskState::Ready,
            priority,
        };
        self.tasks.insert(task_id, task);
        task_id
    }

    pub fn get_task(&self, id: u32) -> Option<Task> {
        self.tasks.get(&id).map(|t| t.clone())
    }

    pub fn set_task_state(&self, id: u32, state: TaskState) -> bool {
        if let Some(mut task) = self.tasks.get_mut(&id) {
            task.state = state;
            true
        } else {
            false
        }
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let scheduler = TaskScheduler::new();
        let id = scheduler.create_task("test_task".to_string(), 5);
        assert_eq!(id, 1);
        assert_eq!(scheduler.task_count(), 1);
    }

    #[test]
    fn test_task_state() {
        let scheduler = TaskScheduler::new();
        let id = scheduler.create_task("test".to_string(), 5);
        scheduler.set_task_state(id, TaskState::Running);
        let task = scheduler.get_task(id).unwrap();
        assert_eq!(task.state, TaskState::Running);
    }
}
