// PATHFINDER Data Models
// Core types used throughout PATHFINDER

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// User account (student, teacher, parent, or admin)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub role: UserRole,
    pub date_of_birth: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    Student,
    Teacher,
    Parent,
    Admin,
}

/// Skill in the curriculum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub grade: i32,
    pub subject: String,
    pub difficulty: SkillDifficulty,
    pub prerequisites: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Skill difficulty level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillDifficulty {
    Easy,
    Medium,
    Hard,
}

/// Exercise for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub id: String,
    pub skill_id: String,
    pub question: String,
    pub exercise_type: ExerciseType,
    pub difficulty: SkillDifficulty,
    pub estimated_time_seconds: Option<i32>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Exercise type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExerciseType {
    MultipleChoice,
    ShortAnswer,
    Essay,
}

/// Exercise options (for multiple choice)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseOption {
    pub id: String,
    pub exercise_id: String,
    pub option_text: String,
    pub position: i32,
    pub is_correct: bool,
}

/// Student's exercise attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseAttempt {
    pub id: String,
    pub user_id: String,
    pub exercise_id: String,
    pub skill_id: String,
    pub selected_option_id: Option<String>,
    pub is_correct: bool,
    pub time_taken_seconds: i32,
    pub attempted_at: DateTime<Utc>,
}

/// Student's progress on a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProgress {
    pub id: String,
    pub user_id: String,
    pub skill_id: String,
    pub mastery_percent: f64,
    pub exercises_attempted: i32,
    pub exercises_correct: i32,
    pub p_know: f64,  // Probability of knowing (BKT)
    pub confidence: f64,
    pub last_updated: DateTime<Utc>,
}

/// Classroom (teacher's class)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classroom {
    pub id: String,
    pub teacher_id: String,
    pub name: String,
    pub grade: Option<i32>,
    pub subject: Option<String>,
    pub invite_code: String,
    pub max_students: i32,
    pub created_at: DateTime<Utc>,
}

/// Classroom membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassroomMembership {
    pub id: String,
    pub classroom_id: String,
    pub student_id: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

/// Parent-child relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentChildRelationship {
    pub id: String,
    pub parent_id: String,
    pub child_id: String,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Achievement badge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub rarity: BadgeRarity,
    pub points: i32,
    pub icon_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Badge rarity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BadgeRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// User achievement unlock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAchievement {
    pub id: String,
    pub user_id: String,
    pub achievement_id: String,
    pub unlocked_at: DateTime<Utc>,
}

/// Gamification stats for user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamificationStats {
    pub id: String,
    pub user_id: String,
    pub total_points: i32,
    pub level: i32,
    pub xp_in_level: i32,
    pub total_xp: i32,
    pub streak_days: i32,
    pub longest_streak: i32,
    pub last_activity_date: Option<String>,
}

/// Student goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub user_id: String,
    pub goal_type: GoalType,
    pub skill_id: Option<String>,
    pub deadline: Option<DateTime<Utc>>,
    pub status: GoalStatus,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
}

/// Goal type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    SkillsMastery,
    AccuracyTarget,
    StreakTarget,
}

/// Goal status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Expired,
    Abandoned,
}

/// Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    MasteryMilestone,
    Alert,
    DailySummary,
    WeeklyReport,
    Achievement,
}

/// Learning insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub id: String,
    pub user_id: String,
    pub total_skills: i32,
    pub skills_mastered: i32,
    pub average_mastery: f64,
    pub average_accuracy: f64,
    pub total_time_spent_seconds: i32,
    pub learning_style: Option<String>,
    pub generated_at: DateTime<Utc>,
}
