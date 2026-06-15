// PATHFINDER Frontend - SkillCard Component
// Skill card showing progress, mastery badge, and review status

import React from 'react';
import { ChevronRight, Flame, Target } from 'lucide-react';

interface SkillCardProps {
  skillId: string;
  skillCode: string;
  skillName: string;
  pKnow: number; // Probability of knowing (0-1)
  isMastered: boolean;
  reviewPriority: number;
  daysOverdue?: number;
  onStart: () => void;
}

const SkillCard: React.FC<SkillCardProps> = ({
  skillId,
  skillCode,
  skillName,
  pKnow,
  isMastered,
  reviewPriority,
  daysOverdue,
  onStart,
}) => {
  const progressPercent = Math.round(pKnow * 100);
  const isOverdue = daysOverdue && daysOverdue > 0;

  // Color based on progress
  let progressColor = 'bg-gray-400';
  if (progressPercent >= 85) progressColor = 'bg-green-500';
  else if (progressPercent >= 60) progressColor = 'bg-blue-500';
  else if (progressPercent >= 30) progressColor = 'bg-yellow-500';
  else progressColor = 'bg-red-500';

  return (
    <div
      className={`rounded-lg shadow hover:shadow-lg transition p-6 cursor-pointer ${
        isMastered ? 'bg-gradient-to-br from-green-50 to-emerald-50 border-2 border-green-200' : 'bg-white border-2 border-gray-200 hover:border-indigo-400'
      }`}
      onClick={onStart}
    >
      {/* HEADER */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <p className="text-xs font-semibold text-gray-500 uppercase tracking-wide">
            {skillCode}
          </p>
          <h3 className="text-lg font-bold text-gray-900 mt-1">{skillName}</h3>
        </div>

        {isMastered && (
          <div className="bg-green-500 text-white px-3 py-1 rounded-full text-xs font-bold flex-shrink-0 ml-2">
            ✓ Mastered
          </div>
        )}
      </div>

      {/* PROGRESS BAR */}
      <div className="mb-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-gray-600">P(Know)</span>
          <span className={`text-sm font-bold ${progressPercent >= 85 ? 'text-green-600' : 'text-gray-900'}`}>
            {progressPercent}%
          </span>
        </div>
        <div className="w-full bg-gray-200 rounded-full h-2">
          <div
            className={`${progressColor} h-2 rounded-full transition-all duration-300`}
            style={{ width: `${progressPercent}%` }}
          ></div>
        </div>
      </div>

      {/* REVIEW STATUS */}
      <div className="space-y-2 mb-4">
        {isOverdue && (
          <div className="flex items-center gap-2 bg-orange-50 border border-orange-200 rounded px-2 py-1">
            <Flame className="text-orange-500 flex-shrink-0" size={14} />
            <span className="text-xs text-orange-700 font-medium">
              {daysOverdue} days overdue
            </span>
          </div>
        )}

        {reviewPriority > 0 && !isMastered && (
          <div className="flex items-center gap-2 bg-indigo-50 border border-indigo-200 rounded px-2 py-1">
            <Target className="text-indigo-600 flex-shrink-0" size={14} />
            <span className="text-xs text-indigo-700 font-medium">
              Priority: {reviewPriority > 0.7 ? 'HIGH' : 'MEDIUM'}
            </span>
          </div>
        )}
      </div>

      {/* CTA BUTTON */}
      <button
        className={`w-full py-2 rounded-lg font-semibold flex items-center justify-center gap-2 transition ${
          isMastered
            ? 'bg-green-100 text-green-700 hover:bg-green-200'
            : 'bg-indigo-100 text-indigo-700 hover:bg-indigo-200'
        }`}
      >
        {isMastered ? 'Review' : 'Practice'}
        <ChevronRight size={16} />
      </button>

      {/* MASTERY THRESHOLD */}
      {!isMastered && (
        <p className="text-xs text-gray-500 text-center mt-3">
          {85 - progressPercent}% more to master
        </p>
      )}
    </div>
  );
};

export default SkillCard;
