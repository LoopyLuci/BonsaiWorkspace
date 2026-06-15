// PATHFINDER Frontend - ExerciseFeedback Component
// Shows results after exercise submission (BKT/HLR impact)

import React from 'react';
import { CheckCircle2, XCircle, TrendingUp, Calendar, Zap } from 'lucide-react';

interface ExerciseAttemptResponse {
  was_correct: boolean;
  feedback: string;
  skill_state: {
    p_know: number;
    previous_p_know: number;
    is_mastered: boolean;
    strength: number;
    halflife_days: number;
  };
  next_review_at: string;
  xp_earned: number;
}

interface Exercise {
  id: string;
  title: string;
}

interface ExerciseFeedbackProps {
  result: ExerciseAttemptResponse;
  exercise: Exercise;
  onContinue: () => void;
}

const ExerciseFeedback: React.FC<ExerciseFeedbackProps> = ({
  result,
  exercise,
  onContinue,
}) => {
  const {
    was_correct,
    feedback,
    skill_state,
    next_review_at,
    xp_earned,
  } = result;

  const pKnowPercent = Math.round(skill_state.p_know * 100);
  const previousPercent = Math.round(skill_state.previous_p_know * 100);
  const improvement = pKnowPercent - previousPercent;

  // Format next review date
  const nextReviewDate = new Date(next_review_at);
  const today = new Date();
  const daysUntilReview = Math.ceil(
    (nextReviewDate.getTime() - today.getTime()) / (1000 * 60 * 60 * 24)
  );

  return (
    <div className="min-h-screen bg-gradient-to-br from-indigo-50 to-purple-50 p-6 flex items-center justify-center">
      <div className="max-w-lg w-full">
        {/* RESULT HEADER */}
        <div className={`rounded-t-2xl p-8 text-white text-center ${
          was_correct
            ? 'bg-gradient-to-r from-green-500 to-emerald-500'
            : 'bg-gradient-to-r from-orange-500 to-amber-500'
        }`}>
          {was_correct ? (
            <>
              <CheckCircle2 className="mx-auto mb-4" size={64} />
              <h1 className="text-4xl font-bold mb-2">Correct! 🎉</h1>
            </>
          ) : (
            <>
              <XCircle className="mx-auto mb-4" size={64} />
              <h1 className="text-4xl font-bold mb-2">Not quite</h1>
            </>
          )}
          <p className="text-lg opacity-90">{feedback}</p>
        </div>

        {/* BODY */}
        <div className="bg-white rounded-b-2xl p-8 shadow-lg space-y-6">
          {/* EXERCISE NAME */}
          <div className="text-center border-b pb-6">
            <p className="text-sm text-gray-600 mb-1">Exercise</p>
            <h2 className="text-2xl font-bold text-gray-900">{exercise.title}</h2>
          </div>

          {/* XP EARNED */}
          <div className="bg-gradient-to-r from-yellow-50 to-amber-50 rounded-lg p-4 flex items-center gap-3">
            <Zap className="text-yellow-600 flex-shrink-0" size={24} />
            <div>
              <p className="text-sm text-gray-600">Experience Points</p>
              <p className="text-3xl font-bold text-gray-900">+{xp_earned} XP</p>
            </div>
          </div>

          {/* BKT IMPACT */}
          <div className="bg-indigo-50 border border-indigo-200 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-4">
              <TrendingUp className="text-indigo-600" size={20} />
              <h3 className="font-semibold text-gray-900">Learning Progress</h3>
            </div>

            <div className="space-y-3">
              {/* Previous P(Know) */}
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm text-gray-600">Before</span>
                  <span className="font-semibold text-gray-900">
                    {previousPercent}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-gray-400 h-2 rounded-full"
                    style={{ width: `${previousPercent}%` }}
                  ></div>
                </div>
              </div>

              {/* Arrow */}
              <div className="flex justify-center">
                <div className="text-2xl text-indigo-600">↓</div>
              </div>

              {/* New P(Know) */}
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm text-gray-600">After</span>
                  <span className="font-bold text-indigo-600">
                    {pKnowPercent}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className={`h-2 rounded-full ${
                      pKnowPercent >= 85 ? 'bg-green-500' : 'bg-indigo-600'
                    }`}
                    style={{ width: `${pKnowPercent}%` }}
                  ></div>
                </div>
              </div>

              {/* Change */}
              {improvement !== 0 && (
                <div className={`text-center pt-2 ${
                  improvement > 0 ? 'text-green-600' : 'text-red-600'
                }`}>
                  <p className="font-semibold">
                    {improvement > 0 ? '+' : ''}{improvement}% improvement
                  </p>
                </div>
              )}
            </div>

            {/* Explanation */}
            <p className="text-xs text-gray-600 mt-4">
              <strong>How it works:</strong> Your probability of knowing this skill
              was updated using Bayesian Knowledge Tracing, a proven learning
              science algorithm.
            </p>
          </div>

          {/* MASTERY STATUS */}
          {skill_state.is_mastered && (
            <div className="bg-green-50 border border-green-200 rounded-lg p-4">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="text-green-600" size={20} />
                <div>
                  <p className="font-semibold text-green-900">Skill Mastered! 🏆</p>
                  <p className="text-sm text-green-700">
                    You've reached 85% proficiency. Great work!
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* NEXT REVIEW */}
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 flex items-center gap-3">
            <Calendar className="text-blue-600 flex-shrink-0" size={20} />
            <div>
              <p className="text-sm text-gray-600">Next Review</p>
              <p className="font-semibold text-gray-900">
                {daysUntilReview} {daysUntilReview === 1 ? 'day' : 'days'} from now
              </p>
              <p className="text-xs text-gray-500">
                {nextReviewDate.toLocaleDateString(undefined, {
                  weekday: 'long',
                  month: 'short',
                  day: 'numeric',
                })}
              </p>
            </div>
          </div>

          {/* STRENGTH (HLR) */}
          <div className="border-t pt-4">
            <p className="text-sm text-gray-600 mb-2">Memory Strength</p>
            <div className="flex items-center gap-2">
              <div className="flex-1 bg-gray-200 rounded-full h-3">
                <div
                  className="bg-purple-600 h-3 rounded-full"
                  style={{
                    width: `${Math.min(100, (skill_state.strength / 10) * 100)}%`,
                  }}
                ></div>
              </div>
              <span className="text-sm font-semibold text-gray-900">
                {skill_state.strength.toFixed(1)} days
              </span>
            </div>
            <p className="text-xs text-gray-500 mt-2">
              Half-life: How long your memory of this skill lasts before fading
            </p>
          </div>

          {/* CONTINUE BUTTON */}
          <button
            onClick={onContinue}
            className="w-full py-4 bg-indigo-600 hover:bg-indigo-700 text-white font-bold rounded-lg transition mt-6"
          >
            {skill_state.is_mastered ? 'Continue to Dashboard' : 'Next Exercise'}
          </button>

          {/* LEARNING INSIGHTS */}
          <div className="bg-gray-50 rounded-lg p-4 text-center text-sm text-gray-600">
            <p>
              💡 <strong>Pro tip:</strong> Review this skill in {daysUntilReview}{' '}
              days to maximize your retention using spaced repetition science.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ExerciseFeedback;
