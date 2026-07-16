// Test fixtures and builders
use serde_json::json;

pub struct UserFixture;
impl UserFixture {
    pub fn builder() -> UserBuilder {
        UserBuilder::default()
    }
}

pub struct UserBuilder {
    id: String,
    email: String,
    name: Option<String>,
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self {
            id: "test_user_1".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
        }
    }
}

impl UserBuilder {
    pub fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn email(mut self, email: String) -> Self {
        self.email = email;
        self
    }

    pub fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    pub fn build(self) -> serde_json::Value {
        json!({
            "id": self.id,
            "email": self.email,
            "name": self.name,
            "role": "student"
        })
    }
}

pub struct SkillFixture;
impl SkillFixture {
    pub fn math_fundamentals() -> serde_json::Value {
        json!({
            "id": "skill_math_1",
            "name": "Math Fundamentals",
            "description": "Basic arithmetic and algebra",
            "difficulty": "easy",
            "prerequisites": []
        })
    }

    pub fn advanced_algebra() -> serde_json::Value {
        json!({
            "id": "skill_math_2",
            "name": "Advanced Algebra",
            "description": "Polynomial and rational functions",
            "difficulty": "hard",
            "prerequisites": ["skill_math_1"]
        })
    }
}

pub struct ExerciseFixture;
impl ExerciseFixture {
    pub fn builder() -> ExerciseBuilder {
        ExerciseBuilder::default()
    }
}

pub struct ExerciseBuilder {
    id: String,
    skill_id: String,
    title: String,
    exercise_type: String,
}

impl Default for ExerciseBuilder {
    fn default() -> Self {
        Self {
            id: "exercise_1".to_string(),
            skill_id: "skill_math_1".to_string(),
            title: "Solve 2x + 3 = 7".to_string(),
            exercise_type: "multiple_choice".to_string(),
        }
    }
}

impl ExerciseBuilder {
    pub fn skill_id(mut self, skill_id: String) -> Self {
        self.skill_id = skill_id;
        self
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn build(self) -> serde_json::Value {
        json!({
            "id": self.id,
            "skill_id": self.skill_id,
            "title": self.title,
            "exercise_type": self.exercise_type
        })
    }
}
