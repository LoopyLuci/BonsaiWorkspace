export interface User {
  id: string
  email: string
  name: string
  role: 'student' | 'teacher' | 'parent'
}

export interface Skill {
  id: string
  name: string
  difficulty: 'beginner' | 'intermediate' | 'advanced'
  prerequisites: string[]
}

export interface Exercise {
  id: string
  skill_id: string
  title: string
  problem_type: string
}

export interface SkillProgress {
  user_id: string
  skill_id: string
  p_know: number
  attempts: number
}

export interface Classroom {
  id: string
  name: string
  teacher_id: string
  grade_level: number
}

export interface Notification {
  id: string
  user_id: string
  message: string
  notification_type: string
}

export interface Achievement {
  id: string
  user_id: string
  badge_id: string
  rarity: string
}

export interface AuthResponse {
  user_id: string
  token: string
  email: string
}
