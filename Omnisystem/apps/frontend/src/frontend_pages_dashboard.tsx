// PATHFINDER Frontend - DashboardPage
// Main learner interface showing skills to review and progress

import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { learnerStateActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import SkillCard from '../components/SkillCard';
import ProgressMetrics from '../components/ProgressMetrics';
import { Zap, TrendingUp, Trophy, Flame } from 'lucide-react';

interface SkillToReview {
  id: string;
  skill_code: string;
  skill_name: string;
  p_know: number;
  next_review_at?: string;
  days_overdue?: number;
  is_mastered: boolean;
  review_priority: number;
}

const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);
  const { progress, nextSkillsToReview } = useSelector((state: RootState) => state.learnerState);

  const [isLoading, setIsLoading] = useState(true);
  const [skillsToReview, setSkillsToReview] = useState<SkillToReview[]>([]);
  const [dailyMetrics, setDailyMetrics] = useState<any>(null);

  // ========================================================================
  // LOAD DATA
  // ========================================================================

  useEffect(() => {
    const loadDashboardData = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        // Fetch skills to review (spaced repetition priority order)
        const skills = await apiClient.getNextSkillsToReview(user.id, 5);
        setSkillsToReview(skills.skills || []);

        // Fetch overall progress
        const progressData = await apiClient.getProgress(user.id);
        dispatch(learnerStateActions.setProgress(progressData));

        // Fetch today's metrics
        const metrics = await apiClient.getDailyMetrics(user.id);
        setDailyMetrics(metrics);
      } catch (error) {
        console.error('Failed to load dashboard data:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadDashboardData();
  }, [user, dispatch]);

  // ========================================================================
  // HANDLERS
  // ========================================================================

  const handleStartLesson = (skillId: string) => {
    navigate(`/skills/${skillId}/lessons/1`);
  };

  // ========================================================================
  // RENDER
  // ========================================================================

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const masteryPercent = progress ? Math.round(progress.mastery_percentage) : 0;
  const currentStreak = dailyMetrics?.is_streak_day ? (progress?.current_streak || 0) + 1 : progress?.current_streak || 0;

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">
          Welcome back, {user?.first_name}! 🎓
        </h1>
        <p className="text-indigo-100">
          Keep up your learning streak and master new skills
        </p>
      </div>

      {/* QUICK STATS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {/* Mastery Percent */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Mastery</span>
            <Trophy className="text-yellow-500" size={20} />
          </div>
          <div className="text-3xl font-bold text-gray-900">{masteryPercent}%</div>
          <div className="w-full bg-gray-200 rounded-full h-2 mt-3">
            <div
              className="bg-indigo-600 h-2 rounded-full"
              style={{ width: `${masteryPercent}%` }}
            ></div>
          </div>
        </div>

        {/* Current Streak */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Streak</span>
            <Flame className="text-red-500" size={20} />
          </div>
          <div className="text-3xl font-bold text-gray-900">
            {currentStreak}
            <span className="text-lg text-gray-500">d</span>
          </div>
          <p className="text-xs text-gray-500 mt-2">
            {dailyMetrics?.is_streak_day ? 'Great job today!' : 'Practice today to continue'}
          </p>
        </div>

        {/* Exercises Today */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Exercises</span>
            <Zap className="text-orange-500" size={20} />
          </div>
          <div className="text-3xl font-bold text-gray-900">
            {dailyMetrics?.exercises_attempted || 0}
          </div>
          <p className="text-xs text-gray-500 mt-2">
            {dailyMetrics?.correct_rate ? `${Math.round(dailyMetrics.correct_rate * 100)}% accuracy` : 'No practice yet'}
          </p>
        </div>

        {/* Skills Mastered */}
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Mastered</span>
            <TrendingUp className="text-green-500" size={20} />
          </div>
          <div className="text-3xl font-bold text-gray-900">
            {progress?.mastered_skills || 0}
            <span className="text-lg text-gray-500">/{progress?.total_skills || 0}</span>
          </div>
          <p className="text-xs text-gray-500 mt-2">skills completed</p>
        </div>
      </div>

      {/* PROGRESS VISUALIZATION */}
      {progress && <ProgressMetrics metrics={progress} />}

      {/* SKILLS TO REVIEW (SPACED REPETITION) */}
      <div>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">Skills to Review</h2>
            <p className="text-gray-600 text-sm mt-1">
              Optimized using spaced repetition science
            </p>
          </div>
          <a href="/progress" className="text-indigo-600 hover:text-indigo-700 font-semibold">
            View all →
          </a>
        </div>

        {skillsToReview.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {skillsToReview.map((skill) => (
              <SkillCard
                key={skill.id}
                skillId={skill.id}
                skillCode={skill.skill_code}
                skillName={skill.skill_name}
                pKnow={skill.p_know}
                isMastered={skill.is_mastered}
                reviewPriority={skill.review_priority}
                daysOverdue={skill.days_overdue}
                onStart={() => handleStartLesson(skill.id)}
              />
            ))}
          </div>
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <p className="text-gray-600 mb-4">
              No skills to review right now!
            </p>
            <p className="text-sm text-gray-500">
              Come back tomorrow for your next spaced repetition review
            </p>
          </div>
        )}
      </div>

      {/* DAILY CHALLENGE */}
      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 border border-indigo-100">
        <h3 className="text-lg font-semibold text-gray-900 mb-2">Today's Challenge</h3>
        <p className="text-gray-700 mb-4">
          Complete 5 exercises to earn 50 XP and keep your streak alive!
        </p>
        <div className="bg-white rounded p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600">Exercises completed: {dailyMetrics?.exercises_attempted || 0}/5</p>
              <div className="w-32 bg-gray-200 rounded-full h-2 mt-2">
                <div
                  className="bg-indigo-600 h-2 rounded-full"
                  style={{ width: `${Math.min(100, ((dailyMetrics?.exercises_attempted || 0) / 5) * 100)}%` }}
                ></div>
              </div>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-indigo-600">
                {(dailyMetrics?.xp_earned || 0)}
              </div>
              <p className="text-xs text-gray-500">XP earned today</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
