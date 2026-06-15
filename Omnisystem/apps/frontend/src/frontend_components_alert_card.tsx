// PATHFINDER Frontend - AlertCard Component
// Display individual student intervention alert

import React from 'react';
import { AlertTriangle, TrendingDown, MessageSquare, CheckCircle2, Clock } from 'lucide-react';

interface AlertCardProps {
  alert: {
    id: string;
    student_name: string;
    alert_type: string;
    skill_name?: string;
    message: string;
    severity: 'low' | 'medium' | 'high';
    p_know: number;
    days_since_progress: number;
    recommendation: string;
    resolved: boolean;
    created_at: string;
  };
  onDismiss: () => void;
}

const AlertCard: React.FC<AlertCardProps> = ({ alert, onDismiss }) => {
  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'high':
        return { bg: 'bg-red-50', border: 'border-red-200', badge: 'bg-red-100 text-red-800', icon: 'text-red-600' };
      case 'medium':
        return { bg: 'bg-orange-50', border: 'border-orange-200', badge: 'bg-orange-100 text-orange-800', icon: 'text-orange-600' };
      case 'low':
        return { bg: 'bg-yellow-50', border: 'border-yellow-200', badge: 'bg-yellow-100 text-yellow-800', icon: 'text-yellow-600' };
      default:
        return { bg: 'bg-gray-50', border: 'border-gray-200', badge: 'bg-gray-100 text-gray-800', icon: 'text-gray-600' };
    }
  };

  const colors = getSeverityColor(alert.severity);
  const formattedDate = new Date(alert.created_at).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });

  const pKnowPercent = Math.round(alert.p_know * 100);

  return (
    <div className={`${colors.bg} border-l-4 border-current rounded-lg p-6 ${colors.border}`}>
      {/* HEADER */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-start gap-3 flex-1">
          <AlertTriangle className={`${colors.icon} flex-shrink-0 mt-1`} size={24} />

          <div className="flex-1 min-w-0">
            <h3 className="text-lg font-bold text-gray-900 mb-1">{alert.student_name}</h3>
            <p className="text-sm text-gray-700">
              {alert.message}
              {alert.skill_name && (
                <span className="ml-2">
                  <strong>{alert.skill_name}</strong>
                </span>
              )}
            </p>
          </div>
        </div>

        <span className={`px-3 py-1 rounded-full text-xs font-bold whitespace-nowrap ml-2 ${colors.badge}`}>
          {alert.severity.toUpperCase()}
        </span>
      </div>

      {/* METRICS */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        {/* P(Know) */}
        <div>
          <p className="text-xs text-gray-600 mb-1">Knowledge Level</p>
          <div className="flex items-center gap-2">
            <div className="flex-1 bg-gray-200 rounded-full h-2">
              <div
                className={`h-2 rounded-full ${
                  pKnowPercent >= 50 ? 'bg-green-500' : pKnowPercent >= 30 ? 'bg-yellow-500' : 'bg-red-500'
                }`}
                style={{ width: `${pKnowPercent}%` }}
              ></div>
            </div>
            <span className="text-sm font-bold text-gray-900">{pKnowPercent}%</span>
          </div>
        </div>

        {/* Days Since Progress */}
        <div>
          <p className="text-xs text-gray-600 mb-1">No Progress For</p>
          <div className="flex items-center gap-2">
            <Clock size={16} className="text-gray-500" />
            <span className="text-sm font-bold text-gray-900">{alert.days_since_progress}d</span>
          </div>
        </div>

        {/* Alert Type */}
        <div>
          <p className="text-xs text-gray-600 mb-1">Alert Type</p>
          <p className="text-sm font-semibold text-gray-900 capitalize">{alert.alert_type}</p>
        </div>

        {/* Date */}
        <div>
          <p className="text-xs text-gray-600 mb-1">Detected</p>
          <p className="text-sm font-semibold text-gray-900">{formattedDate}</p>
        </div>
      </div>

      {/* RECOMMENDATION */}
      <div className="bg-white rounded-lg p-4 mb-4 border-l-4 border-blue-500">
        <h4 className="text-sm font-bold text-gray-900 flex items-center gap-2 mb-2">
          <MessageSquare size={16} className="text-blue-600" />
          Recommended Intervention
        </h4>
        <p className="text-sm text-gray-700">{alert.recommendation}</p>
      </div>

      {/* ACTIONS */}
      <div className="flex gap-3">
        <button className="flex-1 px-4 py-2 bg-white border border-gray-300 rounded-lg font-semibold text-gray-700 hover:bg-gray-50 transition flex items-center justify-center gap-2">
          <MessageSquare size={18} />
          Message Student
        </button>

        {!alert.resolved && (
          <button
            onClick={onDismiss}
            className="flex-1 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg transition flex items-center justify-center gap-2"
          >
            <CheckCircle2 size={18} />
            Mark Resolved
          </button>
        )}

        {alert.resolved && (
          <div className="flex-1 px-4 py-2 bg-green-50 border border-green-200 rounded-lg font-semibold text-green-700 flex items-center justify-center gap-2">
            <CheckCircle2 size={18} />
            Resolved
          </div>
        )}
      </div>

      {/* STATUS BADGE */}
      {alert.resolved && (
        <div className="mt-4 p-3 bg-green-50 border border-green-200 rounded-lg">
          <p className="text-sm text-green-700">
            ✓ This alert has been marked as resolved. Monitor the student's progress.
          </p>
        </div>
      )}
    </div>
  );
};

export default AlertCard;
