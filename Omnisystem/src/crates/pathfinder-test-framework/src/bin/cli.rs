//! CLI that exercises the pathfinder test-support fixtures/mocks/assertions.

use pathfinder_test_framework::{
    ExerciseFixture, MockModuleRequest, MockResponse, PathfinderAssertions, SkillFixture,
    UserFixture,
};

fn main() {
    let user = UserFixture::builder()
        .id("demo_user".to_string())
        .email("demo@example.com".to_string())
        .build();
    user.assert_user_valid();
    println!("Built user fixture: {user}");

    let skill = SkillFixture::math_fundamentals();
    skill.assert_skill_valid();
    println!("Built skill fixture: {skill}");

    let exercise = ExerciseFixture::builder()
        .skill_id(skill["id"].as_str().unwrap().to_string())
        .title("Solve 3x - 5 = 10".to_string())
        .build();
    exercise.assert_exercise_valid();
    println!("Built exercise fixture: {exercise}");

    let request = MockModuleRequest::new("user:register").with_args(user.clone());
    println!(
        "Mock request {} -> operation {}",
        request.request_id, request.operation
    );

    let response = MockResponse::success(serde_json::json!({"id": "demo_user"}));
    println!("Mock response status: {}", response.status);
}
