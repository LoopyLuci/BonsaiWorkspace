// PATHFINDER Frontend - useInsights Hook
// Learning analytics, recommendations, and study planning

import { useState, useCallback, useEffect } from 'react';
import apiClient from '../api-client';

interface LearningAnalytics {
  user_id: string;
  total_skills: number;
  mastered_skills: number;
  struggle_skills: number;
  average_mastery: number;
  average_accuracy: number;
  total_exercises: number;
  total_time_spent_minutes: number;
  average_time_per_exercise: number;
  longest_streak: number;
  current_streak: number;
  last_activity_time: string;
}

interface Recommendation {
  id: string;
  user_id: string;
  type: string;
  skill_name: string;
  reason: string;
  priority: string;
  action_text: string;
  time_needed_minutes: number;
  due_date: string;
  created_at: string;
}

interface StudySession {
  id: string;
  user_id: string;
  skill_name: string;
  duration: number;
  difficulty: string;
  scheduled_for: string;
  status: string;
  completed_at?: string;
  created_at: string;
}

interface LearningStyle {
  user_id: string;
  visual_percent: number;
  auditory_percent: number;
  kinesthetic_percent: number;
  reading_percent: number;
  dominant_style: string;
  updated_at: string;
}

interface PerformanceMetric {
  skill_name: string;
  mastery_percent: number;
  accuracy_percent: number;
  exercises_attempted: number;
  exercises_correct: number;
  trend: string;
  predicted_mastery_percent: number;
}

interface UseInsightsReturn {
  // Analytics
  analytics: LearningAnalytics | null;
  isLoadingAnalytics: boolean;
  fetchAnalytics: () => Promise<void>;

  // Recommendations
  recommendations: Recommendation[];
  isLoadingRecommendations: boolean;
  fetchRecommendations: (limit?: number) => Promise<void>;
  generateRecommendations: () => Promise<void>;

  // Study Plan
  sessions: StudySession[];
  isLoadingSessions: boolean;
  fetchSessions: (status?: string) => Promise<void>;
  createSession: (session: Omit<StudySession, 'id' | 'user_id' | 'created_at'>) => Promise<string>;
  updateSession: (sessionId: string, status: string) => Promise<void>;

  // Learning Style
  learningStyle: LearningStyle | null;
  isLoadingStyle: boolean;
  fetchLearningStyle: () => Promise<void>;

  // Performance
  performance: PerformanceMetric[];
  isLoadingPerformance: boolean;
  fetchPerformance: () => Promise<void>;

  // Error
  error: string | null;
}

