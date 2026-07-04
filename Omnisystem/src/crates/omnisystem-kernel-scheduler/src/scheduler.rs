use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub priority: u8,
    pub ready: bool,
}

pub struct KernelScheduler {
    tasks: Arc<DashMap<String, Task>>,
}

impl KernelScheduler {
    pub fn new() -> Self {
        Self { tasks: Arc::new(DashMap::new()) }
    }
    
    pub fn create_task(&self, id: String, priority: u8) -> String {
        let task = Task { id: id.clone(), priority, ready: true };
        self.tasks.insert(id.clone(), task);
        id
    }
    
    pub fn get_task(&self, id: &str) -> Option<Task> {
        self.tasks.get(id).map(|t| t.clone())
    }
    
    pub fn mark_ready(&self, id: &str, ready: bool) -> bool {
        if let Some(mut task) = self.tasks.get_mut(id) {
            task.ready = ready;
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
    fn test_create_task() {
        let sched = KernelScheduler::new();
        let id = sched.create_task("task1".to_string(), 5);
        assert_eq!(sched.task_count(), 1);
    }
    
    #[test]
    fn test_mark_ready() {
        let sched = KernelScheduler::new();
        sched.create_task("task1".to_string(), 5);
        assert!(sched.mark_ready("task1", false));
    }
}
