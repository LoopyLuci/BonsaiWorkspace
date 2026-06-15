-- PATHFINDER Database Migration Framework
-- Production PostgreSQL schema with all tables

-- ==========================================
-- MIGRATION 001: Initial Schema Setup
-- ==========================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- ==========================================
-- USERS TABLE
-- ==========================================

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  email VARCHAR(255) NOT NULL UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  name VARCHAR(255) NOT NULL,
  role VARCHAR(50) NOT NULL CHECK (role IN ('student', 'teacher', 'parent', 'admin')),
  date_of_birth DATE,
  avatar_url VARCHAR(512),
  timezone VARCHAR(50) DEFAULT 'UTC',
  email_verified BOOLEAN DEFAULT FALSE,
  email_verified_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  deleted_at TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_created_at ON users(created_at DESC);

-- ==========================================
-- SESSIONS TABLE
-- ==========================================

CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash VARCHAR(255) NOT NULL UNIQUE,
  ip_address VARCHAR(45),
  user_agent VARCHAR(512),
  expires_at TIMESTAMP NOT NULL,
  last_activity TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at DESC);

-- ==========================================
-- SKILLS TABLE
-- ==========================================

CREATE TABLE skills (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  name VARCHAR(255) NOT NULL,
  description TEXT,
  grade INTEGER CHECK (grade >= 1 AND grade <= 12),
  subject VARCHAR(100) NOT NULL,
  difficulty VARCHAR(50) CHECK (difficulty IN ('easy', 'medium', 'hard')),
  standard_code VARCHAR(100),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_skills_grade ON skills(grade);
CREATE INDEX idx_skills_subject ON skills(subject);
CREATE INDEX idx_skills_difficulty ON skills(difficulty);
CREATE INDEX idx_skills_name_trgm ON skills USING GIN (name gin_trgm_ops);

-- ==========================================
-- SKILL PREREQUISITES TABLE
-- ==========================================

CREATE TABLE skill_prerequisites (
  skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  prerequisite_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (skill_id, prerequisite_id),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_skill_prerequisites_prereq ON skill_prerequisites(prerequisite_id);

-- ==========================================
-- EXERCISES TABLE
-- ==========================================

CREATE TABLE exercises (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  question TEXT NOT NULL,
  type VARCHAR(50) NOT NULL CHECK (type IN ('multiple_choice', 'short_answer', 'essay')),
  difficulty VARCHAR(50) CHECK (difficulty IN ('easy', 'medium', 'hard')),
  estimated_time_seconds INTEGER,
  created_by UUID REFERENCES users(id),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_exercises_skill_id ON exercises(skill_id);
CREATE INDEX idx_exercises_difficulty ON exercises(difficulty);
CREATE INDEX idx_exercises_created_at ON exercises(created_at DESC);

-- ==========================================
-- EXERCISE OPTIONS TABLE (for multiple choice)
-- ==========================================

CREATE TABLE exercise_options (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  exercise_id UUID NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
  option_text TEXT NOT NULL,
  position INTEGER NOT NULL,
  is_correct BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_exercise_options_exercise_id ON exercise_options(exercise_id);

-- ==========================================
-- EXERCISE ATTEMPTS TABLE
-- ==========================================

CREATE TABLE exercise_attempts (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  exercise_id UUID NOT NULL REFERENCES exercises(id),
  skill_id UUID NOT NULL REFERENCES skills(id),
  selected_option_id UUID REFERENCES exercise_options(id),
  is_correct BOOLEAN NOT NULL,
  time_taken_seconds INTEGER,
  attempted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  synced_at TIMESTAMP
);

CREATE INDEX idx_exercise_attempts_user_id ON exercise_attempts(user_id, attempted_at DESC);
CREATE INDEX idx_exercise_attempts_exercise_id ON exercise_attempts(exercise_id);
CREATE INDEX idx_exercise_attempts_skill_id ON exercise_attempts(skill_id);
CREATE INDEX idx_exercise_attempts_attempted_at ON exercise_attempts(attempted_at DESC);
CREATE INDEX idx_exercise_attempts_synced_at ON exercise_attempts(synced_at);

-- ==========================================
-- SKILL PROGRESS TABLE
-- ==========================================

CREATE TABLE skill_progress (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  mastery_percent NUMERIC(5, 2) DEFAULT 0.0 CHECK (mastery_percent >= 0 AND mastery_percent <= 100),
  exercises_attempted INTEGER DEFAULT 0,
  exercises_correct INTEGER DEFAULT 0,
  p_know NUMERIC(4, 3) DEFAULT 0.0,
  confidence NUMERIC(4, 3) DEFAULT 0.0,
  last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, skill_id)
);

CREATE INDEX idx_skill_progress_user_id ON skill_progress(user_id);
CREATE INDEX idx_skill_progress_mastery ON skill_progress(mastery_percent DESC);
CREATE INDEX idx_skill_progress_last_updated ON skill_progress(last_updated DESC);

-- ==========================================
-- CLASSROOMS TABLE
-- ==========================================

CREATE TABLE classrooms (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  teacher_id UUID NOT NULL REFERENCES users(id),
  name VARCHAR(255) NOT NULL,
  grade INTEGER,
  subject VARCHAR(100),
  invite_code VARCHAR(20) UNIQUE,
  max_students INTEGER DEFAULT 30,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  archived_at TIMESTAMP
);

CREATE INDEX idx_classrooms_teacher_id ON classrooms(teacher_id);
CREATE INDEX idx_classrooms_invite_code ON classrooms(invite_code);
CREATE INDEX idx_classrooms_created_at ON classrooms(created_at DESC);

-- ==========================================
-- CLASSROOM MEMBERSHIPS TABLE
-- ==========================================

CREATE TABLE classroom_memberships (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  classroom_id UUID NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
  student_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  left_at TIMESTAMP,
  UNIQUE(classroom_id, student_id)
);

CREATE INDEX idx_classroom_memberships_classroom_id ON classroom_memberships(classroom_id);
CREATE INDEX idx_classroom_memberships_student_id ON classroom_memberships(student_id);

-- ==========================================
-- PARENT-CHILD RELATIONSHIPS TABLE
-- ==========================================

CREATE TABLE parent_child_relationships (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  parent_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  child_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  verified BOOLEAN DEFAULT FALSE,
  verification_code VARCHAR(20),
  verified_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(parent_id, child_id)
);

CREATE INDEX idx_parent_child_parent_id ON parent_child_relationships(parent_id);
CREATE INDEX idx_parent_child_child_id ON parent_child_relationships(child_id);

-- ==========================================
-- NOTIFICATIONS TABLE
-- ==========================================

CREATE TABLE notifications (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  type VARCHAR(100) NOT NULL,
  title VARCHAR(255),
  message TEXT,
  data JSONB,
  read BOOLEAN DEFAULT FALSE,
  read_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id, created_at DESC);
CREATE INDEX idx_notifications_read ON notifications(read);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);

-- ==========================================
-- NOTIFICATION CHANNELS TABLE
-- ==========================================

CREATE TABLE notification_channels (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
  channel VARCHAR(50) NOT NULL CHECK (channel IN ('email', 'push', 'sms')),
  sent BOOLEAN DEFAULT FALSE,
  sent_at TIMESTAMP,
  error_message TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notification_channels_notification_id ON notification_channels(notification_id);

-- ==========================================
-- NOTIFICATION PREFERENCES TABLE
-- ==========================================

CREATE TABLE notification_preferences (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  email_notifications JSONB,
  push_notifications JSONB,
  sms_notifications JSONB,
  email_frequency VARCHAR(50) DEFAULT 'immediate',
  quiet_hours_enabled BOOLEAN DEFAULT FALSE,
  quiet_hours_start TIME,
  quiet_hours_end TIME,
  timezone VARCHAR(50),
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notification_preferences_user_id ON notification_preferences(user_id);

-- ==========================================
-- ACHIEVEMENTS TABLE
-- ==========================================

CREATE TABLE achievements (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  name VARCHAR(255) NOT NULL UNIQUE,
  description TEXT,
  category VARCHAR(100) NOT NULL,
  rarity VARCHAR(50) NOT NULL CHECK (rarity IN ('common', 'uncommon', 'rare', 'epic', 'legendary')),
  points INTEGER DEFAULT 0,
  icon_url VARCHAR(512),
  requirement_type VARCHAR(100),
  requirement_value INTEGER,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_achievements_category ON achievements(category);
CREATE INDEX idx_achievements_rarity ON achievements(rarity);

-- ==========================================
-- USER ACHIEVEMENTS TABLE
-- ==========================================

CREATE TABLE user_achievements (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  achievement_id UUID NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
  unlocked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, achievement_id)
);

CREATE INDEX idx_user_achievements_user_id ON user_achievements(user_id);
CREATE INDEX idx_user_achievements_unlocked_at ON user_achievements(unlocked_at DESC);

-- ==========================================
-- GAMIFICATION STATS TABLE
-- ==========================================

CREATE TABLE gamification_stats (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  total_points INTEGER DEFAULT 0,
  level INTEGER DEFAULT 1,
  xp_in_level INTEGER DEFAULT 0,
  total_xp INTEGER DEFAULT 0,
  streak_days INTEGER DEFAULT 0,
  longest_streak INTEGER DEFAULT 0,
  last_activity_date DATE,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_gamification_stats_user_id ON gamification_stats(user_id);
CREATE INDEX idx_gamification_stats_total_points ON gamification_stats(total_points DESC);

-- ==========================================
-- GOALS TABLE
-- ==========================================

CREATE TABLE goals (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  type VARCHAR(100) NOT NULL,
  skill_id UUID REFERENCES skills(id),
  deadline TIMESTAMP,
  status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('active', 'completed', 'expired', 'abandoned')),
  progress NUMERIC(5, 2) DEFAULT 0,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  completed_at TIMESTAMP
);

CREATE INDEX idx_goals_user_id ON goals(user_id, status);
CREATE INDEX idx_goals_deadline ON goals(deadline);
CREATE INDEX idx_goals_status ON goals(status);

-- ==========================================
-- LEADERBOARD TABLE (materialized view source)
-- ==========================================

CREATE TABLE leaderboard_cache (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id),
  rank INTEGER,
  total_points INTEGER,
  level INTEGER,
  achievements_count INTEGER,
  mastery_percent NUMERIC(5, 2),
  time_range VARCHAR(50),
  calculated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, time_range)
);

CREATE INDEX idx_leaderboard_cache_rank ON leaderboard_cache(rank, time_range);
CREATE INDEX idx_leaderboard_cache_user_id ON leaderboard_cache(user_id);

-- ==========================================
-- LEARNING INSIGHTS TABLE
-- ==========================================

CREATE TABLE learning_insights (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  total_skills INTEGER DEFAULT 0,
  skills_mastered INTEGER DEFAULT 0,
  average_mastery NUMERIC(5, 2) DEFAULT 0,
  average_accuracy NUMERIC(4, 3) DEFAULT 0,
  total_time_spent_seconds INTEGER DEFAULT 0,
  learning_style VARCHAR(50),
  style_visual NUMERIC(4, 3),
  style_auditory NUMERIC(4, 3),
  style_kinesthetic NUMERIC(4, 3),
  style_reading NUMERIC(4, 3),
  generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_learning_insights_user_id ON learning_insights(user_id);
CREATE INDEX idx_learning_insights_updated_at ON learning_insights(updated_at DESC);

-- ==========================================
-- AUDIT LOG TABLE
-- ==========================================

CREATE TABLE audit_log (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID REFERENCES users(id),
  action VARCHAR(100) NOT NULL,
  resource_type VARCHAR(100),
  resource_id VARCHAR(255),
  old_values JSONB,
  new_values JSONB,
  ip_address VARCHAR(45),
  user_agent VARCHAR(512),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at DESC);

-- ==========================================
-- DATA EXPORT REQUESTS TABLE (GDPR)
-- ==========================================

CREATE TABLE data_export_requests (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  status VARCHAR(50) DEFAULT 'processing' CHECK (status IN ('processing', 'completed', 'failed', 'expired')),
  download_url VARCHAR(512),
  requested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  completed_at TIMESTAMP,
  expires_at TIMESTAMP
);

CREATE INDEX idx_data_export_requests_user_id ON data_export_requests(user_id);
CREATE INDEX idx_data_export_requests_status ON data_export_requests(status);

-- ==========================================
-- TRIGGERS FOR UPDATED_AT COLUMNS
-- ==========================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_skills_updated_at BEFORE UPDATE ON skills
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_exercises_updated_at BEFORE UPDATE ON exercises
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_classrooms_updated_at BEFORE UPDATE ON classrooms
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==========================================
-- PERFORMANCE INDEXES
-- ==========================================

CREATE INDEX idx_exercise_attempts_user_skill ON exercise_attempts(user_id, skill_id);
CREATE INDEX idx_skill_progress_user_skill ON skill_progress(user_id, skill_id);
CREATE INDEX idx_classroom_memberships_combined ON classroom_memberships(classroom_id, student_id);

-- ==========================================
-- PARTITIONING (optional for large tables)
-- ==========================================

-- Partition exercise_attempts by month for performance
-- CREATE TABLE exercise_attempts_2026_06 PARTITION OF exercise_attempts
--   FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

-- ==========================================
-- INITIAL DATA SEEDS
-- ==========================================

-- Insert standard achievement badges
INSERT INTO achievements (name, description, category, rarity, points, icon_url) VALUES
  ('First Steps', 'Complete your first exercise', 'milestone', 'common', 10, 'https://cdn.pathfinder.com/badges/first_steps.png'),
  ('Quick Learner', 'Complete 10 exercises in one day', 'milestone', 'uncommon', 50, 'https://cdn.pathfinder.com/badges/quick_learner.png'),
  ('Master', 'Achieve 85% mastery on a skill', 'skill', 'epic', 100, 'https://cdn.pathfinder.com/badges/master.png'),
  ('Unstoppable', 'Maintain a 30-day streak', 'streak', 'legendary', 500, 'https://cdn.pathfinder.com/badges/unstoppable.png'),
  ('Speed Demon', 'Complete an exercise in under 30 seconds', 'speed', 'rare', 75, 'https://cdn.pathfinder.com/badges/speed_demon.png');

-- ==========================================
-- MIGRATION STATUS TABLE
-- ==========================================

CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  name VARCHAR(255),
  applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO schema_migrations (version, name) VALUES (1, 'Initial schema setup');
