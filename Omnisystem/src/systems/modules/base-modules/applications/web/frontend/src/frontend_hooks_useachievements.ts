// PATHFINDER Frontend - useAchievements Hook
// Achievements, goals, badges, and gamification management

import { useState, useCallback, useEffect } from 'react';
import apiClient from '../api-client';

interface Achievement {
  id: string;
  user_id: string;
  badge_id: string;
  badge_name: string;
  category: string;
  description: string;
  icon_url: string;
  unlocked_at: string;
  created_at: string;
}

interface Badge {
  id: string;
  name: string;
  category: string;
  description: string;
  icon_url: string;
  requirement: string;
  rarity: string;
  points: number;
  created_at: string;
}

interface Goal {
  id: string;
  user_id: string;
  title: string;
  description: string;
  type: string;
  target: number;
  current: number;
  deadline: string;
  status: string;
  completed_at?: string;
  created_at: string;
}

interface LeaderboardEntry {
  user_id: string;
  user_name: string;
  rank: number;
  points: number;
  achievements: number;
  mastery_percent: number;
  streak_days: number;
}

interface GamificationStats {
  user_id: string;
  total_points: number;
  achievements_count: number;
  badges_unlocked: number;
  level: number;
  next_level_xp: number;
  leaderboard_rank: number;
}

interface UseAchievementsReturn {
  // Achievements
  achievements: Achievement[];
  isLoadingAchievements: boolean;
  fetchAchievements: () => Promise<void>;

  // Badges
  badges: Badge[];
  isLoadingBadges: boolean;
  fetchBadges: (category?: string) => Promise<void>;

  // Goals
  goals: Goal[];
  isLoadingGoals: boolean;
  fetchGoals: (status?: string) => Promise<void>;
  createGoal: (goal: Omit<Goal, 'id' | 'user_id' | 'created_at'>) => Promise<string>;
  updateGoalProgress: (goalId: string, current: number) => Promise<void>;
  deleteGoal: (goalId: string) => Promise<void>;

  // Leaderboard
  leaderboard: LeaderboardEntry[];
  isLoadingLeaderboard: boolean;
  fetchLeaderboard: (limit?: number, offset?: number) => Promise<void>;

  // Gamification
  stats: GamificationStats | null;
  isLoadingStats: boolean;
  fetchStats: () => Promise<void>;

  // Error
  error: string | null;
}

export const useAchievements = (userID: string): UseAchievementsReturn => {
  const [achievements, setAchievements] = useState<Achievement[]>([]);
  const [isLoadingAchievements, setIsLoadingAchievements] = useState(false);

  const [badges, setBadges] = useState<Badge[]>([]);
  const [isLoadingBadges, setIsLoadingBadges] = useState(false);

  const [goals, setGoals] = useState<Goal[]>([]);
  const [isLoadingGoals, setIsLoadingGoals] = useState(false);

  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [isLoadingLeaderboard, setIsLoadingLeaderboard] = useState(false);

  const [stats, setStats] = useState<GamificationStats | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(false);

  const [error, setError] = useState<string | null>(null);

  // Fetch achievements
  const fetchAchievements = useCallback(async () => {
    if (!userID) return;

    try {
      setIsLoadingAchievements(true);
      setError(null);

      const response = await apiClient.get('/v1/achievements', {
        headers: { 'X-User-ID': userID },
      });

      setAchievements(response.data.achievements || []);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load achievements';
      setError(message);
      console.error('Error fetching achievements:', err);
    } finally {
      setIsLoadingAchievements(false);
    }
  }, [userID]);

  // Fetch badges
  const fetchBadges = useCallback(async (category?: string) => {
    try {
      setIsLoadingBadges(true);
      setError(null);

      const url = category ? `/v1/badges?category=${category}` : '/v1/badges';
      const response = await apiClient.get(url);

      setBadges(response.data.badges || []);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load badges';
      setError(message);
      console.error('Error fetching badges:', err);
    } finally {
      setIsLoadingBadges(false);
    }
  }, []);

  // Fetch goals
  const fetchGoals = useCallback(async (status?: string) => {
    if (!userID) return;

    try {
      setIsLoadingGoals(true);
      setError(null);

      const url = status ? `/v1/goals?status=${status}` : '/v1/goals';
      const response = await apiClient.get(url, {
        headers: { 'X-User-ID': userID },
      });

      setGoals(response.data.goals || []);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load goals';
      setError(message);
      console.error('Error fetching goals:', err);
    } finally {
      setIsLoadingGoals(false);
    }
  }, [userID]);

  // Create goal
  const createGoal = useCallback(
    async (goal: Omit<Goal, 'id' | 'user_id' | 'created_at'>): Promise<string> => {
      try {
        const response = await apiClient.post('/v1/goals', goal, {
          headers: { 'X-User-ID': userID },
        });
        return response.data.id;
      } catch (err) {
        console.error('Error creating goal:', err);
        throw err;
      }
    },
    [userID]
  );

  // Update goal progress
  const updateGoalProgress = useCallback(
    async (goalId: string, current: number) => {
      try {
        await apiClient.put(
          `/v1/goals/update?id=${goalId}`,
          { current },
          { headers: { 'X-User-ID': userID } }
        );
      } catch (err) {
        console.error('Error updating goal:', err);
        throw err;
      }
    },
    [userID]
  );

  // Delete goal
  const deleteGoal = useCallback(
    async (goalId: string) => {
      try {
        await apiClient.delete(`/v1/goals/delete?id=${goalId}`, {
          headers: { 'X-User-ID': userID },
        });
      } catch (err) {
        console.error('Error deleting goal:', err);
        throw err;
      }
    },
    [userID]
  );

  // Fetch leaderboard
  const fetchLeaderboard = useCallback(async (limit = 100, offset = 0) => {
    try {
      setIsLoadingLeaderboard(true);
      setError(null);

      const response = await apiClient.get(
        `/v1/leaderboard?limit=${limit}&offset=${offset}`,
        { headers: { 'X-User-ID': userID } }
      );

      setLeaderboard(response.data.leaderboard || []);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load leaderboard';
      setError(message);
      console.error('Error fetching leaderboard:', err);
    } finally {
      setIsLoadingLeaderboard(false);
    }
  }, [userID]);

  // Fetch gamification stats
  const fetchStats = useCallback(async () => {
    if (!userID) return;

    try {
      setIsLoadingStats(true);
      setError(null);

      const response = await apiClient.get('/v1/gamification/stats', {
        headers: { 'X-User-ID': userID },
      });

      setStats(response.data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load stats';
      setError(message);
      console.error('Error fetching stats:', err);
    } finally {
      setIsLoadingStats(false);
    }
  }, [userID]);

  // Initial load
  useEffect(() => {
    if (userID) {
      fetchAchievements();
      fetchBadges();
      fetchGoals();
      fetchLeaderboard();
      fetchStats();
    }
  }, [userID, fetchAchievements, fetchBadges, fetchGoals, fetchLeaderboard, fetchStats]);

  return {
    achievements,
    isLoadingAchievements,
    fetchAchievements,

    badges,
    isLoadingBadges,
    fetchBadges,

    goals,
    isLoadingGoals,
    fetchGoals,
    createGoal,
    updateGoalProgress,
    deleteGoal,

    leaderboard,
    isLoadingLeaderboard,
    fetchLeaderboard,

    stats,
    isLoadingStats,
    fetchStats,

    error,
  };
};

export default useAchievements;
