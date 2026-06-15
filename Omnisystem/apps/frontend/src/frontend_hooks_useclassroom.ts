// PATHFINDER Frontend - useClassroom Hook
// Custom hook for classroom management and analytics

import { useState, useCallback } from 'react';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import apiClient from '../api-client';

interface Classroom {
  id: string;
  name: string;
  description?: string;
  subject: string;
  grade_level: string;
  capacity: number;
  invite_code: string;
  settings?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

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

interface ClassProgress {
  classroom_id: string;
  total_students: number;
  active_students: number;
  avg_mastery: number;
  mastery_distribution: Record<string, number>;
  top_skills: Array<any>;
  struggling_skills: Array<any>;
  engagement: Record<string, any>;
}

interface UseClassroomReturn {
  // Data
  classrooms: Classroom[] | null;
  currentClassroom: Classroom | null;
  students: Student[] | null;
  progress: ClassProgress | null;

  // States
  isLoading: boolean;
  isCreating: boolean;
  isSaving: boolean;
  error: string | null;

  // Actions
  listClassrooms: () => Promise<void>;
  getClassroom: (classroomId: string) => Promise<void>;
  createClassroom: (data: any) => Promise<string>;
  updateClassroom: (classroomId: string, data: any) => Promise<void>;
  deleteClassroom: (classroomId: string) => Promise<void>;

  // Student management
  getStudents: (classroomId: string) => Promise<void>;
  addStudent: (classroomId: string, studentId: string) => Promise<void>;
  removeStudent: (classroomId: string, studentId: string) => Promise<void>;

  // Analytics
  getProgress: (classroomId: string) => Promise<void>;
  regenerateInviteCode: (classroomId: string) => Promise<string>;
}

export const useClassroom = (): UseClassroomReturn => {
  const { user } = useSelector((state: RootState) => state.auth);

  const [classrooms, setClassrooms] = useState<Classroom[] | null>(null);
  const [currentClassroom, setCurrentClassroom] = useState<Classroom | null>(null);
  const [students, setStudents] = useState<Student[] | null>(null);
  const [progress, setProgress] = useState<ClassProgress | null>(null);

  const [isLoading, setIsLoading] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const headers = { 'X-User-ID': user?.id || '' };

  // List all classrooms
  const listClassrooms = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      const response = await apiClient.get('/v1/teachers/classrooms', { headers });
      setClassrooms(response.data.classrooms || []);
    } catch (err: any) {
      const message = err.response?.data?.message || 'Failed to load classrooms';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [user?.id]);

  // Get single classroom
  const getClassroom = useCallback(
    async (classroomId: string) => {
      try {
        setIsLoading(true);
        setError(null);

        const response = await apiClient.get(
          `/v1/teachers/classrooms/${classroomId}`,
          { headers }
        );
        setCurrentClassroom(response.data);
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to load classroom';
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    [user?.id]
  );

  // Create classroom
  const createClassroom = useCallback(
    async (data: any) => {
      try {
        setIsCreating(true);
        setError(null);

        const response = await apiClient.post(
          '/v1/teachers/classrooms',
          data,
          { headers }
        );

        const newClassroom = response.data;
        setClassrooms(classrooms ? [...classrooms, newClassroom] : [newClassroom]);
        return newClassroom.id;
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to create classroom';
        setError(message);
        throw err;
      } finally {
        setIsCreating(false);
      }
    },
    [classrooms, user?.id]
  );

  // Update classroom
  const updateClassroom = useCallback(
    async (classroomId: string, data: any) => {
      try {
        setIsSaving(true);
        setError(null);

        const response = await apiClient.put(
          `/v1/teachers/classrooms/${classroomId}`,
          data,
          { headers }
        );

        setCurrentClassroom(response.data);

        // Update in list
        if (classrooms) {
          setClassrooms(
            classrooms.map((c) => (c.id === classroomId ? response.data : c))
          );
        }
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to update classroom';
        setError(message);
        throw err;
      } finally {
        setIsSaving(false);
      }
    },
    [classrooms, user?.id]
  );

  // Delete classroom
  const deleteClassroom = useCallback(
    async (classroomId: string) => {
      try {
        setIsSaving(true);
        setError(null);

        await apiClient.delete(`/v1/teachers/classrooms/${classroomId}`, {
          headers,
        });

        // Remove from list
        if (classrooms) {
          setClassrooms(classrooms.filter((c) => c.id !== classroomId));
        }

        if (currentClassroom?.id === classroomId) {
          setCurrentClassroom(null);
        }
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to delete classroom';
        setError(message);
        throw err;
      } finally {
        setIsSaving(false);
      }
    },
    [classrooms, currentClassroom, user?.id]
  );

  // Get students in classroom
  const getStudents = useCallback(
    async (classroomId: string) => {
      try {
        setIsLoading(true);
        setError(null);

        const response = await apiClient.get(
          `/v1/teachers/classrooms/${classroomId}/students`,
          { headers }
        );
        setStudents(response.data.students || []);
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to load students';
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    [user?.id]
  );

  // Add student
  const addStudent = useCallback(
    async (classroomId: string, studentId: string) => {
      try {
        setIsSaving(true);
        setError(null);

        await apiClient.post(
          `/v1/teachers/classrooms/${classroomId}/students`,
          { student_id: studentId },
          { headers }
        );

        // Refresh students
        await getStudents(classroomId);
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to add student';
        setError(message);
        throw err;
      } finally {
        setIsSaving(false);
      }
    },
    [user?.id, getStudents]
  );

  // Remove student
  const removeStudent = useCallback(
    async (classroomId: string, studentId: string) => {
      try {
        setIsSaving(true);
        setError(null);

        await apiClient.delete(
          `/v1/teachers/classrooms/${classroomId}/students/${studentId}`,
          { headers }
        );

        // Update students list
        if (students) {
          setStudents(students.filter((s) => s.student_id !== studentId));
        }
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to remove student';
        setError(message);
        throw err;
      } finally {
        setIsSaving(false);
      }
    },
    [students, user?.id]
  );

  // Get class progress
  const getProgress = useCallback(
    async (classroomId: string) => {
      try {
        setIsLoading(true);
        setError(null);

        const response = await apiClient.get(
          `/v1/teachers/classrooms/${classroomId}/progress`,
          { headers }
        );
        setProgress(response.data);
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to load progress';
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    [user?.id]
  );

  // Regenerate invite code
  const regenerateInviteCode = useCallback(
    async (classroomId: string) => {
      try {
        setIsSaving(true);
        setError(null);

        const response = await apiClient.post(
          `/v1/teachers/classrooms/${classroomId}/regenerate-invite`,
          {},
          { headers }
        );

        const newCode = response.data.invite_code;

        // Update current classroom
        if (currentClassroom?.id === classroomId) {
          setCurrentClassroom({
            ...currentClassroom,
            invite_code: newCode,
          });
        }

        return newCode;
      } catch (err: any) {
        const message = err.response?.data?.message || 'Failed to regenerate code';
        setError(message);
        throw err;
      } finally {
        setIsSaving(false);
      }
    },
    [currentClassroom, user?.id]
  );

  return {
    // Data
    classrooms,
    currentClassroom,
    students,
    progress,

    // States
    isLoading,
    isCreating,
    isSaving,
    error,

    // Actions
    listClassrooms,
    getClassroom,
    createClassroom,
    updateClassroom,
    deleteClassroom,
    getStudents,
    addStudent,
    removeStudent,
    getProgress,
    regenerateInviteCode,
  };
};
