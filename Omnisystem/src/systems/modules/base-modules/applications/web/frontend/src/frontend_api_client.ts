// PATHFINDER Frontend - API Client
// Service for communicating with backend microservices
// Week 4 Implementation

import axios, { AxiosInstance, AxiosError } from 'axios';

// ============================================================================
// API CONFIGURATION
// ============================================================================

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8000';
const API_TIMEOUT = import.meta.env.VITE_API_TIMEOUT_MS || '30000';

// ============================================================================
// TYPES
// ============================================================================

export interface User {
  id: string;
  email: string;
  email_verified: boolean;
  first_name: string;
  last_name: string;
  language_preference: string;
  is_teacher: boolean;
  created_at: string;
}

export interface Skill {
  id: string;
  code: string;
  name: string;
  description?: string;
  level: string;
  language: string;
  category: string;
  icon_url?: string;
  color_hex?: string;
  estimated_time_minutes: number;
  difficulty_level: number;
  is_published: boolean;
  prerequisites?: string[];
}

export interface Exercise {
  id: string;
  skill_id: string;
  type: string;
  title: string;
  description?: string;
  difficulty_delta: number;
  prompt?: string;
  correct_option?: number;
  options?: string[];
  explanation?: string;
  estimated_time_seconds: number;
  usage_count: number;
  average_success_rate: number;
  is_published: boolean;
  created_at: string;
}

export interface Lesson {
  id: string;
  skill_id: string;
  sequence: number;
  title: string;
  description?: string;
  learning_objectives?: string[];
  exercises?: Exercise[];
}

export interface LearnerSkillState {
  id: string;
  user_id: string;
  skill_id: string;
  p_know: number;
  strength: number;
  is_mastered: boolean;
  next_review_at?: string;
  attempt_count: number;
  correct_count: number;
}

export interface ExerciseAttemptResponse {
  skill_state: LearnerSkillState;
  next_review_at?: string;
  is_mastered: boolean;
  feedback: string;
}

export interface DailyMetrics {
  date: string;
  exercises_attempted: number;
  exercises_correct: number;
  correct_rate: number;
  time_spent_seconds: number;
  skills_reviewed: number;
  new_skills_mastered: number;
  is_streak_day: boolean;
  xp_earned: number;
}

export interface LearningCurveData {
  skill_id: string;
  skill_name: string;
  learning_curve: {
    date: string;
    p_know: number;
    strength: number;
    correct_rate: number;
    attempt_count: number;
  }[];
  mastery_progress: number;
  trend_direction: string;
}

export interface ProgressMetrics {
  total_skills: number;
  mastered_skills: number;
  developing_skills: number;
  mastery_percentage: number;
  average_strength: number;
  total_exercises_completed: number;
  average_accuracy: number;
  current_streak: number;
  longest_streak: number;
}

export interface AuthResponse {
  user_id: string;
  token: string;
  refresh_token: string;
  expires_in: number;
}

// ============================================================================
// API CLIENT CLASS
// ============================================================================

class PathfinderAPIClient {
  private axiosInstance: AxiosInstance;
  private token: string | null = null;

