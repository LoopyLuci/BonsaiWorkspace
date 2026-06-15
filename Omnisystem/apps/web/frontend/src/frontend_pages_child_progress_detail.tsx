// PATHFINDER Frontend - ChildProgressDetailPage
// Detailed view of a child's learning metrics and recommendations

import React, { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, PieChart, Pie, Cell } from 'recharts';
import { TrendingUp, Award, Zap, Target, Calendar, Book } from 'lucide-react';

interface ChildDetails {
  student_id: string;
  student_name: string;
  mastery_percent: number;
  skills_mastered: number;
  total_skills: number;
  current_skill: string;
  status: string;
  exercises_today: number;
  accuracy_today: number;
  streak_days: number;
  last_activity: string;
}

interface LearningCurve {
  date: string;
  mastery: number;
  exercises: number;
}

interface SkillBreakdown {
  skill_name: string;
  mastery: number;
  status: string;
  last_attempt: string;
}

interface Recommendation {
  id: string;
  title: string;
  description: string;
  action: string;
  priority: string;
}

const ChildProgressDetailPage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);
  const { id: studentId } = useParams<{ id: string }>();

  const [isLoading, setIsLoading] = useState(true);
  const [child, setChild] = useState<ChildDetails | null>(null);
  const [learningCurve, setLearningCurve] = useState<LearningCurve[]>([]);
  const [skillBreakdown, setSkillBreakdown] = useState<SkillBreakdown[]>([]);
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [selectedTimeRange, setSelectedTimeRange] = useState<'week' | 'month' | 'all'>('week');

  // Load child details and metrics
  useEffect(() => {
    const loadChildProgress = async () => {
      if (!user || !studentId) return;

      try {
        setIsLoading(true);

        // Get child progress
        const progressRes = await apiClient.get(
          `/v1/parents/children/${studentId}/progress`,
          { headers: { 'X-User-ID': user.id } }
        );
        setChild(progressRes.data);

        // Get learning curve data
        const curveRes = await apiClient.get(
          `/v1/parents/children/${studentId}/learning-curve?range=${selectedTimeRange}`,
          { headers: { 'X-User-ID': user.id } }
        );
        setLearningCurve(curveRes.data.data || []);

        // Get skill breakdown
        const skillsRes = await apiClient.get(
          `/v1/parents/children/${studentId}/skills`,
          { headers: { 'X-User-ID': user.id } }
        );
        setSkillBreakdown(skillsRes.data.skills || []);

        // Get recommendations
        const recsRes = await apiClient.get(
          `/v1/parents/children/${studentId}/recommendations`,
          { headers: { 'X-User-ID': user.id } }
        );
        setRecommendations(recsRes.data.recommendations || []);
      } catch (error) {
        console.error('Failed to load child progress:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load child progress',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadChildProgress();
  }, [user, studentId, selectedTimeRange, dispatch]);

  if (isLoading || !child) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const masteryColor =
    child.status === 'excellent'
      ? 'text-green-600'
      : child.status === 'developing'
      ? 'text-blue-600'
      : 'text-red-600';

  const masteryBg =
    child.status === 'excellent'
      ? 'bg-green-50'
      : child.status === 'developing'
      ? 'bg-blue-50'
      : 'bg-red-50';

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div
        className={`${masteryBg} rounded-lg p-8 border-l-4 ${masteryColor.replace('text-', 'border-')}`}
      >
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 mb-2">{child.student_name}</h1>
            <p className={`text-lg font-semibold ${masteryColor}`}>
              {child.status.charAt(0).toUpperCase() + child.status.slice(1)} Progress
            </p>
          </div>
          <button
            onClick={() => navigate('/parent')}
            className="px-4 py-2 bg-white border border-gray-300 rounded-lg text-gray-700 font-semibold hover:bg-gray-50 transition"
          >
            ← Back
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {/* Overall Mastery */}
          <div>
            <p className="text-sm text-gray-600 mb-2">Overall Mastery</p>
            <div className="flex items-center gap-3">
              <div className="flex-1">
                <div className="bg-gray-200 rounded-full h-3">
                  <div
                    className={`h-3 rounded-full ${
                      child.mastery_percent >= 85
                        ? 'bg-green-500'
                        : child.mastery_percent >= 50
                        ? 'bg-blue-500'
                        : 'bg-orange-500'
                    }`}
                    style={{ width: `${child.mastery_percent}%` }}
                  ></div>
                </div>
              </div>
              <span className="text-xl font-bold text-gray-900">{Math.round(child.mastery_percent)}%</span>
            </div>
          </div>

          {/* Skills Mastered */}
          <div>
            <p className="text-sm text-gray-600 mb-2">Skills Mastered</p>
            <p className="text-2xl font-bold text-gray-900">
              {child.skills_mastered}/{child.total_skills}
            </p>
            <p className="text-xs text-gray-500">
              {Math.round((child.skills_mastered / child.total_skills) * 100)}% complete
            </p>
          </div>

          {/* Streak */}
          <div>
            <p className="text-sm text-gray-600 mb-2">Learning Streak</p>
            <p className="text-2xl font-bold text-gray-900">🔥 {child.streak_days} days</p>
            <p className="text-xs text-gray-500">Keep it going!</p>
          </div>

          {/* Accuracy */}
          <div>
            <p className="text-sm text-gray-600 mb-2">Accuracy Today</p>
            <p className="text-2xl font-bold text-gray-900">{Math.round(child.accuracy_today)}%</p>
            <p className="text-xs text-gray-500">{child.exercises_today} exercises</p>
          </div>
        </div>
      </div>

      {/* LEARNING CURVE */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-2xl font-bold text-gray-900">Learning Curve</h2>
          <div className="flex gap-2">
            {(['week', 'month', 'all'] as const).map((range) => (
              <button
                key={range}
                onClick={() => setSelectedTimeRange(range)}
                className={`px-4 py-2 rounded-lg font-semibold transition ${
                  selectedTimeRange === range
                    ? 'bg-indigo-600 text-white'
                    : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                }`}
              >
                {range.charAt(0).toUpperCase() + range.slice(1)}
              </button>
            ))}
          </div>
        </div>

        {learningCurve.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={learningCurve}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="date" />
              <YAxis />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#fff',
                  border: '1px solid #e5e7eb',
                  borderRadius: '0.5rem',
                }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="mastery"
                stroke="#10b981"
                name="Mastery %"
                strokeWidth={2}
                dot={{ r: 4 }}
              />
              <Line
                type="monotone"
                dataKey="exercises"
                stroke="#3b82f6"
                name="Exercises"
                strokeWidth={2}
                dot={{ r: 4 }}
              />
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <div className="text-center py-12 text-gray-500">
            No data available for this time period
          </div>
        )}
      </div>

      {/* SKILL BREAKDOWN */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Skills Table */}
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-2xl font-bold text-gray-900 mb-4">Skills Progress</h2>

          {skillBreakdown.length > 0 ? (
            <div className="space-y-3 max-h-96 overflow-y-auto">
              {skillBreakdown.map((skill) => (
                <div key={skill.skill_name} className="border border-gray-200 rounded-lg p-4">
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <p className="font-semibold text-gray-900">{skill.skill_name}</p>
                      <p className="text-xs text-gray-500">Last: {skill.last_attempt}</p>
                    </div>
                    <span
                      className={`text-xs font-bold px-2 py-1 rounded ${
                        skill.status === 'mastered'
                          ? 'bg-green-100 text-green-800'
                          : skill.status === 'developing'
                          ? 'bg-blue-100 text-blue-800'
                          : 'bg-orange-100 text-orange-800'
                      }`}
                    >
                      {skill.status.charAt(0).toUpperCase() + skill.status.slice(1)}
                    </span>
                  </div>
                  <div className="bg-gray-200 rounded-full h-2">
                    <div
                      className={`h-2 rounded-full ${
                        skill.mastery >= 85 ? 'bg-green-500' : skill.mastery >= 50 ? 'bg-blue-500' : 'bg-orange-500'
                      }`}
                      style={{ width: `${skill.mastery}%` }}
                    ></div>
                  </div>
                  <p className="text-sm font-bold text-gray-900 mt-2">{Math.round(skill.mastery)}%</p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-8">No skills tracked yet</p>
          )}
        </div>

        {/* Recommendations */}
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-2xl font-bold text-gray-900 mb-4">Recommendations</h2>

          {recommendations.length > 0 ? (
            <div className="space-y-3 max-h-96 overflow-y-auto">
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
                  <h3 className="font-bold text-gray-900 mb-1">{rec.title}</h3>
                  <p className="text-sm text-gray-700 mb-3">{rec.description}</p>
                  <button className="text-sm font-semibold text-indigo-600 hover:text-indigo-700">
                    {rec.action} →
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-8">No recommendations at this time</p>
          )}
        </div>
      </div>

      {/* ACTIVITY SUMMARY */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">Activity Summary</h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="border border-gray-200 rounded-lg p-4">
            <div className="flex items-center gap-3 mb-2">
              <Zap className="text-orange-500" size={20} />
              <p className="text-sm font-medium text-gray-600">Last Activity</p>
            </div>
            <p className="text-lg font-bold text-gray-900">{child.last_activity}</p>
          </div>

          <div className="border border-gray-200 rounded-lg p-4">
            <div className="flex items-center gap-3 mb-2">
              <Target className="text-blue-500" size={20} />
              <p className="text-sm font-medium text-gray-600">Current Focus</p>
            </div>
            <p className="text-lg font-bold text-gray-900">{child.current_skill || 'Assessing...'}</p>
          </div>

          <div className="border border-gray-200 rounded-lg p-4">
            <div className="flex items-center gap-3 mb-2">
              <TrendingUp className="text-green-500" size={20} />
              <p className="text-sm font-medium text-gray-600">Learning Rate</p>
            </div>
            <p className="text-lg font-bold text-gray-900">
              {child.mastery_percent > 70
                ? '↗ Fast'
                : child.mastery_percent > 40
                ? '→ Steady'
                : '↙ Needs Support'}
            </p>
          </div>
        </div>
      </div>

      {/* PARENT RESOURCES */}
      <div className="bg-gradient-to-r from-indigo-50 to-blue-50 rounded-lg p-6 border border-indigo-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4">How to Support Your Child</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>📚 Encourage Consistency:</strong> Daily learning builds lasting knowledge. Help create a
            routine.
          </li>
          <li>
            <strong>💬 Ask About Skills:</strong> Talk with your child about what they're learning. Ask them to
            explain concepts.
          </li>
          <li>
            <strong>🎯 Focus on Progress:</strong> Celebrate improvements in mastery percentage and skill
            completions.
          </li>
          <li>
            <strong>🤝 Review Recommendations:</strong> Check recommendations section for personalized support
            strategies.
          </li>
          <li>
            <strong>📊 Track Streaks:</strong> Help your child maintain their learning streak for motivation.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default ChildProgressDetailPage;
