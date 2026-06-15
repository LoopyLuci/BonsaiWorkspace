// PATHFINDER Frontend - ParentDashboardPage
// Parent/Guardian dashboard showing linked children and their progress

import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { Users, TrendingUp, Target, AlertCircle, Link as LinkIcon } from 'lucide-react';

interface LinkedChild {
  id: string;
  student_id: string;
  student_name: string;
  student_email: string;
  relationship: string;
  verified: boolean;
}

interface ChildProgress {
  student_id: string;
  student_name: string;
  mastery_percent: number;
  skills_mastered: number;
  total_skills: number;
  current_skill: string;
  last_activity: string;
  status: string;
  exercises_today: number;
  accuracy_today: number;
  streak_days: number;
}

const ParentDashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [children, setChildren] = useState<LinkedChild[]>([]);
  const [childProgress, setChildProgress] = useState<Record<string, ChildProgress>>({});
  const [showLinkModal, setShowLinkModal] = useState(false);
  const [linkEmail, setLinkEmail] = useState('');
  const [isLinking, setIsLinking] = useState(false);

  // Load linked children
  useEffect(() => {
    const loadChildren = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        const response = await apiClient.get('/v1/parents/children', {
          headers: { 'X-User-ID': user.id },
        });

        const linkedChildren = response.data.children || [];
        setChildren(linkedChildren);

        // Load progress for each child
        const progress: Record<string, ChildProgress> = {};
        for (const child of linkedChildren) {
          try {
            const progressRes = await apiClient.get(
              `/v1/parents/children/${child.student_id}/progress`,
              { headers: { 'X-User-ID': user.id } }
            );
            progress[child.student_id] = progressRes.data;
          } catch (error) {
            console.error(`Failed to load progress for ${child.student_name}`);
          }
        }
        setChildProgress(progress);
      } catch (error) {
        console.error('Failed to load children:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load your children',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadChildren();
  }, [user, dispatch]);

  // Link child
  const handleLinkChild = async () => {
    if (!linkEmail.trim() || !user) return;

    try {
      setIsLinking(true);

      await apiClient.post(
        '/v1/parents/link-child',
        { student_email: linkEmail },
        { headers: { 'X-User-ID': user.id } }
      );

      dispatch(
        uiActions.showNotification({
          message: 'Verification code sent to your child',
          type: 'success',
        })
      );

      setLinkEmail('');
      setShowLinkModal(false);
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to link child',
          type: 'error',
        })
      );
    } finally {
      setIsLinking(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const totalMastery = children.length > 0
    ? Math.round(
        children.reduce((sum, child) => sum + (childProgress[child.student_id]?.mastery_percent || 0), 0) /
          children.length
      )
    : 0;

  const strugglingCount = children.filter(
    (child) => childProgress[child.student_id]?.status === 'struggling'
  ).length;

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-blue-600 to-cyan-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Welcome, {user?.first_name}! 👨‍👩‍👧</h1>
        <p className="text-blue-100">
          Monitor your children's learning journey
        </p>
      </div>

      {/* QUICK STATS */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Children</span>
            <Users className="text-blue-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{children.length}</p>
          <p className="text-xs text-gray-500 mt-2">linked accounts</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Avg Mastery</span>
            <TrendingUp className="text-green-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{totalMastery}%</p>
          <p className="text-xs text-gray-500 mt-2">across all children</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Struggling</span>
            <AlertCircle className="text-orange-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{strugglingCount}</p>
          <p className="text-xs text-gray-500 mt-2">need support</p>
        </div>

        <button
          onClick={() => setShowLinkModal(true)}
          className="bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-700 hover:to-purple-700 rounded-lg p-6 text-white shadow transition flex flex-col items-center justify-center gap-2"
        >
          <LinkIcon size={24} />
          <span className="font-semibold">Link Child</span>
        </button>
      </div>

      {/* CHILDREN OVERVIEW */}
      {children.length > 0 ? (
        <div className="space-y-4">
          <h2 className="text-2xl font-bold text-gray-900">Your Children</h2>

          {children.map((child) => {
            const progress = childProgress[child.student_id];
            if (!progress) return null;

            const masteryPercent = Math.round(progress.mastery_percent);
            const statusColor =
              progress.status === 'excellent'
                ? 'text-green-600'
                : progress.status === 'developing'
                ? 'text-blue-600'
                : 'text-red-600';

            return (
              <div
                key={child.student_id}
                onClick={() => navigate(`/parent/children/${child.student_id}`)}
                className="bg-white rounded-lg p-6 shadow hover:shadow-lg transition cursor-pointer"
              >
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <h3 className="text-xl font-bold text-gray-900">{progress.student_name}</h3>
                    <p className="text-sm text-gray-600">{progress.current_skill || 'Getting started'}</p>
                  </div>
                  <span className={`text-sm font-bold px-3 py-1 rounded-full bg-gray-100 ${statusColor}`}>
                    {progress.status.charAt(0).toUpperCase() + progress.status.slice(1)}
                  </span>
                </div>

                <div className="grid grid-cols-4 gap-4">
                  {/* Mastery */}
                  <div>
                    <p className="text-xs text-gray-600 mb-1">Mastery</p>
                    <div className="flex items-center gap-2">
                      <div className="flex-1 bg-gray-200 rounded-full h-2">
                        <div
                          className={`h-2 rounded-full ${
                            masteryPercent >= 85 ? 'bg-green-500' : masteryPercent >= 50 ? 'bg-blue-500' : 'bg-orange-500'
                          }`}
                          style={{ width: `${masteryPercent}%` }}
                        ></div>
                      </div>
                      <span className="text-sm font-bold text-gray-900">{masteryPercent}%</span>
                    </div>
                  </div>

                  {/* Skills */}
                  <div>
                    <p className="text-xs text-gray-600 mb-1">Skills</p>
                    <p className="text-sm font-bold text-gray-900">
                      {progress.skills_mastered}/{progress.total_skills}
                    </p>
                  </div>

                  {/* Today */}
                  <div>
                    <p className="text-xs text-gray-600 mb-1">Today</p>
                    <p className="text-sm font-bold text-gray-900">
                      {progress.exercises_today} exercises
                    </p>
                  </div>

                  {/* Streak */}
                  <div>
                    <p className="text-xs text-gray-600 mb-1">Streak</p>
                    <p className="text-sm font-bold text-gray-900">
                      🔥 {progress.streak_days}d
                    </p>
                  </div>
                </div>

                <div className="mt-4 text-right">
                  <span className="text-sm text-indigo-600 font-semibold">
                    View Details →
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="text-center py-12 bg-gray-50 rounded-lg">
          <Users className="mx-auto text-gray-400 mb-3" size={40} />
          <p className="text-gray-600 mb-4">No children linked yet</p>
          <button
            onClick={() => setShowLinkModal(true)}
            className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg transition"
          >
            Link Your First Child
          </button>
        </div>
      )}

      {/* LINK CHILD MODAL */}
      {showLinkModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-8 max-w-md w-full mx-4">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">Link Your Child</h2>
            <p className="text-gray-700 mb-4">
              Enter your child's email address. They'll receive a verification code to confirm the connection.
            </p>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Child's Email
              </label>
              <input
                type="email"
                value={linkEmail}
                onChange={(e) => setLinkEmail(e.target.value)}
                placeholder="child@example.com"
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            <div className="flex gap-3">
              <button
                onClick={() => setShowLinkModal(false)}
                className="flex-1 px-4 py-2 border border-gray-300 rounded-lg font-semibold text-gray-700 hover:bg-gray-50 transition"
              >
                Cancel
              </button>
              <button
                onClick={handleLinkChild}
                disabled={!linkEmail.trim() || isLinking}
                className="flex-1 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition"
              >
                {isLinking ? 'Linking...' : 'Link Child'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* TIPS */}
      <div className="bg-gradient-to-r from-cyan-50 to-blue-50 rounded-lg p-6 border border-cyan-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4">Parent Tips</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>📊 Monitor Progress:</strong> Check your child's mastery level and skills
            learned.
          </li>
          <li>
            <strong>🎯 Celebrate Milestones:</strong> Watch for achievements like skill mastery
            and streaks.
          </li>
          <li>
            <strong>💬 Stay Connected:</strong> Receive notifications when your child reaches
            milestones or needs support.
          </li>
          <li>
            <strong>📈 See Learning Curves:</strong> Track how your child's knowledge probability
            improves over time.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default ParentDashboardPage;