  constructor() {
    this.axiosInstance = axios.create({
      baseURL: API_BASE_URL,
      timeout: parseInt(API_TIMEOUT),
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Load token from localStorage
    this.token = localStorage.getItem('auth_token');

    // Add request interceptor for auth
    this.axiosInstance.interceptors.request.use((config) => {
      if (this.token) {
        config.headers.Authorization = `Bearer ${this.token}`;
      }
      return config;
    });

    // Add response interceptor for error handling
    this.axiosInstance.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.status === 401) {
          // Unauthorized - clear token and redirect to login
          this.logout();
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }

  // ========================================================================
  // AUTHENTICATION
  // ========================================================================

  async register(email: string, password: string, firstName: string, lastName?: string): Promise<AuthResponse> {
    const response = await this.axiosInstance.post<AuthResponse>('/v1/auth/register', {
      email,
      password,
      first_name: firstName,
      last_name: lastName || '',
    });

    this.setToken(response.data.token);
    return response.data;
  }

  async login(email: string, password: string): Promise<AuthResponse> {
    const response = await this.axiosInstance.post<AuthResponse>('/v1/auth/login', {
      email,
      password,
    });

    this.setToken(response.data.token);
    return response.data;
  }

  async logout(): Promise<void> {
    try {
      await this.axiosInstance.post('/v1/auth/logout');
    } catch (error) {
      // Ignore errors, logout anyway
    }

    this.clearToken();
  }

  async getProfile(): Promise<User> {
    const response = await this.axiosInstance.get<User>('/v1/users/me');
    return response.data;
  }

  async updateProfile(data: Partial<User>): Promise<void> {
    await this.axiosInstance.put('/v1/users/me', data);
  }

  async deleteAccount(): Promise<void> {
    await this.axiosInstance.delete('/v1/users/me');
    this.clearToken();
  }

  // ========================================================================
  // SKILLS & CONTENT
  // ========================================================================

  async getSkills(filters?: {
    language?: string;
    level?: string;
    category?: string;
  }): Promise<Skill[]> {
    const response = await this.axiosInstance.get<{ skills: Skill[] }>(
      '/v1/skills',
      { params: filters }
    );
    return response.data.skills;
  }

  async getSkill(skillId: string): Promise<Skill> {
    const response = await this.axiosInstance.get<Skill>(`/v1/skills/${skillId}`);
    return response.data;
  }

  async getExercisesForSkill(skillId: string): Promise<Exercise[]> {
    const response = await this.axiosInstance.get<{ exercises: Exercise[] }>(
      `/v1/skills/${skillId}/exercises`
    );
    return response.data.exercises;
  }

  async getExercise(exerciseId: string): Promise<Exercise> {
    const response = await this.axiosInstance.get<Exercise>(
      `/v1/exercises/${exerciseId}`
    );
    return response.data;
  }

  async getLessonsForSkill(skillId: string): Promise<Lesson[]> {
    const response = await this.axiosInstance.get<{ lessons: Lesson[] }>(
      `/v1/skills/${skillId}/lessons`
    );
    return response.data.lessons;
  }

  async getLesson(lessonId: string): Promise<Lesson> {
    const response = await this.axiosInstance.get<Lesson>(
      `/v1/lessons/${lessonId}`
    );
    return response.data;
  }

  async searchContent(query: string): Promise<{
    skills: Skill[];
    exercises: Exercise[];
  }> {
    const response = await this.axiosInstance.get('/v1/search', {
      params: { q: query },
    });
    return response.data;
  }

  // ========================================================================
  // LEARNING & EXERCISES
  // ========================================================================

  async recordExerciseAttempt(
    userId: string,
    exerciseId: string,
    skillId: string,
    wasCorrect: boolean,
    response?: string,
    responseTimeSeconds?: number
  ): Promise<ExerciseAttemptResponse> {
    const attemptResponse = await this.axiosInstance.post<ExerciseAttemptResponse>(
      `/v1/learners/${userId}/exercises/${exerciseId}/attempt`,
      {
        exercise_id: exerciseId,
        skill_id: skillId,
        was_correct: wasCorrect,
        response,
        response_time_seconds: responseTimeSeconds,
      }
    );
    return attemptResponse.data;
  }

  async getLearnerSkills(userId: string): Promise<LearnerSkillState[]> {
    const response = await this.axiosInstance.get<{ skills: LearnerSkillState[] }>(
      `/v1/learners/${userId}/skills`
    );
    return response.data.skills;
  }

  async getNextSkillsToReview(userId: string, limit?: number): Promise<any> {
    const response = await this.axiosInstance.get(
      `/v1/learners/${userId}/next-skills`,
      { params: { limit } }
    );
    return response.data;
  }

  async getProgress(userId: string): Promise<ProgressMetrics> {
    const response = await this.axiosInstance.get<ProgressMetrics>(
      `/v1/learners/${userId}/progress`
    );
    return response.data;
  }

  // ========================================================================
  // ANALYTICS
  // ========================================================================

  async getDailyMetrics(userId: string, date?: string): Promise<DailyMetrics> {
    const response = await this.axiosInstance.get<DailyMetrics>(
      `/v1/learners/${userId}/daily-metrics`,
      { params: { date } }
    );
    return response.data;
  }

  async getMonthlyMetrics(userId: string, month?: string): Promise<any> {
    const response = await this.axiosInstance.get(
      `/v1/learners/${userId}/monthly-metrics`,
      { params: { month } }
    );
    return response.data;
  }

  async getLearningCurve(userId: string, skillId: string): Promise<LearningCurveData> {
    const response = await this.axiosInstance.get<LearningCurveData>(
      `/v1/learners/${userId}/skills/${skillId}/learning-curve`
    );
    return response.data;
  }

  // ========================================================================
  // TOKEN MANAGEMENT
  // ========================================================================

  private setToken(token: string): void {
    this.token = token;
    localStorage.setItem('auth_token', token);
  }

  private clearToken(): void {
    this.token = null;
    localStorage.removeItem('auth_token');
  }

  getToken(): string | null {
    return this.token;
  }

  isAuthenticated(): boolean {
    return this.token !== null;
  }

  // ========================================================================
  // ERROR HANDLING
  // ========================================================================

  getErrorMessage(error: unknown): string {
    if (axios.isAxiosError(error)) {
      if (error.response?.data?.message) {
        return error.response.data.message;
      }
      if (error.response?.statusText) {
        return error.response.statusText;
      }
    }
    return 'An error occurred';
  }
}

// Export singleton instance
export const apiClient = new PathfinderAPIClient();

export default apiClient;
