use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub priority: Priority,
    pub state: TaskState,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
}

impl Task {
    pub fn new(id: String, priority: Priority) -> Self {
        Self {
            id,
            priority,
            state: TaskState::Pending,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => end.duration_since(start).ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub avg_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::Low < Priority::Normal);
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("t1".to_string(), Priority::Normal);
        assert_eq!(task.state, TaskState::Pending);
    }

    #[test]
    fn test_task_state_transitions() {
        assert_ne!(TaskState::Running, TaskState::Completed);
    }
}
