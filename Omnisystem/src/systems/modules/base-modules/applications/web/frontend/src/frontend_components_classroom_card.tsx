// PATHFINDER Frontend - ClassroomCard Component
// Card showing classroom summary and key metrics

import React from 'react';
import { Users, TrendingUp, AlertCircle, ChevronRight } from 'lucide-react';

interface ClassroomCardProps {
  classroom: {
    id: string;
    name: string;
    subject: string;
    grade_level: string;
    total_students?: number;
    active_students?: number;
    avg_mastery?: number;
    unresolved_alerts?: number;
    created_at: string;
  };
  onViewDetails: () => void;
}

const ClassroomCard: React.FC<ClassroomCardProps> = ({ classroom, onViewDetails }) => {
  const masteryPercent = Math.round((classroom.avg_mastery || 0) * 100);
  const hasAlerts = (classroom.unresolved_alerts || 0) > 0;

  return (
    <div
      onClick={onViewDetails}
      className="bg-white rounded-lg shadow hover:shadow-lg transition cursor-pointer overflow-hidden border-l-4 border-indigo-600"
    >
      {/* HEADER */}
      <div className="p-6 pb-4">
        <h3 className="text-lg font-bold text-gray-900 mb-1">{classroom.name}</h3>
        <p className="text-sm text-gray-600">
          {classroom.subject} • {classroom.grade_level}
        </p>
      </div>

      {/* METRICS */}
      <div className="px-6 pb-4 space-y-3">
        {/* Students */}
        <div className="flex items-center gap-3">
          <Users className="text-blue-500 flex-shrink-0" size={18} />
          <div className="flex-1">
            <p className="text-xs text-gray-600">Students</p>
            <p className="text-sm font-semibold text-gray-900">
              {classroom.total_students || 0} enrolled{' '}
              <span className="text-xs text-gray-500">
                ({classroom.active_students || 0} active)
              </span>
            </p>
          </div>
        </div>

        {/* Mastery */}
        <div className="flex items-center gap-3">
          <TrendingUp className="text-green-500 flex-shrink-0" size={18} />
          <div className="flex-1">
            <p className="text-xs text-gray-600">Class Mastery</p>
            <div className="flex items-center gap-2">
              <div className="flex-1 bg-gray-200 rounded-full h-2">
                <div
                  className={`h-2 rounded-full ${
                    masteryPercent >= 85
                      ? 'bg-green-500'
                      : masteryPercent >= 50
                      ? 'bg-blue-500'
                      : 'bg-orange-500'
                  }`}
                  style={{ width: `${masteryPercent}%` }}
                ></div>
              </div>
              <span className="text-sm font-semibold text-gray-900 w-10 text-right">
                {masteryPercent}%
              </span>
            </div>
          </div>
        </div>

        {/* Alerts */}
        {hasAlerts && (
          <div className="flex items-center gap-3 bg-red-50 p-3 rounded-lg">
            <AlertCircle className="text-red-600 flex-shrink-0" size={18} />
            <div className="flex-1">
              <p className="text-xs text-red-700 font-semibold">
                {classroom.unresolved_alerts} Alert{(classroom.unresolved_alerts || 0) !== 1 ? 's' : ''}
              </p>
              <p className="text-xs text-red-600">Students need attention</p>
            </div>
          </div>
        )}
      </div>

      {/* FOOTER - CTA */}
      <div className="px-6 py-4 bg-gradient-to-r from-indigo-50 to-purple-50 border-t flex items-center justify-between cursor-pointer hover:from-indigo-100 hover:to-purple-100 transition">
        <span className="text-sm font-semibold text-indigo-600">View Details</span>
        <ChevronRight className="text-indigo-600" size={20} />
      </div>
    </div>
  );
};

export default ClassroomCard;
