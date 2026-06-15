// PATHFINDER Frontend - StudyPlannerPage
// Personalized study schedule and session management

import React, { useEffect, useState } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { Calendar, Plus, CheckCircle, Clock, Target } from 'lucide-react';

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

const StudyPlannerPage: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [sessions, setSessions] = useState<StudySession[]>([]);
  const [showModal, setShowModal] = useState(false);
  const [newSession, setNewSession] = useState({
    skill_name: '',
    duration: 30,
    difficulty: 'medium',
    scheduled_for: new Date().toISOString().split('T')[0],
  });
  const [filter, setFilter] = useState<'scheduled' | 'completed' | 'all'>('scheduled');

  // Load study plan
  useEffect(() => {
    const loadPlan = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        const response = await apiClient.get(
          `/v1/insights/study-plan?status=${filter}`,
          { headers: { 'X-User-ID': user.id } }
        );

        setSessions(response.data.sessions || []);
      } catch (error) {
        console.error('Failed to load study plan:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load study plan',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadPlan();
  }, [user, filter, dispatch]);

  const handleCreateSession = async () => {
    if (!user || !newSession.skill_name.trim()) return;

    try {
      await apiClient.post(
        '/v1/insights/study-plan',
        {
          ...newSession,
          scheduled_for: new Date(newSession.scheduled_for).toISOString(),
        },
        { headers: { 'X-User-ID': user.id } }
      );

      dispatch(
        uiActions.showNotification({
          message: 'Study session created!',
          type: 'success',
        })
      );

      setNewSession({
        skill_name: '',
        duration: 30,
        difficulty: 'medium',
        scheduled_for: new Date().toISOString().split('T')[0],
      });
      setShowModal(false);

      // Reload
      const response = await apiClient.get(
        `/v1/insights/study-plan?status=${filter}`,
        { headers: { 'X-User-ID': user.id } }
      );
      setSessions(response.data.sessions || []);
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to create session',
          type: 'error',
        })
      );
    }
  };

  const handleCompleteSession = async (sessionId: string) => {
    if (!user) return;

    try {
      await apiClient.put(
        `/v1/insights/study-plan/update?id=${sessionId}`,
        { status: 'completed' },
        { headers: { 'X-User-ID': user.id } }
      );

      dispatch(
        uiActions.showNotification({
          message: 'Session marked as complete!',
          type: 'success',
        })
      );

      // Reload
      const response = await apiClient.get(
        `/v1/insights/study-plan?status=${filter}`,
        { headers: { 'X-User-ID': user.id } }
      );
      setSessions(response.data.sessions || []);
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to update session',
          type: 'error',
        })
      );
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const upcomingSessions = sessions.filter(
    (s) => new Date(s.scheduled_for) >= new Date() && s.status === 'scheduled'
  );
  const completedSessions = sessions.filter((s) => s.status === 'completed');

  const totalMinutesPlanned = sessions.reduce(
    (sum, s) => sum + (s.status === 'scheduled' ? s.duration : 0),
    0
  );

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <div className="flex items-center gap-3 mb-2">
          <Calendar size={32} />
          <h1 className="text-3xl font-bold">Study Planner</h1>
        </div>
        <p className="text-indigo-100">
          Plan and track your personalized learning sessions
        </p>
      </div>

      {/* STATS */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Upcoming Sessions</p>
          <p className="text-3xl font-bold text-gray-900">{upcomingSessions.length}</p>
          <p className="text-xs text-gray-500 mt-2">
            {totalMinutesPlanned} minutes planned
          </p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Completed Today</p>
          <p className="text-3xl font-bold text-gray-900">
            {completedSessions.filter((s) => {
              const today = new Date().toDateString();
              const sessionDate = new Date(s.completed_at || '').toDateString();
              return today === sessionDate;
            }).length}
          </p>
          <p className="text-xs text-gray-500 mt-2">Keep it up!</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <p className="text-sm text-gray-600 mb-2">Total Completed</p>
          <p className="text-3xl font-bold text-gray-900">{completedSessions.length}</p>
          <p className="text-xs text-gray-500 mt-2">Great progress!</p>
        </div>
      </div>

      {/* FILTERS */}
      <div className="flex gap-2">
        {(['scheduled', 'completed', 'all'] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-4 py-2 rounded-lg font-semibold transition ${
              filter === f
                ? 'bg-indigo-600 text-white'
                : 'bg-white border border-gray-300 text-gray-700 hover:bg-gray-50'
            }`}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}

        <button
          onClick={() => setShowModal(true)}
          className="ml-auto px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg flex items-center gap-2"
        >
          <Plus size={18} />
          Add Session
        </button>
      </div>

      {/* STUDY SESSIONS */}
      <div className="space-y-4">
        {sessions.length > 0 ? (
          sessions.map((session) => {
            const isUpcoming = new Date(session.scheduled_for) >= new Date();
            const sessionDate = new Date(session.scheduled_for);
            const today = new Date();
            const isToday = sessionDate.toDateString() === today.toDateString();

            return (
              <div
                key={session.id}
                className={`rounded-lg p-6 border-l-4 ${
                  session.status === 'completed'
                    ? 'bg-green-50 border-green-500'
                    : isToday
                    ? 'bg-blue-50 border-blue-500'
                    : 'bg-white border-gray-300'
                }`}
              >
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="text-lg font-bold text-gray-900">
                      {session.skill_name}
                    </h3>
                    <p className="text-sm text-gray-600">
                      {sessionDate.toLocaleDateString(undefined, {
                        weekday: 'short',
                        month: 'short',
                        day: 'numeric',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </p>
                  </div>

                  {session.status === 'completed' ? (
                    <span className="text-green-700 font-bold flex items-center gap-1">
                      <CheckCircle size={20} />
                      Completed
                    </span>
                  ) : isToday ? (
                    <span className="text-blue-700 font-bold px-3 py-1 bg-blue-100 rounded-full">
                      Today
                    </span>
                  ) : (
                    <span className="text-gray-700 font-bold px-3 py-1 bg-gray-100 rounded-full">
                      Scheduled
                    </span>
                  )}
                </div>

                <div className="grid grid-cols-3 gap-4 mb-4">
                  <div>
                    <p className="text-xs text-gray-600 mb-1">Duration</p>
                    <p className="text-sm font-bold text-gray-900 flex items-center gap-1">
                      <Clock size={16} />
                      {session.duration}m
                    </p>
                  </div>

                  <div>
                    <p className="text-xs text-gray-600 mb-1">Difficulty</p>
                    <p
                      className={`text-sm font-bold capitalize px-2 py-1 rounded w-fit ${
                        session.difficulty === 'easy'
                          ? 'bg-green-100 text-green-800'
                          : session.difficulty === 'medium'
                          ? 'bg-yellow-100 text-yellow-800'
                          : 'bg-red-100 text-red-800'
                      }`}
                    >
                      {session.difficulty}
                    </p>
                  </div>

                  <div>
                    <p className="text-xs text-gray-600 mb-1">Status</p>
                    <p className="text-sm font-bold text-gray-900 capitalize">
                      {session.status}
                    </p>
                  </div>
                </div>

                {session.status === 'scheduled' && isUpcoming && (
                  <button
                    onClick={() => handleCompleteSession(session.id)}
                    className="w-full px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg transition"
                  >
                    Mark Complete
                  </button>
                )}
              </div>
            );
          })
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <Calendar className="text-gray-300 mx-auto mb-3" size={40} />
            <p className="text-gray-600 mb-4">No study sessions scheduled</p>
            <button
              onClick={() => setShowModal(true)}
              className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg"
            >
              Schedule Your First Session
            </button>
          </div>
        )}
      </div>

      {/* NEW SESSION MODAL */}
      {showModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">Schedule Study Session</h2>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Skill to Study
                </label>
                <input
                  type="text"
                  value={newSession.skill_name}
                  onChange={(e) =>
                    setNewSession({ ...newSession, skill_name: e.target.value })
                  }
                  placeholder="e.g., Fractions, Algebra, Biology"
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Duration (minutes)
                  </label>
                  <select
                    value={newSession.duration}
                    onChange={(e) =>
                      setNewSession({
                        ...newSession,
                        duration: parseInt(e.target.value),
                      })
                    }
                    className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    <option value="15">15m</option>
                    <option value="30">30m</option>
                    <option value="45">45m</option>
                    <option value="60">60m</option>
                    <option value="90">90m</option>
                  </select>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Difficulty
                  </label>
                  <select
                    value={newSession.difficulty}
                    onChange={(e) =>
                      setNewSession({
                        ...newSession,
                        difficulty: e.target.value,
                      })
                    }
                    className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    <option value="easy">Easy</option>
                    <option value="medium">Medium</option>
                    <option value="hard">Hard</option>
                  </select>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Scheduled For
                </label>
                <input
                  type="date"
                  value={newSession.scheduled_for}
                  onChange={(e) =>
                    setNewSession({
                      ...newSession,
                      scheduled_for: e.target.value,
                    })
                  }
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
              </div>
            </div>

            <div className="flex gap-3 mt-6">
              <button
                onClick={() => setShowModal(false)}
                className="flex-1 px-4 py-2 border border-gray-300 rounded-lg text-gray-700 font-semibold hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateSession}
                className="flex-1 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg"
              >
                Schedule
              </button>
            </div>
          </div>
        </div>
      )}

      {/* TIPS */}
      <div className="bg-indigo-50 border border-indigo-200 rounded-lg p-6">
        <h3 className="text-lg font-bold text-gray-900 mb-4">📅 Study Planning Tips</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>🎯 Be Specific:</strong> Schedule sessions for specific skills, not vague topics.
          </li>
          <li>
            <strong>⏰ Consistent Time:</strong> Schedule at the same time each day for better habits.
          </li>
          <li>
            <strong>📈 Progressive Difficulty:</strong> Start easy and increase difficulty as you improve.
          </li>
          <li>
            <strong>✅ Track Completion:</strong> Mark sessions complete to see your progress and stay motivated.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default StudyPlannerPage;
