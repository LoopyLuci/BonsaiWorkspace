// PATHFINDER Frontend - TeacherDashboardPage
// Overview of teacher's classrooms and key metrics

import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import ClassroomCard from '../components/ClassroomCard';
import { Plus, Users, TrendingUp, AlertCircle } from 'lucide-react';

interface Classroom {
  id: string;
  name: string;
  subject: string;
  grade_level: string;
  total_students?: number;
  active_students?: number;
  avg_mastery?: number;
  unresolved_alerts?: number;
  created_at: string;
}

const TeacherDashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const { user } = useSelector((state: RootState) => state.auth);
  const [isLoading, setIsLoading] = useState(true);
  const [classrooms, setClassrooms] = useState<Classroom[]>([]);
  const [totalStudents, setTotalStudents] = useState(0);
  const [totalAlerts, setTotalAlerts] = useState(0);
  const [avgClassMastery, setAvgClassMastery] = useState(0);

  // Load classrooms
  useEffect(() => {
    const loadClassrooms = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        // Get classrooms
        const response = await apiClient.get('/v1/teachers/classrooms', {
          headers: { 'X-User-ID': user.id },
        });

        const classroomsData = response.data.classrooms || [];
        setClassrooms(classroomsData);

        // Calculate aggregates
        let totalStudents = 0;
        let totalAlerts = 0;
        let totalMastery = 0;

        for (const classroom of classroomsData) {
          totalStudents += classroom.total_students || 0;
          totalAlerts += classroom.unresolved_alerts || 0;
          totalMastery += classroom.avg_mastery || 0;
        }

        setTotalStudents(totalStudents);
        setTotalAlerts(totalAlerts);

        if (classroomsData.length > 0) {
          setAvgClassMastery(Math.round((totalMastery / classroomsData.length) * 100));
        }
      } catch (error) {
        console.error('Failed to load classrooms:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadClassrooms();
  }, [user]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Welcome back, {user?.first_name}! 👨‍🏫</h1>
        <p className="text-indigo-100">
          Manage your classrooms and monitor student progress
        </p>
      </div>

      {/* QUICK STATS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-gray-600 text-sm font-medium mb-2">Total Classrooms</p>
          <p className="text-3xl font-bold text-gray-900">{classrooms.length}</p>
          <p className="text-xs text-gray-500 mt-2">Active classes</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Total Students</span>
            <Users className="text-blue-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{totalStudents}</p>
          <p className="text-xs text-gray-500 mt-2">Enrolled students</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Class Mastery</span>
            <TrendingUp className="text-green-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{avgClassMastery}%</p>
          <p className="text-xs text-gray-500 mt-2">Average across classes</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Alerts</span>
            <AlertCircle className={totalAlerts > 0 ? 'text-red-500' : 'text-gray-400'} size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{totalAlerts}</p>
          <p className="text-xs text-gray-500 mt-2">Unresolved alerts</p>
        </div>
      </div>

      {/* CLASSROOMS SECTION */}
      <div>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">Your Classrooms</h2>
            <p className="text-gray-600 text-sm mt-1">
              Manage classrooms and monitor student progress
            </p>
          </div>
          <button
            onClick={() => navigate('/teacher/classrooms/new')}
            className="flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg transition"
          >
            <Plus size={20} />
            New Classroom
          </button>
        </div>

        {classrooms.length > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {classrooms.map((classroom) => (
              <ClassroomCard
                key={classroom.id}
                classroom={classroom}
                onViewDetails={() => navigate(`/teacher/classrooms/${classroom.id}`)}
              />
            ))}
          </div>
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <p className="text-gray-600 mb-4">No classrooms yet</p>
            <button
              onClick={() => navigate('/teacher/classrooms/new')}
              className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg transition"
            >
              Create Your First Classroom
            </button>
          </div>
        )}
      </div>

      {/* ALERTS SECTION */}
      {totalAlerts > 0 && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-6">
          <div className="flex items-center gap-3 mb-4">
            <AlertCircle className="text-red-600" size={24} />
            <h3 className="text-lg font-bold text-red-900">Student Alerts</h3>
          </div>
          <p className="text-red-700 mb-4">
            You have {totalAlerts} unresolved alerts about students who may need intervention.
          </p>
          <button
            onClick={() => navigate('/teacher/alerts')}
            className="px-6 py-2 bg-red-600 hover:bg-red-700 text-white font-semibold rounded-lg transition"
          >
            View All Alerts
          </button>
        </div>
      )}

      {/* QUICK START GUIDE */}
      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 border border-indigo-200">
        <h3 className="text-lg font-bold text-gray-900 mb-3">Quick Start</h3>
        <ul className="space-y-2 text-gray-700">
          <li>✅ <strong>Create Classroom</strong> - Set up a new class for your students</li>
          <li>✅ <strong>Invite Students</strong> - Share the invite code with your students</li>
          <li>✅ <strong>Monitor Progress</strong> - Track real-time learning with detailed analytics</li>
          <li>✅ <strong>Get Alerts</strong> - Automatically notified when students struggle</li>
        </ul>
      </div>
    </div>
  );
};

export default TeacherDashboardPage;
