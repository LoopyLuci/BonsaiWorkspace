use crate::{Task, TaskState, Result, RuntimeError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct TaskExecutor {
    tasks: Arc<DashMap<String, Task>>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
        }
    }

    pub async fn execute(&self, mut task: Task) -> Result<()> {
        task.state = TaskState::Running;
        task.started_at = Some(std::time::SystemTime::now());

        self.tasks.insert(task.id.clone(), task.clone());
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        if let Some(mut entry) = self.tasks.get_mut(&task.id) {
            entry.state = TaskState::Completed;
            entry.completed_at = Some(std::time::SystemTime::now());
        }

        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Option<Task> {
        self.tasks.get(id).map(|ref_| ref_.clone())
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
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
    use crate::Priority;

    #[tokio::test]
    async fn test_execute_task() {
        let executor = TaskExecutor::new();
        let task = Task::new("t1".to_string(), Priority::Normal);
        assert!(executor.execute(task).await.is_ok());
    }

    #[test]
    fn test_executor_new() {
        let executor = TaskExecutor::new();
        assert_eq!(executor.task_count(), 0);
    }
}
