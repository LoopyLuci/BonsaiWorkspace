// PATHFINDER Frontend - LearningInsightsPage
// Personalized insights, recommendations, and learning analytics

import React, { useEffect, useState } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { BarChart, Bar, LineChart, Line, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { TrendingUp, Brain, Target, Clock, Lightbulb, AlertCircle } from 'lucide-react';

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

const LearningInsightsPage: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [analytics, setAnalytics] = useState<LearningAnalytics | null>(null);
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [learningStyle, setLearningStyle] = useState<LearningStyle | null>(null);
  const [performance, setPerformance] = useState<PerformanceMetric[]>([]);

  // Load all insights
  useEffect(() => {
    const loadInsights = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        // Get analytics
        const analyticsRes = await apiClient.get('/v1/insights/analytics', {
          headers: { 'X-User-ID': user.id },
        });
        setAnalytics(analyticsRes.data);

        // Get recommendations
        const recsRes = await apiClient.get('/v1/insights/recommendations?limit=5', {
          headers: { 'X-User-ID': user.id },
        });
        setRecommendations(recsRes.data.recommendations || []);

        // Get learning style
        const styleRes = await apiClient.get('/v1/insights/learning-style', {
          headers: { 'X-User-ID': user.id },
        });
        setLearningStyle(styleRes.data);

        // Get performance metrics
        const perfRes = await apiClient.get('/v1/insights/performance', {
          headers: { 'X-User-ID': user.id },
        });
        setPerformance(perfRes.data.metrics || []);
      } catch (error) {
        console.error('Failed to load insights:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load insights',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadInsights();
  }, [user, dispatch]);

  if (isLoading || !analytics) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const learningStyleData = learningStyle ? [
    { name: 'Visual', value: learningStyle.visual_percent },
    { name: 'Auditory', value: learningStyle.auditory_percent },
    { name: 'Kinesthetic', value: learningStyle.kinesthetic_percent },
    { name: 'Reading', value: learningStyle.reading_percent },
  ] : [];

  const COLORS = ['#3b82f6', '#8b5cf6', '#ec4899', '#f59e0b'];

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-cyan-600 to-blue-600 rounded-lg p-8 text-white">
        <div className="flex items-center gap-3 mb-2">
          <Brain size={32} />
          <h1 className="text-3xl font-bold">Learning Insights</h1>
        </div>
        <p className="text-cyan-100">
          Personalized analytics and recommendations to optimize your learning
        </p>
      </div>

      {/* KEY METRICS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Total Skills</p>
          <p className="text-3xl font-bold text-gray-900">{analytics.total_skills}</p>
          <p className="text-xs text-gray-500 mt-2">
            <strong>{analytics.mastered_skills}</strong> mastered
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Average Mastery</p>
          <p className="text-3xl font-bold text-gray-900">{Math.round(analytics.average_mastery)}%</p>
          <p className="text-xs text-gray-500 mt-2">
            <strong>{analytics.struggle_skills}</strong> struggling
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Accuracy</p>
          <p className="text-3xl font-bold text-gray-900">{Math.round(analytics.average_accuracy)}%</p>
          <p className="text-xs text-gray-500 mt-2">
            <strong>{analytics.total_exercises}</strong> exercises
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Learning Streak</p>
          <p className="text-3xl font-bold text-gray-900">🔥 {analytics.current_streak}d</p>
          <p className="text-xs text-gray-500 mt-2">
            Best: <strong>{analytics.longest_streak}</strong> days
          </p>
        </div>
      </div>

      {/* RECOMMENDATIONS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
          <Lightbulb className="text-yellow-500" size={24} />
          Personalized Recommendations
        </h2>

        {recommendations.length > 0 ? (
          <div className="space-y-3">
            {recommendations.map((rec) => (
              <div
                key={rec.id}
                className={`rounded-lg p-4 border-l-4 ${
                  rec.priority === 'high'
                    ? 'bg-red-50 border-red-500'
                    : rec.priority === 'medium'
                    ? 'bg-orange-50 border-orange-500'
                    : 'bg-yellow-50 border-yellow-500'
                }`}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <h3 className="font-bold text-gray-900">
                      {rec.action_text || `${rec.type.charAt(0).toUpperCase()}${rec.type.slice(1)}`}
                    </h3>
                    {rec.skill_name && (
                      <p className="text-sm text-gray-600">Skill: {rec.skill_name}</p>
                    )}
                  </div>
                  <span className="text-xs font-bold px-2 py-1 bg-white rounded">
                    {rec.priority.toUpperCase()}
                  </span>
                </div>
                <p className="text-sm text-gray-700 mb-2">{rec.reason}</p>
                <p className="text-xs text-gray-500">
                  ⏱️ {rec.time_needed_minutes} minutes
                </p>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-gray-500 text-center py-8">
            No recommendations at this time. Keep learning!
          </p>
        )}
      </div>

      {/* LEARNING STYLE & PERFORMANCE */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Learning Style */}
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
            <Brain className="text-purple-500" size={24} />
            Your Learning Style
          </h2>

          <div className="text-center mb-6">
            {learningStyle && learningStyleData.length > 0 ? (
              <ResponsiveContainer width="100%" height={250}>
                <PieChart>
                  <Pie
                    data={learningStyleData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={({ name, value }) => `${name} ${value}%`}
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {learningStyleData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => `${value}%`} />
                </PieChart>
              </ResponsiveContainer>
            ) : null}
          </div>

          {learningStyle && (
            <div className="bg-blue-50 rounded-lg p-4">
              <p className="text-sm text-gray-600 mb-1">Dominant Style</p>
              <p className="text-lg font-bold text-gray-900 capitalize">
                {learningStyle.dominant_style}
              </p>
              <p className="text-xs text-gray-500 mt-2">
                We recommend tailoring your study approach to this style for optimal learning.
              </p>
            </div>
          )}
        </div>

        {/* Top Skills */}
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
            <TrendingUp className="text-green-500" size={24} />
            Top Skills
          </h2>

          <div className="space-y-3 max-h-96 overflow-y-auto">
            {performance.slice(0, 5).map((skill, index) => (
              <div key={skill.skill_name} className="border border-gray-200 rounded-lg p-3">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <p className="font-semibold text-gray-900">
                      #{index + 1} {skill.skill_name}
                    </p>
                  </div>
                  <span
                    className={`text-xs font-bold px-2 py-1 rounded ${
                      skill.trend === 'up'
                        ? 'bg-green-100 text-green-800'
                        : skill.trend === 'down'
                        ? 'bg-red-100 text-red-800'
                        : 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    {skill.trend === 'up' ? '↗' : skill.trend === 'down' ? '↘' : '→'}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div>
                    <p className="text-gray-600">Mastery</p>
                    <p className="font-bold text-gray-900">{Math.round(skill.mastery_percent)}%</p>
                  </div>
                  <div>
                    <p className="text-gray-600">Accuracy</p>
                    <p className="font-bold text-gray-900">{Math.round(skill.accuracy_percent)}%</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* STUDY TIME ANALYSIS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
          <Clock className="text-blue-500" size={24} />
          Study Analysis
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-blue-50 rounded-lg p-4">
            <p className="text-sm text-gray-600 mb-1">Total Study Time</p>
            <p className="text-2xl font-bold text-gray-900">
              {Math.round(analytics.total_time_spent_minutes / 60)}h
              <span className="text-sm text-gray-600">
                {analytics.total_time_spent_minutes % 60}m
              </span>
            </p>
          </div>

          <div className="bg-green-50 rounded-lg p-4">
            <p className="text-sm text-gray-600 mb-1">Average per Exercise</p>
            <p className="text-2xl font-bold text-gray-900">
              {analytics.average_time_per_exercise}m
            </p>
          </div>

          <div className="bg-purple-50 rounded-lg p-4">
            <p className="text-sm text-gray-600 mb-1">Recommended Daily</p>
            <p className="text-2xl font-bold text-gray-900">30-45m</p>
            <p className="text-xs text-gray-500 mt-1">Optimal for retention</p>
          </div>
        </div>
      </div>

      {/* INSIGHTS TIPS */}
      <div className="bg-gradient-to-r from-cyan-50 to-blue-50 rounded-lg p-6 border border-cyan-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4 flex items-center gap-2">
          <Lightbulb className="text-yellow-500" size={20} />
          Learning Insights Tips
        </h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>📊 Learn Your Style:</strong> Use your dominant learning style to choose study materials (videos for auditory, diagrams for visual, etc.)
          </li>
          <li>
            <strong>🎯 Follow Recommendations:</strong> Our AI generates recommendations based on your progress and learning patterns.
          </li>
          <li>
            <strong>⏱️ Optimize Time:</strong> Study for 30-45 minutes daily rather than long cramming sessions.
          </li>
          <li>
            <strong>📈 Track Trends:</strong> Monitor whether your skills are improving (↗), stable (→), or declining (↘).
          </li>
          <li>
            <strong>🔥 Maintain Streaks:</strong> Consistent daily practice is more important than total hours.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default LearningInsightsPage;
