//! Async task abstractions

/// Represents an async task
#[derive(Debug, Clone)]
pub struct Task {
    id: u64,
}

impl Task {
    /// Create a new task with given ID
    pub fn new(id: u64) -> Self {
        Task { id }
    }

    /// Get task ID
    pub fn id(&self) -> u64 {
        self.id
    }
}
