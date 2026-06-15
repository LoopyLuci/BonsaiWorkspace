// PATHFINDER Frontend - ClassroomManagementPage
// Manage students, settings, and invite codes

import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import StudentRosterTable from '../components/StudentRosterTable';
import LoadingSpinner from '../components/LoadingSpinner';
import { Copy, Settings, Users, Mail, Code } from 'lucide-react';

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

interface ClassroomSettings {
  allow_peer_learning: boolean;
  show_leaderboard: boolean;
  parent_access: boolean;
  mastery_threshold: number;
}

const ClassroomManagementPage: React.FC = () => {
  const { classroomId } = useParams<{ classroomId: string }>();
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [classroom, setClassroom] = useState<any>(null);
  const [students, setStudents] = useState<Student[]>([]);
  const [classroomName, setClassroomName] = useState('');
  const [classroomDesc, setClassroomDesc] = useState('');
  const [settings, setSettings] = useState<ClassroomSettings>({
    allow_peer_learning: true,
    show_leaderboard: true,
    parent_access: true,
    mastery_threshold: 0.85,
  });
  const [isSaving, setIsSaving] = useState(false);
  const [copied, setCopied] = useState(false);

  // Load classroom data
  useEffect(() => {
    const loadClassroom = async () => {
      if (!classroomId || !user) return;

      try {
        setIsLoading(true);

        // Get classroom details
        const classroomRes = await apiClient.get(`/v1/teachers/classrooms/${classroomId}`, {
          headers: { 'X-User-ID': user.id },
        });
        setClassroom(classroomRes.data);
        setClassroomName(classroomRes.data.name);
        setClassroomDesc(classroomRes.data.description);
        if (classroomRes.data.settings) {
          setSettings(classroomRes.data.settings);
        }

        // Get student roster
        const studentsRes = await apiClient.get(`/v1/teachers/classrooms/${classroomId}/students`, {
          headers: { 'X-User-ID': user.id },
        });
        setStudents(studentsRes.data.students || []);
      } catch (error) {
        console.error('Failed to load classroom:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load classroom',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadClassroom();
  }, [classroomId, user, dispatch]);

  // Save classroom changes
  const handleSaveChanges = async () => {
    if (!classroom || !user) return;

    try {
      setIsSaving(true);

      await apiClient.put(`/v1/teachers/classrooms/${classroomId}`, {
        name: classroomName,
        description: classroomDesc,
        settings,
      }, {
        headers: { 'X-User-ID': user.id },
      });

      dispatch(
        uiActions.showNotification({
          message: 'Classroom settings updated',
          type: 'success',
        })
      );
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to save changes',
          type: 'error',
        })
      );
    } finally {
      setIsSaving(false);
    }
  };

  // Copy invite code
  const handleCopyInvite = () => {
    if (classroom?.invite_code) {
      navigator.clipboard.writeText(classroom.invite_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      dispatch(
        uiActions.showNotification({
          message: 'Invite code copied to clipboard',
          type: 'success',
        })
      );
    }
  };

  // Remove student
  const handleRemoveStudent = async (studentId: string) => {
    if (!classroomId || !user) return;

    if (!window.confirm('Remove this student from the classroom?')) {
      return;
    }

    try {
      await apiClient.delete(`/v1/teachers/classrooms/${classroomId}/students/${studentId}`, {
        headers: { 'X-User-ID': user.id },
      });

      setStudents(students.filter(s => s.student_id !== studentId));

      dispatch(
        uiActions.showNotification({
          message: 'Student removed from classroom',
          type: 'success',
        })
      );
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to remove student',
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

  if (!classroom) {
    return <div className="text-center py-12">Classroom not found</div>;
  }

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">{classroom.name}</h1>
        <p className="text-indigo-100">
          {classroom.subject} • {classroom.grade_level}
        </p>
      </div>

      {/* CLASSROOM SETTINGS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center gap-3 mb-6">
          <Settings size={24} className="text-indigo-600" />
          <h2 className="text-2xl font-bold text-gray-900">Classroom Settings</h2>
        </div>

        <div className="space-y-4">
          {/* Classroom Name */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Classroom Name
            </label>
            <input
              type="text"
              value={classroomName}
              onChange={(e) => setClassroomName(e.target.value)}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
              disabled={isSaving}
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Description
            </label>
            <textarea
              value={classroomDesc}
              onChange={(e) => setClassroomDesc(e.target.value)}
              rows={3}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
              disabled={isSaving}
            />
          </div>

          {/* Settings Checkboxes */}
          <div className="space-y-3 pt-4 border-t">
            <label className="flex items-center gap-3">
              <input
                type="checkbox"
                checked={settings.allow_peer_learning}
                onChange={(e) => setSettings({
                  ...settings,
                  allow_peer_learning: e.target.checked,
                })}
                className="w-4 h-4"
                disabled={isSaving}
              />
              <span className="text-sm font-medium text-gray-700">Allow peer learning</span>
            </label>

            <label className="flex items-center gap-3">
              <input
                type="checkbox"
                checked={settings.show_leaderboard}
                onChange={(e) => setSettings({
                  ...settings,
                  show_leaderboard: e.target.checked,
                })}
                className="w-4 h-4"
                disabled={isSaving}
              />
              <span className="text-sm font-medium text-gray-700">Show leaderboard to students</span>
            </label>

            <label className="flex items-center gap-3">
              <input
                type="checkbox"
                checked={settings.parent_access}
                onChange={(e) => setSettings({
                  ...settings,
                  parent_access: e.target.checked,
                })}
                className="w-4 h-4"
                disabled={isSaving}
              />
              <span className="text-sm font-medium text-gray-700">Allow parent access</span>
            </label>
          </div>

          {/* Save Button */}
          <button
            onClick={handleSaveChanges}
            disabled={isSaving}
            className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition"
          >
            {isSaving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>

      {/* INVITE CODE */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center gap-3 mb-6">
          <Code size={24} className="text-green-600" />
          <h2 className="text-2xl font-bold text-gray-900">Invite Students</h2>
        </div>

        <p className="text-gray-700 mb-4">
          Share this code with students to join your classroom
        </p>

        <div className="flex gap-3">
          <div className="flex-1 bg-gray-100 rounded-lg p-4 font-mono text-lg font-bold text-gray-900">
            {classroom.invite_code}
          </div>
          <button
            onClick={handleCopyInvite}
            className={`px-4 py-2 rounded-lg font-semibold transition flex items-center gap-2 ${
              copied
                ? 'bg-green-600 text-white'
                : 'bg-indigo-600 hover:bg-indigo-700 text-white'
            }`}
          >
            <Copy size={20} />
            {copied ? 'Copied!' : 'Copy'}
          </button>
        </div>

        <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
          <p className="text-sm text-blue-700">
            💡 Students can use this code to join your classroom from their dashboard
          </p>
        </div>
      </div>

      {/* STUDENT ROSTER */}
      <div>
        <div className="flex items-center gap-3 mb-6">
          <Users size={24} className="text-blue-600" />
          <h2 className="text-2xl font-bold text-gray-900">Student Roster</h2>
          <span className="ml-auto text-lg font-semibold text-gray-600">
            {students.length} students
          </span>
        </div>

        {students.length > 0 ? (
          <StudentRosterTable
            students={students}
            onRemoveStudent={handleRemoveStudent}
          />
        ) : (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <Users className="mx-auto text-gray-400 mb-3" size={40} />
            <p className="text-gray-600 mb-4">No students in this classroom yet</p>
            <p className="text-sm text-gray-500">
              Share the invite code above for students to join
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

export default ClassroomManagementPage;
