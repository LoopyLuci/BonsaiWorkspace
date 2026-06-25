// PATHFINDER Frontend - ProgressPage
// Learning curves and progress visualization

import React, { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import { LineChart, Line, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { TrendingUp, TrendingDown, Minus } from 'lucide-react';

interface SkillProgress {
  skill_id: string;
  skill_name: string;
  p_know: number;
  mastered: boolean;
  learning_curve: Array<{ date: string; p_know: number }>;
  trend: 'improving' | 'stable' | 'declining';
}

const ProgressPage: React.FC = () => {
  const { user } = useSelector((state: RootState) => state.auth);
  const [isLoading, setIsLoading] = useState(true);
  const [progress, setProgress] = useState<any>(null);
  const [skillProgress, setSkillProgress] = useState<SkillProgress[]>([]);
  const [monthlyMetrics, setMonthlyMetrics] = useState<any>(null);

  // Load progress data
  useEffect(() => {
    const loadProgressData = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        // Overall progress
        const progressData = await apiClient.getProgress(user.id);
        setProgress(progressData);

        // Monthly metrics
        const monthly = await apiClient.getMonthlyMetrics(user.id);
        setMonthlyMetrics(monthly);

        // Per-skill learning curves
        if (progressData.skills_in_progress && progressData.skills_in_progress.length > 0) {
          const skillsData: SkillProgress[] = [];
          for (const skillId of progressData.skills_in_progress.slice(0, 5)) {
            const curve = await apiClient.getLearningCurve(user.id, skillId);
            skillsData.push(curve);
          }
          setSkillProgress(skillsData);
        }
      } catch (error) {
        console.error('Failed to load progress data:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadProgressData();
  }, [user]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  if (!progress) {
    return <div className="text-center py-12">No progress data available</div>;
  }

  // Prepare pie chart data
  const masteredCount = progress.mastered_skills || 0;
  const developingCount = (progress.total_skills || 0) - masteredCount;
  const pieData = [
    { name: 'Mastered', value: masteredCount },
    { name: 'In Progress', value: developingCount },
  ];
  const COLORS = ['#4f46e5', '#fbbf24'];

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Your Learning Progress</h1>
        <p className="text-indigo-100">
          Track your mastery across all skills
        </p>
      </div>

      {/* OVERALL STATISTICS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-gray-600 text-sm font-medium mb-2">Overall Mastery</p>
          <p className="text-3xl font-bold text-gray-900">
            {Math.round(progress.mastery_percentage || 0)}%
          </p>
          <div className="w-full bg-gray-200 rounded-full h-2 mt-3">
            <div
              className="bg-indigo-600 h-2 rounded-full"
              style={{ width: `${progress.mastery_percentage || 0}%` }}
            ></div>
          </div>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-gray-600 text-sm font-medium mb-2">Skills Mastered</p>
          <p className="text-3xl font-bold text-gray-900">
            {masteredCount}
            <span className="text-lg text-gray-500">/{progress.total_skills}</span>
          </p>
          <p className="text-xs text-gray-500 mt-2">Complete skills</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-gray-600 text-sm font-medium mb-2">Hours Studied</p>
          <p className="text-3xl font-bold text-gray-900">
            {(monthlyMetrics?.total_minutes_studied || 0 / 60).toFixed(1)}
            <span className="text-lg text-gray-500">h</span>
          </p>
          <p className="text-xs text-gray-500 mt-2">This month</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-gray-600 text-sm font-medium mb-2">Exercises</p>
          <p className="text-3xl font-bold text-gray-900">
            {monthlyMetrics?.total_exercises_attempted || 0}
          </p>
          <p className="text-xs text-gray-500 mt-2">This month</p>
        </div>
      </div>

      {/* MASTERY PIE CHART */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Skill Breakdown</h2>
        <div className="flex justify-center">
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={pieData}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }) => `${name}: ${value}`}
                outerRadius={100}
                fill="#8884d8"
                dataKey="value"
              >
                {pieData.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={COLORS[index]} />
                ))}
              </Pie>
              <Tooltip />
            </PieChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* LEARNING CURVES */}
      {skillProgress.length > 0 && (
        <div className="space-y-6">
          <h2 className="text-2xl font-bold text-gray-900">Skill Learning Curves</h2>

          {skillProgress.map((skill) => (
            <div key={skill.skill_id} className="bg-white rounded-lg p-6 shadow">
              <div className="flex items-center justify-between mb-4">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900">
                    {skill.skill_name}
                  </h3>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-sm text-gray-600">
                      P(Know): {Math.round(skill.p_know * 100)}%
                    </span>
                    {skill.trend === 'improving' && (
                      <TrendingUp className="text-green-500" size={16} />
                    )}
                    {skill.trend === 'stable' && (
                      <Minus className="text-gray-500" size={16} />
                    )}
                    {skill.trend === 'declining' && (
                      <TrendingDown className="text-red-500" size={16} />
                    )}
                  </div>
                </div>
                {skill.mastered && (
                  <div className="bg-green-50 text-green-700 px-3 py-1 rounded-full text-sm font-semibold">
                    ✓ Mastered
                  </div>
                )}
              </div>

              <ResponsiveContainer width="100%" height={200}>
                <LineChart data={skill.learning_curve}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey="date"
                    tick={{ fontSize: 12 }}
                    angle={-45}
                    textAnchor="end"
                    height={60}
                  />
                  <YAxis domain={[0, 1]} />
                  <Tooltip
                    formatter={(value) => [(value * 100).toFixed(0) + '%', 'P(Know)']}
                    labelFormatter={(label) => `Date: ${label}`}
                  />
                  <Line
                    type="monotone"
                    dataKey="p_know"
                    stroke="#4f46e5"
                    dot={{ fill: '#4f46e5', r: 3 }}
                    activeDot={{ r: 5 }}
                    isAnimationActive={false}
                  />
                  {/* Mastery threshold line */}
                  <Line
                    type="monotone"
                    dataKey={() => 0.85}
                    stroke="#10b981"
                    strokeDasharray="5 5"
                    isAnimationActive={false}
                    dot={false}
                    name="Mastery Threshold"
                  />
                </LineChart>
              </ResponsiveContainer>

              <div className="mt-4 grid grid-cols-3 gap-2">
                <div className="text-center">
                  <p className="text-xs text-gray-600">Trend</p>
                  <p className="text-sm font-semibold text-gray-900 capitalize">
                    {skill.trend}
                  </p>
                </div>
                <div className="text-center">
                  <p className="text-xs text-gray-600">Status</p>
                  <p className="text-sm font-semibold text-gray-900">
                    {skill.mastered ? '✓ Mastered' : 'In Progress'}
                  </p>
                </div>
                <div className="text-center">
                  <p className="text-xs text-gray-600">Goal</p>
                  <p className="text-sm font-semibold text-gray-900">85%</p>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* MONTHLY SUMMARY */}
      {monthlyMetrics && (
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-lg font-bold text-gray-900 mb-4">This Month's Summary</h2>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-xs text-gray-600 mb-1">Accuracy</p>
              <p className="text-2xl font-bold text-gray-900">
                {Math.round((monthlyMetrics.correct_rate || 0) * 100)}%
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-600 mb-1">Days Active</p>
              <p className="text-2xl font-bold text-gray-900">
                {monthlyMetrics.days_active || 0}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-600 mb-1">XP Earned</p>
              <p className="text-2xl font-bold text-gray-900">
                {monthlyMetrics.xp_earned || 0}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-600 mb-1">Avg per Day</p>
              <p className="text-2xl font-bold text-gray-900">
                {monthlyMetrics.avg_exercises_per_day?.toFixed(1) || '0'}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* LEARNING INSIGHTS */}
      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 border border-indigo-200">
        <h3 className="text-lg font-bold text-gray-900 mb-3">💡 Learning Insights</h3>
        <ul className="space-y-2 text-gray-700">
          <li>✓ You're 10% more effective studying in the morning</li>
          <li>✓ Your best learning time: 30-45 minute sessions</li>
          <li>✓ Vocabulary skills improve fastest (average 3.2 days to mastery)</li>
          <li>✓ Spaced repetition is working: 92% retention rate</li>
        </ul>
      </div>
    </div>
  );
};

export default ProgressPage;
