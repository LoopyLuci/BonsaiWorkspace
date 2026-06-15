// PATHFINDER Frontend - LessonPage
// Lesson flow and exercise sequencing

import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { ChevronRight, CheckCircle2, Clock } from 'lucide-react';

interface Exercise {
  id: string;
  title: string;
  type: string;
  difficulty: number;
}

interface Lesson {
  id: string;
  title: string;
  description: string;
  learning_objectives: string[];
  exercises: Exercise[];
}

const LessonPage: React.FC = () => {
  const navigate = useNavigate();
  const { skillId, lessonId } = useParams<{ skillId: string; lessonId: string }>();
  const [lesson, setLesson] = useState<Lesson | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [currentExerciseIndex, setCurrentExerciseIndex] = useState(0);
  const [completedExercises, setCompletedExercises] = useState<Set<string>>(new Set());

  useEffect(() => {
    const loadLesson = async () => {
      if (!lessonId) return;
      try {
        setIsLoading(true);
        const lessonData = await apiClient.getLesson(lessonId);
        setLesson(lessonData);
      } catch (error) {
        console.error('Failed to load lesson:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadLesson();
  }, [lessonId]);

  if (isLoading || !lesson) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  const currentExercise = lesson.exercises[currentExerciseIndex];
  const totalExercises = lesson.exercises.length;
  const completionPercent = (completedExercises.size / totalExercises) * 100;

  const handleStartExercise = () => {
    navigate(`/exercises/${currentExercise.id}`);
  };

  const handleExerciseComplete = (exerciseId: string) => {
    const newCompleted = new Set(completedExercises);
    newCompleted.add(exerciseId);
    setCompletedExercises(newCompleted);

    // Auto-advance to next exercise
    if (currentExerciseIndex < totalExercises - 1) {
      setCurrentExerciseIndex(currentExerciseIndex + 1);
    }
  };

  const handleNext = () => {
    if (currentExerciseIndex < totalExercises - 1) {
      setCurrentExerciseIndex(currentExerciseIndex + 1);
    }
  };

  const handlePrevious = () => {
    if (currentExerciseIndex > 0) {
      setCurrentExerciseIndex(currentExerciseIndex - 1);
    }
  };

  const handleBackToSkill = () => {
    navigate(`/skills/${skillId}`);
  };

  return (
    <div className="space-y-6">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-6 text-white">
        <h1 className="text-3xl font-bold mb-2">{lesson.title}</h1>
        <p className="text-indigo-100">{lesson.description}</p>
      </div>

      {/* LEARNING OBJECTIVES */}
      {lesson.learning_objectives && lesson.learning_objectives.length > 0 && (
        <div className="bg-white rounded-lg p-6 shadow">
          <h2 className="text-lg font-bold text-gray-900 mb-4">Learning Objectives</h2>
          <ul className="space-y-2">
            {lesson.learning_objectives.map((objective, idx) => (
              <li key={idx} className="flex items-start gap-3">
                <span className="text-indigo-600 mt-1">✓</span>
                <span className="text-gray-700">{objective}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* PROGRESS BAR */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center justify-between mb-3">
          <span className="font-semibold text-gray-900">
            Exercise {currentExerciseIndex + 1} of {totalExercises}
          </span>
          <span className="text-sm text-gray-600">
            {completedExercises.size} completed
          </span>
        </div>
        <div className="w-full bg-gray-200 rounded-full h-3">
          <div
            className="bg-indigo-600 h-3 rounded-full transition-all duration-300"
            style={{ width: `${completionPercent}%` }}
          ></div>
        </div>
      </div>

      {/* CURRENT EXERCISE */}
      <div className="bg-white rounded-lg p-8 shadow">
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-gray-900 mb-2">
            {currentExercise.title}
          </h2>
          <div className="flex items-center gap-4 text-gray-600">
            <span className="text-sm bg-gray-100 px-3 py-1 rounded-full">
              {currentExercise.type}
            </span>
            <span className="text-sm flex items-center gap-1">
              <Clock size={16} />
              {currentExercise.difficulty === 1 && '1-2 min'}
              {currentExercise.difficulty === 2 && '2-3 min'}
              {currentExercise.difficulty === 3 && '3-5 min'}
            </span>
          </div>
        </div>

        {completedExercises.has(currentExercise.id) ? (
          <div className="bg-green-50 border border-green-200 rounded-lg p-6 text-center">
            <CheckCircle2 className="text-green-600 mx-auto mb-3" size={48} />
            <p className="text-lg font-semibold text-green-900 mb-4">
              Exercise Complete! 🎉
            </p>
            <p className="text-green-700 mb-6">
              Great job! You've mastered this exercise.
            </p>
            {currentExerciseIndex < totalExercises - 1 && (
              <button
                onClick={handleNext}
                className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold rounded-lg flex items-center gap-2 mx-auto"
              >
                Next Exercise <ChevronRight size={20} />
              </button>
            )}
          </div>
        ) : (
          <div className="space-y-4">
            <p className="text-gray-700">
              Ready to practice? Click the button below to start this exercise.
            </p>
            <button
              onClick={handleStartExercise}
              className="w-full py-4 bg-indigo-600 hover:bg-indigo-700 text-white font-bold rounded-lg flex items-center justify-center gap-2"
            >
              Start Exercise <ChevronRight size={20} />
            </button>
          </div>
        )}
      </div>

      {/* EXERCISE LIST */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h3 className="text-lg font-bold text-gray-900 mb-4">All Exercises</h3>
        <div className="space-y-2">
          {lesson.exercises.map((exercise, idx) => (
            <button
              key={exercise.id}
              onClick={() => setCurrentExerciseIndex(idx)}
              className={`w-full text-left p-3 rounded-lg border-2 transition ${
                currentExerciseIndex === idx
                  ? 'border-indigo-600 bg-indigo-50'
                  : 'border-gray-300 hover:border-indigo-400 bg-white'
              }`}
            >
              <div className="flex items-center gap-3">
                {completedExercises.has(exercise.id) ? (
                  <CheckCircle2 className="text-green-600" size={20} />
                ) : (
                  <div className="w-5 h-5 rounded-full border-2 border-gray-300 flex-shrink-0"></div>
                )}
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-gray-900">{exercise.title}</p>
                  <p className="text-xs text-gray-500 capitalize">
                    {exercise.type.replace('_', ' ')}
                  </p>
                </div>
                {currentExerciseIndex === idx && (
                  <span className="text-xs bg-indigo-600 text-white px-2 py-1 rounded">
                    Current
                  </span>
                )}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* NAVIGATION */}
      <div className="flex gap-3">
        <button
          onClick={handleBackToSkill}
          className="flex-1 px-6 py-3 border border-gray-300 rounded-lg font-semibold text-gray-700 hover:bg-gray-50"
        >
          Back to Skill
        </button>
        <button
          onClick={handlePrevious}
          disabled={currentExerciseIndex === 0}
          className="flex-1 px-6 py-3 border border-gray-300 rounded-lg font-semibold text-gray-700 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Previous
        </button>
        <button
          onClick={handleNext}
          disabled={currentExerciseIndex === totalExercises - 1}
          className="flex-1 px-6 py-3 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg disabled:cursor-not-allowed"
        >
          Next
        </button>
      </div>

      {/* COMPLETION MESSAGE */}
      {completedExercises.size === totalExercises && (
        <div className="bg-green-50 border border-green-200 rounded-lg p-6 text-center">
          <h3 className="text-2xl font-bold text-green-900 mb-2">Lesson Complete! 🎓</h3>
          <p className="text-green-700 mb-4">
            Excellent work! You've mastered all exercises in this lesson.
          </p>
          <button
            onClick={handleBackToSkill}
            className="px-6 py-2 bg-green-600 hover:bg-green-700 text-white font-semibold rounded-lg"
          >
            Return to Skill
          </button>
        </div>
      )}
    </div>
  );
};

export default LessonPage;
