use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct CourseLibrary {
    courses: Arc<DashMap<String, Course>>,
}

#[derive(Debug, Clone)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub description: String,
    pub duration_weeks: u32,
    pub level: CourseLevel,
    pub enrolled_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl CourseLibrary {
    pub fn new() -> Self {
        Self {
            courses: Arc::new(DashMap::new()),
        }
    }

    pub fn add_course(&self, course: Course) -> Result<()> {
        self.courses.insert(course.id.clone(), course);
        tracing::info!("Course added");
        Ok(())
    }

    pub fn get_course(&self, id: &str) -> Result<Course> {
        self.courses
            .get(id)
            .map(|c| c.value().clone())
            .ok_or_else(|| crate::PathfinderError::CourseNotFound(id.to_string()))
    }

    pub fn get_courses_by_level(&self, level: CourseLevel) -> Vec<Course> {
        self.courses
            .iter()
            .filter(|c| c.value().level == level)
            .map(|c| c.value().clone())
            .collect()
    }

    pub fn course_count(&self) -> usize {
        self.courses.len()
    }
}

impl Default for CourseLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_course_library() {
        let library = CourseLibrary::new();
        let course = Course {
            id: "c1".to_string(),
            title: "Rust 101".to_string(),
            description: "Learn Rust".to_string(),
            duration_weeks: 8,
            level: CourseLevel::Beginner,
            enrolled_count: 0,
        };
        assert!(library.add_course(course).is_ok());
        assert_eq!(library.course_count(), 1);
    }

    #[test]
    fn test_filter_by_level() {
        let library = CourseLibrary::new();
        let course = Course {
            id: "c1".to_string(),
            title: "Advanced Rust".to_string(),
            description: "Advanced topics".to_string(),
            duration_weeks: 12,
            level: CourseLevel::Advanced,
            enrolled_count: 5,
        };
        library.add_course(course).unwrap();
        let advanced = library.get_courses_by_level(CourseLevel::Advanced);
        assert_eq!(advanced.len(), 1);
    }
}
