// PATHFINDER Frontend - StudentRosterTable Component
// Display and manage classroom student roster

import React from 'react';
import { Trash2, AlertTriangle } from 'lucide-react';

interface Student {
  student_id: string;
  name: string;
  email: string;
  mastery_percent: number;
  skills_mastered: number;
  total_skills: number;
  current_skill: string;
  last_activity: string;
  status: string;
  joined_at: string;
}

interface StudentRosterTableProps {
  students: Student[];
  onRemoveStudent: (studentId: string) => void;
}

const StudentRosterTable: React.FC<StudentRosterTableProps> = ({
  students,
  onRemoveStudent,
}) => {
  const formatDate = (dateString: string) => {
    if (!dateString) return 'N/A';
    const date = new Date(dateString);
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'struggling':
        return 'bg-red-100 text-red-800';
      case 'inactive':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-blue-100 text-blue-800';
    }
  };

  const getStatusIcon = (status: string) => {
    return status === 'struggling' ? (
      <AlertTriangle className="text-red-600" size={16} />
    ) : null;
  };

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="bg-gray-50 border-b">
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Student Name
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Email
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Mastery
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Current Skill
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Last Activity
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Status
            </th>
            <th className="px-6 py-3 text-left text-sm font-semibold text-gray-900">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          {students.map((student) => (
            <tr
              key={student.student_id}
              className="border-b hover:bg-gray-50 transition"
            >
              {/* Name */}
              <td className="px-6 py-4">
                <p className="font-medium text-gray-900">{student.name}</p>
              </td>

              {/* Email */}
              <td className="px-6 py-4">
                <p className="text-sm text-gray-600">{student.email}</p>
              </td>

              {/* Mastery */}
              <td className="px-6 py-4">
                <div className="flex items-center gap-2">
                  <div className="w-20 bg-gray-200 rounded-full h-2">
                    <div
                      className={`h-2 rounded-full ${
                        student.mastery_percent >= 85
                          ? 'bg-green-500'
                          : student.mastery_percent >= 50
                          ? 'bg-blue-500'
                          : 'bg-red-500'
                      }`}
                      style={{ width: `${student.mastery_percent}%` }}
                    ></div>
                  </div>
                  <span className="text-sm font-semibold text-gray-900">
                    {Math.round(student.mastery_percent)}%
                  </span>
                </div>
              </td>

              {/* Current Skill */}
              <td className="px-6 py-4">
                <p className="text-sm text-gray-700">
                  {student.current_skill || 'Not started'}
                </p>
              </td>

              {/* Last Activity */}
              <td className="px-6 py-4">
                <p className="text-sm text-gray-600">
                  {student.last_activity ? formatDate(student.last_activity) : 'Never'}
                </p>
              </td>

              {/* Status */}
              <td className="px-6 py-4">
                <div
                  className={`inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs font-semibold ${getStatusColor(
                    student.status
                  )}`}
                >
                  {getStatusIcon(student.status)}
                  <span className="capitalize">{student.status}</span>
                </div>
              </td>

              {/* Actions */}
              <td className="px-6 py-4">
                <button
                  onClick={() => onRemoveStudent(student.student_id)}
                  className="p-2 text-red-600 hover:bg-red-50 rounded-lg transition"
                  title="Remove student"
                >
                  <Trash2 size={18} />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Summary */}
      <div className="px-6 py-4 bg-gray-50 border-t">
        <p className="text-sm text-gray-600">
          Showing {students.length} student{students.length !== 1 ? 's' : ''}
        </p>
      </div>
    </div>
  );
};

export default StudentRosterTable;
