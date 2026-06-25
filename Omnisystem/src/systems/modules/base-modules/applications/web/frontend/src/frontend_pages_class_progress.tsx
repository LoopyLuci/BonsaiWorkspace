// PATHFINDER Frontend - ClassProgressPage
// Real-time class progress, skill heatmap, trend analysis

import React, { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import { PieChart, Pie, Cell, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { TrendingUp, Users, Target, AlertTriangle } from 'lucide-react';

interface ClassProgress {
  classroom_id: string;
  total_students: number;
  active_students: number;
  avg_mastery: number;
  mastery_distribution: {
    mastered: number;
    developing: number;
    beginner: number;
  };
  top_skills: Array<{
    skill_id: string;
    skill_name: string;
    mastery_percent: number;
    students_mastered: number;
  }>;
  struggling_skills: Array<{
    skill_id: string;
    skill_name: string;
    avg_mastery: number;
    students_struggling: number;
  }>;
  engagement: {
    exercises_completed: number;
    active_students_today: number;
    avg_time_per_session: number;
  };
}

const ClassProgressPage: React.FC = () => {
  const { classroomId } = useParams<{ classroomId: string }>();
  const { user } = useSelector((state: RootState) => state.auth);
  const [isLoading, setIsLoading] = useState(true);
  const [progress, setProgress] = useState<ClassProgress | null>(null);

  useEffect(() => {
    const loadProgress = async () => {
      if (!classroomId || !user) return;

      try {
        setIsLoading(true);
        const res = await apiClient.get(`/v1/teachers/classrooms/${classroomId}/progress`, {
          headers: { 'X-User-ID': user.id },
        });
        setProgress(res.data);
      } catch (error) {
        console.error('Failed to load progress:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadProgress();
  }, [classroomId, user]);

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

  const masteryData = [
    { name: 'Mastered', value: progress.mastery_distribution.mastered },
    { name: 'Developing', value: progress.mastery_distribution.developing },
    { name: 'Beginner', value: progress.mastery_distribution.beginner },
  ];
  const COLORS = ['#10b981', '#f59e0b', '#ef4444'];

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Class Progress</h1>
        <p className="text-indigo-100">Real-time analytics and student performance</p>
      </div>

      {/* KEY METRICS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Total Students</span>
            <Users className="text-blue-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{progress.total_students}</p>
          <p className="text-xs text-gray-500 mt-2">
            {progress.active_students} active today
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Class Mastery</span>
            <Target className="text-green-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">
            {Math.round(progress.avg_mastery * 100)}%
          </p>
          <div className="w-full bg-gray-200 rounded-full h-2 mt-3">
            <div
              className="bg-green-500 h-2 rounded-full"
              style={{ width: `${progress.avg_mastery * 100}%` }}
            ></div>
          </div>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Exercises Today</span>
            <TrendingUp className="text-purple-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">
            {progress.engagement.exercises_completed}
          </p>
          <p className="text-xs text-gray-500 mt-2">
            {progress.engagement.active_students_today} students active
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Struggling</span>
            <AlertTriangle className="text-orange-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">
            {progress.struggling_skills.length}
          </p>
          <p className="text-xs text-gray-500 mt-2">Skills need attention</p>
        </div>
      </div>

      {/* MASTERY DISTRIBUTION PIE CHART */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Mastery Distribution</h2>
        <div className="flex justify-center">
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={masteryData}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }) => `${name}: ${value}`}
                outerRadius={100}
                fill="#8884d8"
                dataKey="value"
              >
                {masteryData.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={COLORS[index]} />
                ))}
              </Pie>
              <Tooltip />
            </PieChart>
          </ResponsiveContainer>
        </div>

        <div className="mt-6 grid grid-cols-3 gap-4">
          <div className="text-center">
            <div className="w-3 h-3 bg-green-500 rounded-full mx-auto mb-2"></div>
            <p className="text-sm text-gray-600">Mastered (≥85%)</p>
            <p className="text-2xl font-bold text-gray-900">
              {progress.mastery_distribution.mastered}
            </p>
          </div>
          <div className="text-center">
            <div className="w-3 h-3 bg-amber-500 rounded-full mx-auto mb-2"></div>
            <p className="text-sm text-gray-600">Developing (30-85%)</p>
            <p className="text-2xl font-bold text-gray-900">
              {progress.mastery_distribution.developing}
            </p>
          </div>
          <div className="text-center">
            <div className="w-3 h-3 bg-red-500 rounded-full mx-auto mb-2"></div>
            <p className="text-sm text-gray-600">Beginner (<30%)</p>
            <p className="text-2xl font-bold text-gray-900">
              {progress.mastery_distribution.beginner}
            </p>
          </div>
        </div>
      </div>

      {/* TOP SKILLS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Top Performing Skills</h2>
        <div className="space-y-4">
          {progress.top_skills.slice(0, 5).map((skill) => (
            <div key={skill.skill_id} className="border-b pb-4 last:border-b-0">
              <div className="flex items-center justify-between mb-2">
                <p className="font-medium text-gray-900">{skill.skill_name}</p>
                <span className="text-sm font-bold text-green-600">
                  {Math.round(skill.mastery_percent * 100)}%
                </span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className="bg-green-500 h-2 rounded-full"
                  style={{ width: `${skill.mastery_percent * 100}%` }}
                ></div>
              </div>
              <p className="text-xs text-gray-500 mt-2">
                {skill.students_mastered} students mastered
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* STRUGGLING SKILLS */}
      <div className="bg-white rounded-lg p-6 shadow border-l-4 border-red-500">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Skills Needing Attention</h2>
        {progress.struggling_skills.length > 0 ? (
          <div className="space-y-4">
            {progress.struggling_skills.slice(0, 5).map((skill) => (
              <div key={skill.skill_id} className="border-b pb-4 last:border-b-0">
                <div className="flex items-center justify-between mb-2">
                  <p className="font-medium text-gray-900">{skill.skill_name}</p>
                  <span className="text-sm font-bold text-red-600">
                    {Math.round(skill.avg_mastery * 100)}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-red-500 h-2 rounded-full"
                    style={{ width: `${skill.avg_mastery * 100}%` }}
                  ></div>
                </div>
                <p className="text-xs text-gray-500 mt-2">
                  {skill.students_struggling} students struggling
                </p>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <p className="text-gray-600">All skills are performing well!</p>
          </div>
        )}
      </div>

      {/* ENGAGEMENT STATS */}
      <div className="bg-gradient-to-r from-indigo-50 to-purple-50 rounded-lg p-6 border border-indigo-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4">Class Engagement</h3>
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-sm text-gray-600 mb-1">Exercises Completed</p>
            <p className="text-2xl font-bold text-gray-900">
              {progress.engagement.exercises_completed}
            </p>
          </div>
          <div>
            <p className="text-sm text-gray-600 mb-1">Active Students</p>
            <p className="text-2xl font-bold text-gray-900">
              {progress.engagement.active_students_today}
            </p>
          </div>
          <div>
            <p className="text-sm text-gray-600 mb-1">Avg Session Time</p>
            <p className="text-2xl font-bold text-gray-900">
              {Math.round(progress.engagement.avg_time_per_session)}m
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ClassProgressPage;
