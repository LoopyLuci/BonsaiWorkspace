/// Task Executor - High-level async task management

use async_trait::async_trait;
use std::sync::Arc;

pub type TaskId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct TaskResult<T> {
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub value: Option<T>,
    pub error: Option<String>,
}

#[async_trait]
pub trait AsyncTask: Send + Sync {
    async fn execute(&self) -> Result<String, String>;
    fn name(&self) -> &str;
}

/// Task executor
pub struct TaskExecutor {
    tasks: parking_lot::RwLock<std::collections::HashMap<TaskId, Arc<dyn AsyncTask>>>,
    next_task_id: parking_lot::RwLock<TaskId>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        TaskExecutor {
            tasks: parking_lot::RwLock::new(std::collections::HashMap::new()),
            next_task_id: parking_lot::RwLock::new(1),
        }
    }

    pub fn register_task(&self, task: Arc<dyn AsyncTask>) -> TaskId {
        let task_id = {
            let mut next_id = self.next_task_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        self.tasks.write().insert(task_id, task);
        task_id
    }

    pub fn get_task(&self, task_id: TaskId) -> Option<Arc<dyn AsyncTask>> {
        self.tasks.read().get(&task_id).cloned()
    }

    pub fn list_tasks(&self) -> Vec<TaskId> {
        self.tasks.read().keys().cloned().collect()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.read().len()
    }

    pub fn remove_task(&self, task_id: TaskId) -> bool {
        self.tasks.write().remove(&task_id).is_some()
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTask;

    #[async_trait]
    impl AsyncTask for TestTask {
        async fn execute(&self) -> Result<String, String> {
            Ok("test result".to_string())
        }

        fn name(&self) -> &str {
            "test_task"
        }
    }

    #[test]
    fn test_task_executor() {
        let executor = TaskExecutor::new();
        let task = Arc::new(TestTask);

        let task_id = executor.register_task(task);
        assert!(executor.get_task(task_id).is_some());
        assert_eq!(executor.task_count(), 1);
    }
}
