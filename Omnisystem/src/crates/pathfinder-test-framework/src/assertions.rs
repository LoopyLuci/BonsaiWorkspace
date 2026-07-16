// Custom assertions for testing
use serde_json::Value;

pub trait PathfinderAssertions {
    fn assert_user_valid(&self);
    fn assert_skill_valid(&self);
    fn assert_exercise_valid(&self);
    fn assert_progress_valid(&self);
}

impl PathfinderAssertions for Value {
    fn assert_user_valid(&self) {
        assert!(self.get("id").is_some(), "User must have id");
        assert!(self.get("email").is_some(), "User must have email");
        assert!(self.get("role").is_some(), "User must have role");
    }

    fn assert_skill_valid(&self) {
        assert!(self.get("id").is_some(), "Skill must have id");
        assert!(self.get("name").is_some(), "Skill must have name");
        assert!(self.get("difficulty").is_some(), "Skill must have difficulty");
    }

    fn assert_exercise_valid(&self) {
        assert!(self.get("id").is_some(), "Exercise must have id");
        assert!(self.get("skill_id").is_some(), "Exercise must have skill_id");
        assert!(self.get("title").is_some(), "Exercise must have title");
    }

    fn assert_progress_valid(&self) {
        assert!(self.get("user_id").is_some(), "Progress must have user_id");
        assert!(self.get("skill_id").is_some(), "Progress must have skill_id");
        assert!(self.get("p_know").is_some(), "Progress must have p_know");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_user_assertions() {
        let user = json!({
            "id": "user_1",
            "email": "test@example.com",
            "role": "student"
        });
        user.assert_user_valid();
    }
}
