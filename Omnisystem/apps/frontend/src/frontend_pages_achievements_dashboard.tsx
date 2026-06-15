// PATHFINDER Frontend - AchievementsDashboardPage
// Achievements, badges, goals, and gamification

import React, { useEffect, useState } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { Trophy, Zap, Target, TrendingUp, Award, Flame } from 'lucide-react';
import { BarChart, Bar, LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

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

interface GamificationStats {
  user_id: string;
  total_points: number;
  achievements_count: number;
  badges_unlocked: number;
  level: number;
  next_level_xp: number;
  leaderboard_rank: number;
}

const AchievementsDashboardPage: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [achievements, setAchievements] = useState<Achievement[]>([]);
  const [goals, setGoals] = useState<Goal[]>([]);
  const [stats, setStats] = useState<GamificationStats | null>(null);
  const [newGoal, setNewGoal] = useState({ title: '', description: '', type: 'skills_to_master', target: 10 });
  const [showGoalModal, setShowGoalModal] = useState(false);

  // Load achievements, goals, and stats
  useEffect(() => {
    const loadData = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        // Get achievements
        const achRes = await apiClient.get('/v1/achievements', {
          headers: { 'X-User-ID': user.id },
        });
        setAchievements(achRes.data.achievements || []);

        // Get goals
        const goalsRes = await apiClient.get('/v1/goals?status=active', {
          headers: { 'X-User-ID': user.id },
        });
        setGoals(goalsRes.data.goals || []);

        // Get gamification stats
        const statsRes = await apiClient.get('/v1/gamification/stats', {
          headers: { 'X-User-ID': user.id },
        });
        setStats(statsRes.data);
      } catch (error) {
        console.error('Failed to load achievements:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load achievements',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadData();
  }, [user, dispatch]);

  const handleCreateGoal = async () => {
    if (!user || !newGoal.title.trim()) return;

    try {
      await apiClient.post(
        '/v1/goals',
        {
          title: newGoal.title,
          description: newGoal.description,
          type: newGoal.type,
          target: newGoal.target,
          deadline: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
        },
        { headers: { 'X-User-ID': user.id } }
      );

      dispatch(
        uiActions.showNotification({
          message: 'Goal created!',
          type: 'success',
        })
      );

      setNewGoal({ title: '', description: '', type: 'skills_to_master', target: 10 });
      setShowGoalModal(false);

      // Reload goals
      const goalsRes = await apiClient.get('/v1/goals?status=active', {
        headers: { 'X-User-ID': user.id },
      });
      setGoals(goalsRes.data.goals || []);
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to create goal',
          type: 'error',
        })
      );
    }
  };

  if (isLoading || !stats) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const progressToNextLevel = (stats.total_points % 100) / 100;
  const totalAchievementPoints = achievements.reduce((sum, a) => sum + 10, 0); // Assume 10 points per achievement

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-purple-600 to-pink-600 rounded-lg p-8 text-white">
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-bold mb-2">Achievements</h1>
            <p className="text-purple-100">
              Celebrate your learning milestones and track your progress
            </p>
          </div>
          <div className="text-right">
            <p className="text-sm text-purple-200">Level</p>
            <p className="text-4xl font-bold">{stats.level}</p>
          </div>
        </div>
      </div>

      {/* GAMIFICATION STATS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {/* Level Progress */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm font-medium text-gray-600">Level Progress</span>
            <Zap className="text-yellow-500" size={20} />
          </div>
          <div className="mb-2">
            <div className="flex justify-between mb-1">
              <span className="font-bold text-gray-900">{stats.total_points % 100}/100 XP</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className="h-2 rounded-full bg-gradient-to-r from-yellow-400 to-yellow-600 transition-all duration-300"
                style={{ width: `${progressToNextLevel * 100}%` }}
              ></div>
            </div>
          </div>
          <p className="text-xs text-gray-500">Next level: {stats.next_level_xp} XP</p>
        </div>

        {/* Total Points */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm font-medium text-gray-600">Total Points</span>
            <Trophy className="text-blue-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{stats.total_points}</p>
          <p className="text-xs text-gray-500 mt-2">
            Rank <strong>#{stats.leaderboard_rank}</strong>
          </p>
        </div>

        {/* Achievements Unlocked */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm font-medium text-gray-600">Achievements</span>
            <Award className="text-green-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{stats.achievements_count}</p>
          <p className="text-xs text-gray-500 mt-2">
            <strong>{stats.badges_unlocked}</strong> badges
          </p>
        </div>

        {/* Leaderboard Rank */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm font-medium text-gray-600">Leaderboard</span>
            <TrendingUp className="text-red-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">#{stats.leaderboard_rank}</p>
          <p className="text-xs text-gray-500 mt-2">Keep climbing!</p>
        </div>
      </div>

      {/* ACTIVE GOALS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-gray-900">Active Goals</h2>
          <button
            onClick={() => setShowGoalModal(true)}
            className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white font-semibold rounded-lg transition"
          >
            + New Goal
          </button>
        </div>

        {goals.length > 0 ? (
          <div className="space-y-4">
            {goals.map((goal) => (
              <div key={goal.id} className="border border-gray-200 rounded-lg p-4">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="font-bold text-gray-900">{goal.title}</h3>
                    <p className="text-sm text-gray-600">{goal.description}</p>
                  </div>
                  <span className="text-xs font-bold px-2 py-1 bg-blue-100 text-blue-800 rounded">
                    {goal.type.replace(/_/g, ' ')}
                  </span>
                </div>

                <div className="mb-2">
                  <div className="flex justify-between mb-1">
                    <span className="text-sm font-medium text-gray-600">Progress</span>
                    <span className="text-sm font-bold text-gray-900">
                      {goal.current}/{goal.target}
                    </span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-2">
                    <div
                      className="h-2 rounded-full bg-gradient-to-r from-purple-400 to-pink-600"
                      style={{ width: `${(goal.current / goal.target) * 100}%` }}
                    ></div>
                  </div>
                </div>

                <p className="text-xs text-gray-500">
                  Due: {new Date(goal.deadline).toLocaleDateString()}
                </p>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <Target className="text-gray-300 mx-auto mb-3" size={40} />
            <p className="text-gray-600 mb-4">No active goals</p>
            <button
              onClick={() => setShowGoalModal(true)}
              className="px-6 py-2 bg-purple-600 hover:bg-purple-700 text-white font-semibold rounded-lg"
            >
              Create Your First Goal
            </button>
          </div>
        )}
      </div>

      {/* RECENT ACHIEVEMENTS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">Recent Achievements</h2>

        {achievements.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {achievements.slice(0, 9).map((achievement) => (
              <div key={achievement.id} className="border border-gray-200 rounded-lg p-4 text-center hover:shadow-lg transition">
                <div className="text-4xl mb-2">{achievement.icon_url || '🏆'}</div>
                <h3 className="font-bold text-gray-900 mb-1">{achievement.badge_name}</h3>
                <p className="text-sm text-gray-600 mb-2">{achievement.description}</p>
                <span className="text-xs font-bold text-purple-600 capitalize">
                  {achievement.category.replace(/_/g, ' ')}
                </span>
                <p className="text-xs text-gray-500 mt-2">
                  {new Date(achievement.unlocked_at).toLocaleDateString()}
                </p>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <Trophy className="text-gray-300 mx-auto mb-3" size={40} />
            <p className="text-gray-600">No achievements yet</p>
            <p className="text-sm text-gray-500 mt-2">Master skills to unlock achievements!</p>
          </div>
        )}

        {achievements.length > 9 && (
          <div className="text-center mt-6">
            <a href="/achievements/all" className="text-purple-600 font-semibold hover:text-purple-700">
              View all {achievements.length} achievements →
            </a>
          </div>
        )}
      </div>

      {/* GAMIFICATION TIPS */}
      <div className="bg-gradient-to-r from-purple-50 to-pink-50 rounded-lg p-6 border border-purple-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4">🎮 Gamification Tips</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>📈 Earn Points:</strong> Complete exercises and master skills to earn XP and unlock achievements.
          </li>
          <li>
            <strong>🏆 Level Up:</strong> Reach 100 XP to advance to the next level and unlock rewards.
          </li>
          <li>
            <strong>🎯 Set Goals:</strong> Create learning goals to stay motivated and focused.
          </li>
          <li>
            <strong>👥 Compete:</strong> See your rank on the leaderboard and compete with peers!
          </li>
          <li>
            <strong>🌟 Collect Badges:</strong> Rare badges unlock for special achievements like 30-day streaks.
          </li>
        </ul>
      </div>

      {/* NEW GOAL MODAL */}
      {showGoalModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">Create New Goal</h2>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Goal Title
                </label>
                <input
                  type="text"
                  value={newGoal.title}
                  onChange={(e) => setNewGoal({ ...newGoal, title: e.target.value })}
                  placeholder="e.g., Master 10 skills"
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Description
                </label>
                <textarea
                  value={newGoal.description}
                  onChange={(e) => setNewGoal({ ...newGoal, description: e.target.value })}
                  placeholder="Why is this goal important?"
                  rows={3}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Goal Type
                </label>
                <select
                  value={newGoal.type}
                  onChange={(e) => setNewGoal({ ...newGoal, type: e.target.value })}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500"
                >
                  <option value="skills_to_master">Skills to Master</option>
                  <option value="accuracy_target">Accuracy Target</option>
                  <option value="streak_target">Streak Target (days)</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Target Number
                </label>
                <input
                  type="number"
                  value={newGoal.target}
                  onChange={(e) => setNewGoal({ ...newGoal, target: parseInt(e.target.value) || 0 })}
                  min="1"
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-purple-500"
                />
              </div>
            </div>

            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowGoalModal(false)}
                className="flex-1 px-4 py-2 border border-gray-300 rounded-lg text-gray-700 font-semibold hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateGoal}
                className="flex-1 px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white font-semibold rounded-lg"
              >
                Create Goal
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AchievementsDashboardPage;
