//! CLI demo for pathfinder-core: registers a user, adds a course,
//! tracks enrollment progress to completion, and unlocks an
//! achievement.

use pathfinder_core::{
    Achievement, AchievementManager, Course, CourseLevel, CourseLibrary, EnrollmentProgress,
    ProgressTracker, User, UserManager,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let users = UserManager::new();
    let courses = CourseLibrary::new();
    let progress = ProgressTracker::new();
    let achievements = AchievementManager::new();

    users.register_user(User {
        id: "u1".to_string(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        enrollment_date: 1000,
        completed_courses: 0,
    })?;

    courses.add_course(Course {
        id: "c1".to_string(),
        title: "Rust 101".to_string(),
        description: "Learn Rust".to_string(),
        duration_weeks: 8,
        level: CourseLevel::Beginner,
        enrolled_count: 1,
    })?;

    progress.enroll(EnrollmentProgress {
        user_id: "u1".to_string(),
        course_id: "c1".to_string(),
        completion_percent: 0.0,
        lessons_completed: 0,
        total_lessons: 10,
    })?;
    progress.update_progress("u1", "c1", 10, 10)?;

    achievements.define_achievement(Achievement {
        id: "complete_c1".to_string(),
        name: "Complete Rust 101".to_string(),
        description: "Finished the course".to_string(),
        badge_url: "badge.png".to_string(),
    })?;
    achievements.unlock_achievement("u1".to_string(), "complete_c1".to_string())?;

    let user_progress = progress.get_user_courses("u1");
    println!(
        "User u1: {} courses, {:.0}% complete on c1",
        user_progress.len(),
        user_progress[0].completion_percent
    );
    println!(
        "Achievements unlocked: {}",
        achievements.get_user_achievements("u1").len()
    );

    Ok(())
}
