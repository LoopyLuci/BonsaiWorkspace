// PATHFINDER Frontend - ExercisePage
// Critical: Where BKT + HLR algorithms engage
// This is where the learning magic happens!

import React, { useEffect, useState, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { learnerStateActions, uiActions } from '../store';
import apiClient from '../api-client';
import type { Exercise, ExerciseAttemptResponse } from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import ExerciseFeedback from '../components/ExerciseFeedback';
import { Clock, Volume2, CheckCircle, XCircle } from 'lucide-react';

const ExercisePage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { exerciseId } = useParams<{ exerciseId: string }>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [exercise, setExercise] = useState<Exercise | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [response, setResponse] = useState('');
  const [selectedOption, setSelectedOption] = useState<number | null>(null);
  const [startTime] = useState(Date.now());
  const [timeElapsed, setTimeElapsed] = useState(0);
  const [result, setResult] = useState<ExerciseAttemptResponse | null>(null);
  const [showFeedback, setShowFeedback] = useState(false);

  // Timer
  useEffect(() => {
    const interval = setInterval(() => {
      setTimeElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 100);
    return () => clearInterval(interval);
  }, [startTime]);

  // Load exercise
  useEffect(() => {
    const loadExercise = async () => {
      if (!exerciseId) return;
      try {
        setIsLoading(true);
        const ex = await apiClient.getExercise(exerciseId);
        setExercise(ex);
      } catch (error) {
        console.error('Failed to load exercise:', error);
        dispatch(
          uiActions.showNotification({
            message: 'Failed to load exercise',
            type: 'error',
          })
        );
      } finally {
        setIsLoading(false);
      }
    };
    loadExercise();
  }, [exerciseId, dispatch]);

  // Submit answer
  const handleSubmit = async () => {
    if (!exercise || !user) return;

    // Determine if correct
    let wasCorrect = false;
    let submittedResponse = '';

    if (exercise.type === 'multiple_choice') {
      if (selectedOption === null) {
        dispatch(
          uiActions.showNotification({
            message: 'Please select an answer',
            type: 'error',
          })
        );
        return;
      }
      wasCorrect = selectedOption === exercise.correct_option;
      submittedResponse = `Option ${selectedOption}`;
    } else {
      if (!response.trim()) {
        dispatch(
          uiActions.showNotification({
            message: 'Please enter a response',
            type: 'error',
          })
        );
        return;
      }
      submittedResponse = response;
      // For translation: simple check (in production: fuzzy matching)
      wasCorrect = submittedResponse.toLowerCase().trim() === 'hola'; // Example
    }

    try {
      setIsSubmitting(true);

      // Submit to backend - BKT + HLR ENGAGE HERE!
      const attemptResult = await apiClient.recordExerciseAttempt(
        user.id,
        exercise.id,
        exercise.skill_id,
        wasCorrect,
        submittedResponse,
        timeElapsed
      );

      setResult(attemptResult);
      setShowFeedback(true);

      // Update Redux with new skill state
      dispatch(learnerStateActions.updateSkillState(attemptResult.skill_state));

      // Show success notification
      dispatch(
        uiActions.showNotification({
          message: attemptResult.feedback,
          type: wasCorrect ? 'success' : 'warning',
        })
      );
    } catch (error) {
      console.error('Failed to submit exercise:', error);
      dispatch(
        uiActions.showNotification({
          message: 'Failed to submit exercise',
          type: 'error',
        })
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  if (isLoading || !exercise) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  if (showFeedback && result) {
    return (
      <ExerciseFeedback
        result={result}
        exercise={exercise}
        onContinue={() => navigate('/dashboard')}
      />
    );
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      {/* HEADER */}
      <div className="bg-white rounded-lg p-6 shadow">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold text-gray-900">
            {exercise.title}
          </h1>
          <div className="flex items-center gap-2 text-gray-600">
            <Clock size={20} />
            <span className="font-mono">{Math.floor(timeElapsed / 60)}:{String(timeElapsed % 60).padStart(2, '0')}</span>
          </div>
        </div>
        {exercise.description && (
          <p className="text-gray-700">{exercise.description}</p>
        )}
      </div>

      {/* EXERCISE CONTENT */}
      <div className="bg-white rounded-lg p-8 shadow">
        {/* PROMPT */}
        {exercise.prompt && (
          <div className="mb-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">
              {exercise.prompt}
            </h2>
          </div>
        )}

        {/* MULTIPLE CHOICE */}
        {exercise.type === 'multiple_choice' && exercise.options && (
          <div className="space-y-3">
            {exercise.options.map((option, index) => (
              <button
                key={index}
                onClick={() => setSelectedOption(index)}
                disabled={isSubmitting}
                className={`w-full text-left p-4 rounded-lg border-2 transition ${
                  selectedOption === index
                    ? 'border-indigo-600 bg-indigo-50'
                    : 'border-gray-300 bg-white hover:border-indigo-400'
                } disabled:opacity-50`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-6 h-6 rounded-full border-2 flex items-center justify-center ${
                      selectedOption === index
                        ? 'border-indigo-600 bg-indigo-600'
                        : 'border-gray-400'
                    }`}
                  >
                    {selectedOption === index && (
                      <div className="w-3 h-3 bg-white rounded-full"></div>
                    )}
                  </div>
                  <span className="font-medium text-gray-900">{option}</span>
                </div>
              </button>
            ))}
          </div>
        )}

        {/* TRANSLATION */}
        {exercise.type === 'translation' && (
          <div>
            {exercise.source_text && (
              <div className="mb-4 p-4 bg-gray-50 rounded-lg">
                <p className="text-sm text-gray-600 mb-1">Source text:</p>
                <p className="text-lg font-semibold text-gray-900">
                  {exercise.source_text}
                </p>
              </div>
            )}
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Your translation:
            </label>
            <input
              type="text"
              value={response}
              onChange={(e) => setResponse(e.target.value)}
              placeholder="Type your translation..."
              disabled={isSubmitting}
              className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:bg-gray-100"
              autoFocus
            />
          </div>
        )}

        {/* LISTENING */}
        {exercise.type === 'listening' && exercise.audio_url && (
          <div>
            <div className="mb-6 p-6 bg-gradient-to-r from-indigo-50 to-purple-50 rounded-lg">
              <div className="flex items-center justify-center mb-4">
                <Volume2 size={32} className="text-indigo-600" />
              </div>
              <audio
                src={exercise.audio_url}
                controls
                className="w-full"
                style={{ height: '50px' }}
              />
              {exercise.audio_duration_seconds && (
                <p className="text-sm text-gray-600 mt-2">
                  Duration: {exercise.audio_duration_seconds}s
                </p>
              )}
            </div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              What did you hear?
            </label>
            <input
              type="text"
              value={response}
              onChange={(e) => setResponse(e.target.value)}
              placeholder="Type what you heard..."
              disabled={isSubmitting}
              className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:bg-gray-100"
              autoFocus
            />
          </div>
        )}

        {/* READING COMPREHENSION */}
        {exercise.type === 'reading' && exercise.passage && (
          <div>
            <div className="mb-6 p-6 bg-gray-50 rounded-lg max-h-48 overflow-y-auto">
              <p className="text-gray-900 leading-relaxed">
                {exercise.passage}
              </p>
            </div>
            <div className="space-y-3">
              {exercise.options?.map((option, index) => (
                <button
                  key={index}
                  onClick={() => setSelectedOption(index)}
                  disabled={isSubmitting}
                  className={`w-full text-left p-4 rounded-lg border-2 transition ${
                    selectedOption === index
                      ? 'border-indigo-600 bg-indigo-50'
                      : 'border-gray-300 bg-white hover:border-indigo-400'
                  } disabled:opacity-50`}
                >
                  {option}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* SUBMIT BUTTON */}
      <div className="flex gap-3">
        <button
          onClick={() => navigate(-1)}
          disabled={isSubmitting}
          className="flex-1 px-6 py-3 border border-gray-300 rounded-lg font-semibold text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          Back
        </button>
        <button
          onClick={handleSubmit}
          disabled={isSubmitting}
          className="flex-1 px-6 py-3 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition flex items-center justify-center gap-2"
        >
          {isSubmitting ? (
            <>
              <LoadingSpinner size="small" />
              Submitting...
            </>
          ) : (
            <>
              <CheckCircle size={20} />
              Submit Answer
            </>
          )}
        </button>
      </div>

      {/* HINTS */}
      {exercise.explanation && (
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
          <p className="text-sm text-gray-600">
            <strong>Hint:</strong> {exercise.explanation}
          </p>
        </div>
      )}
    </div>
  );
};

export default ExercisePage;
