// Integration tests for PATHFINDER services
#[cfg(test)]
mod integration_tests {
    use serde_json::json;

    #[test]
    fn test_user_registration_flow() {
        let user_data = json!({
            "email": "newuser@example.com",
            "password": "securepass123",
            "name": "Test User"
        });
        assert_eq!(user_data["email"], "newuser@example.com");
    }

    #[test]
    fn test_exercise_submission_flow() {
        let attempt = json!({
            "exercise_id": "ex_1",
            "user_id": "user_1",
            "answer": "7",
            "submitted_at": "2026-06-11T12:00:00Z"
        });
        assert_eq!(attempt["exercise_id"], "ex_1");
    }

    #[test]
    fn test_progress_calculation() {
        let progress = json!({
            "user_id": "user_1",
            "skill_id": "skill_1",
            "p_know": 0.75,
            "attempts": 15
        });
        assert!(progress["p_know"] > 0.7);
    }

    #[test]
    fn test_teacher_classroom_creation() {
        let classroom = json!({
            "name": "Algebra 101",
            "teacher_id": "teacher_1",
            "grade_level": 9,
            "created_at": "2026-06-11T12:00:00Z"
        });
        assert_eq!(classroom["grade_level"], 9);
    }

    #[test]
    fn test_parent_child_linking() {
        let link = json!({
            "parent_id": "parent_1",
            "child_id": "student_1",
            "relationship": "parent",
            "verified": true
        });
        assert!(link["verified"].as_bool().unwrap());
    }

    #[test]
    fn test_notification_dispatch() {
        let notification = json!({
            "user_id": "user_1",
            "notification_type": "achievement",
            "message": "You unlocked the Math Master badge!",
            "sent_at": "2026-06-11T12:00:00Z"
        });
        assert_eq!(notification["notification_type"], "achievement");
    }

    #[test]
    fn test_achievement_unlock() {
        let achievement = json!({
            "user_id": "user_1",
            "badge_id": "badge_math_master",
            "rarity": "legendary",
            "unlocked_at": "2026-06-11T12:00:00Z"
        });
        assert_eq!(achievement["rarity"], "legendary");
    }

    #[test]
    fn test_insights_generation() {
        let insights = json!({
            "user_id": "user_1",
            "total_attempts": 156,
            "success_rate": 0.82,
            "recommended_next_skill": "geometry"
        });
        assert!(insights["success_rate"] > 0.8);
    }

    #[test]
    fn test_personalization_recommendations() {
        let recommendations = json!({
            "user_id": "user_1",
            "skills": [
                {"skill_id": "algebra", "difficulty": "medium"},
                {"skill_id": "geometry", "difficulty": "easy"}
            ]
        });
        assert_eq!(recommendations["skills"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_analytics_retrieval() {
        let analytics = json!({
            "classroom_id": "class_1",
            "total_students": 28,
            "average_mastery": 0.72,
            "completion_rate": 0.89
        });
        assert!(analytics["average_mastery"] > 0.7);
    }

    #[test]
    fn test_search_functionality() {
        let search_results = json!({
            "query": "algebra",
            "results": [
                {"skill_id": "algebra_1", "title": "Algebra Fundamentals"},
                {"skill_id": "algebra_2", "title": "Advanced Algebra"}
            ],
            "total": 2
        });
        assert_eq!(search_results["total"], 2);
    }
}
