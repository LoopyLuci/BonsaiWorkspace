use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ProgressTracker {
    progress: Arc<DashMap<String, EnrollmentProgress>>,
}

#[derive(Debug, Clone)]
pub struct EnrollmentProgress {
    pub user_id: String,
    pub course_id: String,
    pub completion_percent: f32,
    pub lessons_completed: u32,
    pub total_lessons: u32,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(DashMap::new()),
        }
    }

    pub fn enroll(&self, progress: EnrollmentProgress) -> Result<()> {
        let key = format!("{}::{}", progress.user_id, progress.course_id);
        self.progress.insert(key, progress);
        tracing::info!("User enrolled");
        Ok(())
    }

    pub fn update_progress(&self, user_id: &str, course_id: &str, completed: u32, total: u32) -> Result<()> {
        let key = format!("{}::{}", user_id, course_id);
        if let Some(mut entry) = self.progress.get_mut(&key) {
            entry.lessons_completed = completed;
            entry.completion_percent = (completed as f32 / total as f32) * 100.0;
            Ok(())
        } else {
            Err(crate::PathfinderError::ProgressError("Enrollment not found".to_string()))
        }
    }

    pub fn get_user_courses(&self, user_id: &str) -> Vec<EnrollmentProgress> {
        self.progress
            .iter()
            .filter(|e| e.value().user_id == user_id)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn enrollment_count(&self) -> usize {
        self.progress.len()
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracking() {
        let tracker = ProgressTracker::new();
        let progress = EnrollmentProgress {
            user_id: "u1".to_string(),
            course_id: "c1".to_string(),
            completion_percent: 0.0,
            lessons_completed: 0,
            total_lessons: 10,
        };
        assert!(tracker.enroll(progress).is_ok());
        assert!(tracker.update_progress("u1", "c1", 5, 10).is_ok());
    }
}