export const useInsights = (userID: string): UseInsightsReturn => {
  const [analytics, setAnalytics] = useState<LearningAnalytics | null>(null);
  const [isLoadingAnalytics, setIsLoadingAnalytics] = useState(false);

  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [isLoadingRecommendations, setIsLoadingRecommendations] = useState(false);

  const [sessions, setSessions] = useState<StudySession[]>([]);
  const [isLoadingSessions, setIsLoadingSessions] = useState(false);

  const [learningStyle, setLearningStyle] = useState<LearningStyle | null>(null);
  const [isLoadingStyle, setIsLoadingStyle] = useState(false);

  const [performance, setPerformance] = useState<PerformanceMetric[]>([]);
  const [isLoadingPerformance, setIsLoadingPerformance] = useState(false);

  const [error, setError] = useState<string | null>(null);

  // Fetch analytics
  const fetchAnalytics = useCallback(async () => {
    if (!userID) return;

    try {
      setIsLoadingAnalytics(true);
      setError(null);

      const response = await apiClient.get('/v1/insights/analytics', {
        headers: { 'X-User-ID': userID },
      });

      setAnalytics(response.data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load analytics';
      setError(message);
      console.error('Error fetching analytics:', err);
    } finally {
      setIsLoadingAnalytics(false);
    }
  }, [userID]);

  // Fetch recommendations
  const fetchRecommendations = useCallback(
    async (limit = 10) => {
      if (!userID) return;

      try {
        setIsLoadingRecommendations(true);
        setError(null);

        const response = await apiClient.get(`/v1/insights/recommendations?limit=${limit}`, {
          headers: { 'X-User-ID': userID },
        });

        setRecommendations(response.data.recommendations || []);
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Failed to load recommendations';
        setError(message);
        console.error('Error fetching recommendations:', err);
      } finally {
        setIsLoadingRecommendations(false);
      }
    },
    [userID]
  );

  // Generate recommendations
  const generateRecommendations = useCallback(async () => {
    try {
      setIsLoadingRecommendations(true);

      await apiClient.post(
        '/v1/insights/recommendations/generate',
        { user_id: userID },
        { headers: { 'X-User-ID': userID } }
      );

      // Fetch updated recommendations
      await fetchRecommendations();
    } catch (err) {
      console.error('Error generating recommendations:', err);
      throw err;
    } finally {
      setIsLoadingRecommendations(false);
    }
  }, [userID, fetchRecommendations]);

  // Fetch study sessions
  const fetchSessions = useCallback(
    async (status?: string) => {
      if (!userID) return;

      try {
        setIsLoadingSessions(true);
        setError(null);

        const url = status ? `/v1/insights/study-plan?status=${status}` : '/v1/insights/study-plan';
        const response = await apiClient.get(url, {
          headers: { 'X-User-ID': userID },
        });

        setSessions(response.data.sessions || []);
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Failed to load sessions';
        setError(message);
        console.error('Error fetching sessions:', err);
      } finally {
        setIsLoadingSessions(false);
      }
    },
    [userID]
  );

  // Create session
  const createSession = useCallback(
    async (session: Omit<StudySession, 'id' | 'user_id' | 'created_at'>): Promise<string> => {
      try {
        const response = await apiClient.post('/v1/insights/study-plan', session, {
          headers: { 'X-User-ID': userID },
        });
        return response.data.id;
      } catch (err) {
        console.error('Error creating session:', err);
        throw err;
      }
    },
    [userID]
  );

  // Update session
  const updateSession = useCallback(
    async (sessionId: string, status: string) => {
      try {
        await apiClient.put(
          `/v1/insights/study-plan/update?id=${sessionId}`,
          { status },
          { headers: { 'X-User-ID': userID } }
        );
      } catch (err) {
        console.error('Error updating session:', err);
        throw err;
      }
    },
    [userID]
  );

  // Fetch learning style
  const fetchLearningStyle = useCallback(async () => {
    if (!userID) return;

    try {
      setIsLoadingStyle(true);
      setError(null);

      const response = await apiClient.get('/v1/insights/learning-style', {
        headers: { 'X-User-ID': userID },
      });

      setLearningStyle(response.data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load learning style';
      setError(message);
      console.error('Error fetching learning style:', err);
    } finally {
      setIsLoadingStyle(false);
    }
  }, [userID]);

  // Fetch performance
  const fetchPerformance = useCallback(async () => {
    if (!userID) return;

    try {
      setIsLoadingPerformance(true);
      setError(null);

      const response = await apiClient.get('/v1/insights/performance', {
        headers: { 'X-User-ID': userID },
      });

      setPerformance(response.data.metrics || []);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load performance';
      setError(message);
      console.error('Error fetching performance:', err);
    } finally {
      setIsLoadingPerformance(false);
    }
  }, [userID]);

  // Initial load
  useEffect(() => {
    if (userID) {
      fetchAnalytics();
      fetchRecommendations();
      fetchSessions();
      fetchLearningStyle();
      fetchPerformance();
    }
  }, [userID, fetchAnalytics, fetchRecommendations, fetchSessions, fetchLearningStyle, fetchPerformance]);

  return {
    analytics,
    isLoadingAnalytics,
    fetchAnalytics,

    recommendations,
    isLoadingRecommendations,
    fetchRecommendations,
    generateRecommendations,

    sessions,
    isLoadingSessions,
    fetchSessions,
    createSession,
    updateSession,

    learningStyle,
    isLoadingStyle,
    fetchLearningStyle,

    performance,
    isLoadingPerformance,
    fetchPerformance,

    error,
  };
};

export default useInsights;
