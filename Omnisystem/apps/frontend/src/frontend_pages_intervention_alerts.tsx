// PATHFINDER Frontend - InterventionAlertsPage
// Monitor and manage struggling students

import React, { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import AlertCard from '../components/AlertCard';
import LoadingSpinner from '../components/LoadingSpinner';
import { AlertTriangle, MessageCircle, CheckCircle2, Clock, TrendingDown } from 'lucide-react';

interface Alert {
  id: string;
  classroom_id: string;
  student_id: string;
  student_name: string;
  alert_type: string;
  skill_id?: string;
  skill_name?: string;
  message: string;
  severity: 'low' | 'medium' | 'high';
  p_know: number;
  days_since_progress: number;
  recommendation: string;
  resolved: boolean;
  created_at: string;
  resolved_at?: string;
}

const InterventionAlertsPage: React.FC = () => {
  const { classroomId } = useParams<{ classroomId: string }>();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [filterSeverity, setFilterSeverity] = useState<'all' | 'high' | 'medium' | 'low'>('all');
  const [filterResolved, setFilterResolved] = useState<'unresolved' | 'resolved' | 'all'>('unresolved');

  // Load alerts
  useEffect(() => {
    const loadAlerts = async () => {
      if (!classroomId || !user) return;

      try {
        setIsLoading(true);

        const response = await apiClient.get(`/v1/teachers/classrooms/${classroomId}/alerts`, {
          headers: { 'X-User-ID': user.id },
        });

        setAlerts(response.data.alerts || []);
      } catch (error) {
        console.error('Failed to load alerts:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load alerts',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadAlerts();
  }, [classroomId, user, dispatch]);

  // Dismiss alert
  const handleDismissAlert = async (alertId: string) => {
    if (!classroomId || !user) return;

    try {
      await apiClient.post(`/v1/teachers/classrooms/${classroomId}/alerts/${alertId}/dismiss`, {}, {
        headers: { 'X-User-ID': user.id },
      });

      setAlerts(alerts.map(a => a.id === alertId ? { ...a, resolved: true } : a));

      dispatch(
        uiActions.showNotification({
          message: 'Alert marked as resolved',
          type: 'success',
        })
      );
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to dismiss alert',
          type: 'error',
        })
      );
    }
  };

  // Filter alerts
  const filteredAlerts = alerts.filter((alert) => {
    const severityMatch = filterSeverity === 'all' || alert.severity === filterSeverity;
    const resolvedMatch =
      filterResolved === 'all' ||
      (filterResolved === 'unresolved' && !alert.resolved) ||
      (filterResolved === 'resolved' && alert.resolved);

    return severityMatch && resolvedMatch;
  });

  // Count by severity
  const highCount = alerts.filter(a => a.severity === 'high' && !a.resolved).length;
  const mediumCount = alerts.filter(a => a.severity === 'medium' && !a.resolved).length;
  const lowCount = alerts.filter(a => a.severity === 'low' && !a.resolved).length;

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
      <div className="bg-gradient-to-r from-red-600 to-orange-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Student Alerts</h1>
        <p className="text-red-100">
          Monitor and support students who need intervention
        </p>
      </div>

      {/* ALERT SUMMARY */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg p-6 shadow border-l-4 border-red-500">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">High Priority</span>
            <AlertTriangle className="text-red-600" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{highCount}</p>
          <p className="text-xs text-gray-500 mt-2">Require immediate attention</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow border-l-4 border-orange-500">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Medium Priority</span>
            <TrendingDown className="text-orange-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{mediumCount}</p>
          <p className="text-xs text-gray-500 mt-2">Monitor closely</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow border-l-4 border-yellow-500">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Low Priority</span>
            <Clock className="text-yellow-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">{lowCount}</p>
          <p className="text-xs text-gray-500 mt-2">Watch for improvement</p>
        </div>

        <div className="bg-white rounded-lg p-6 shadow">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-600">Total</span>
            <CheckCircle2 className="text-green-500" size={20} />
          </div>
          <p className="text-3xl font-bold text-gray-900">
            {alerts.filter(a => !a.resolved).length}
          </p>
          <p className="text-xs text-gray-500 mt-2">Unresolved alerts</p>
        </div>
      </div>

      {/* FILTERS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-lg font-bold text-gray-900 mb-4">Filter Alerts</h2>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Severity Filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Severity Level
            </label>
            <div className="flex gap-2 flex-wrap">
              {(['all', 'high', 'medium', 'low'] as const).map((level) => (
                <button
                  key={level}
                  onClick={() => setFilterSeverity(level)}
                  className={`px-4 py-2 rounded-lg font-medium transition ${
                    filterSeverity === level
                      ? 'bg-indigo-600 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {level === 'all' ? 'All' : level.charAt(0).toUpperCase() + level.slice(1)}
                </button>
              ))}
            </div>
          </div>

          {/* Status Filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Status
            </label>
            <div className="flex gap-2 flex-wrap">
              {(['all', 'unresolved', 'resolved'] as const).map((status) => (
                <button
                  key={status}
                  onClick={() => setFilterResolved(status)}
                  className={`px-4 py-2 rounded-lg font-medium transition ${
                    filterResolved === status
                      ? 'bg-indigo-600 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {status === 'all' ? 'All' : status.charAt(0).toUpperCase() + status.slice(1)}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* ALERTS LIST */}
      <div>
        <h2 className="text-xl font-bold text-gray-900 mb-6">
          {filteredAlerts.length > 0 ? `${filteredAlerts.length} Alert${filteredAlerts.length !== 1 ? 's' : ''}` : 'No Alerts'}
        </h2>

        {filteredAlerts.length > 0 ? (
          <div className="space-y-4">
            {filteredAlerts.map((alert) => (
              <AlertCard
                key={alert.id}
                alert={alert}
                onDismiss={() => handleDismissAlert(alert.id)}
              />
            ))}
          </div>
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <CheckCircle2 className="mx-auto text-green-500 mb-3" size={48} />
            <p className="text-gray-600 mb-2 text-lg font-semibold">All Students Doing Well!</p>
            <p className="text-sm text-gray-500">
              {filterResolved === 'unresolved'
                ? 'No students currently require intervention'
                : 'No alerts match the selected filters'}
            </p>
          </div>
        )}
      </div>

      {/* INTERVENTION TIPS */}
      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 border border-indigo-200">
        <h3 className="text-lg font-bold text-gray-900 mb-4">Intervention Tips</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>🎯 High Priority (P(Know) &lt; 30%):</strong> Student is struggling significantly.
            Consider one-on-one tutoring or additional practice.
          </li>
          <li>
            <strong>⚠️ Medium Priority (30-50%):</strong> Student is making slow progress. Provide
            encouragement and additional resources.
          </li>
          <li>
            <strong>📋 Low Priority (50-70%):</strong> Student is developing competency. Continue
            monitoring and support as needed.
          </li>
          <li>
            <strong>💡 Next Steps:</strong> Use the recommended intervention for each student to
            provide targeted support.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default InterventionAlertsPage;
