use pathfinder_core::*;

#[test]
fn test_full_learning_workflow() {
    let user_mgr = user::UserManager::new();
    let course_lib = course::CourseLibrary::new();
    let progress = progress::ProgressTracker::new();
    let achievements = achievement::AchievementManager::new();
    
    let user = user::User {
        id: "u1".to_string(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        enrollment_date: 1000,
        completed_courses: 0,
    };
    user_mgr.register_user(user).unwrap();
    
    let course = course::Course {
        id: "c1".to_string(),
        title: "Rust 101".to_string(),
        description: "Learn Rust".to_string(),
        duration_weeks: 8,
        level: course::CourseLevel::Beginner,
        enrolled_count: 1,
    };
    course_lib.add_course(course).unwrap();
    
    let enrollment = progress::EnrollmentProgress {
        user_id: "u1".to_string(),
        course_id: "c1".to_string(),
        completion_percent: 0.0,
        lessons_completed: 0,
        total_lessons: 10,
    };
    progress.enroll(enrollment).unwrap();
    
    progress.update_progress("u1", "c1", 10, 10).unwrap();
    
    let achievement = achievement::Achievement {
        id: "complete_c1".to_string(),
        name: "Complete Rust 101".to_string(),
        description: "Finished the course".to_string(),
        badge_url: "badge.png".to_string(),
    };
    achievements.define_achievement(achievement).unwrap();
    achievements.unlock_achievement("u1".to_string(), "complete_c1".to_string()).unwrap();
    
    let user_courses = progress.get_user_courses("u1");
    assert_eq!(user_courses[0].completion_percent, 100.0);
}
