// PATHFINDER Frontend - BadgeCard Component
// Display individual achievement badges

import React from 'react';
import { Lock, Star } from 'lucide-react';

interface Badge {
  id: string;
  name: string;
  category: string;
  description: string;
  icon_url: string;
  requirement: string;
  rarity: 'common' | 'uncommon' | 'rare' | 'epic' | 'legendary';
  points: number;
  unlocked?: boolean;
  progress?: number; // 0-100
}

interface BadgeCardProps {
  badge: Badge;
  unlocked?: boolean;
  progress?: number;
}

const BadgeCard: React.FC<BadgeCardProps> = ({ badge, unlocked = false, progress = 0 }) => {
  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case 'common':
        return { bg: 'bg-gray-100', border: 'border-gray-300', text: 'text-gray-700', icon: 'text-gray-400' };
      case 'uncommon':
        return { bg: 'bg-green-100', border: 'border-green-300', text: 'text-green-700', icon: 'text-green-500' };
      case 'rare':
        return { bg: 'bg-blue-100', border: 'border-blue-300', text: 'text-blue-700', icon: 'text-blue-500' };
      case 'epic':
        return { bg: 'bg-purple-100', border: 'border-purple-300', text: 'text-purple-700', icon: 'text-purple-500' };
      case 'legendary':
        return { bg: 'bg-yellow-100', border: 'border-yellow-300', text: 'text-yellow-700', icon: 'text-yellow-500' };
      default:
        return { bg: 'bg-gray-100', border: 'border-gray-300', text: 'text-gray-700', icon: 'text-gray-400' };
    }
  };

  const colors = getRarityColor(badge.rarity);

  return (
    <div
      className={`rounded-lg p-6 border-2 transition ${colors.border} ${
        unlocked ? colors.bg : 'bg-gray-50 opacity-60'
      }`}
    >
      {/* BADGE ICON */}
      <div className="relative mb-4">
        <div className={`text-5xl text-center mb-2 ${unlocked ? '' : 'grayscale'}`}>
          {badge.icon_url || '🏆'}
        </div>
        {!unlocked && (
          <div className={`absolute top-0 right-0 ${colors.icon}`}>
            <Lock size={20} />
          </div>
        )}
      </div>

      {/* BADGE INFO */}
      <h3 className={`font-bold text-center mb-1 ${colors.text}`}>{badge.name}</h3>
      <p className="text-xs text-gray-600 text-center mb-3">{badge.description}</p>

      {/* RARITY & CATEGORY */}
      <div className="flex gap-2 mb-3 justify-center">
        <span className={`text-xs px-2 py-1 rounded-full font-bold capitalize ${colors.bg} ${colors.text}`}>
          {badge.rarity}
        </span>
        <span className="text-xs px-2 py-1 rounded-full bg-gray-200 text-gray-700 font-medium">
          {badge.category.replace(/_/g, ' ')}
        </span>
      </div>

      {/* POINTS */}
      <div className="text-center mb-3">
        <p className="text-xs text-gray-600">Reward</p>
        <p className="text-lg font-bold text-gray-900">⭐ {badge.points} XP</p>
      </div>

      {/* REQUIREMENT */}
      <div className="bg-white rounded-lg p-3 mb-3 border border-gray-200">
        <p className="text-xs text-gray-600 mb-1">Requirement</p>
        <p className="text-sm font-medium text-gray-900">{badge.requirement}</p>
      </div>

      {/* PROGRESS BAR (if locked and progress > 0) */}
      {!unlocked && progress > 0 && (
        <div className="mb-3">
          <div className="flex justify-between mb-1">
            <span className="text-xs font-medium text-gray-600">Progress</span>
            <span className="text-xs font-bold text-gray-900">{Math.round(progress)}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className={`h-2 rounded-full transition-all duration-300 ${
                progress > 66
                  ? 'bg-green-500'
                  : progress > 33
                  ? 'bg-yellow-500'
                  : 'bg-orange-500'
              }`}
              style={{ width: `${progress}%` }}
            ></div>
          </div>
        </div>
      )}

      {/* STATUS */}
      {unlocked ? (
        <div className="bg-green-50 border border-green-200 rounded-lg p-2 text-center">
          <p className="text-xs font-bold text-green-700">✓ Unlocked</p>
        </div>
      ) : (
        <div className="bg-gray-100 border border-gray-200 rounded-lg p-2 text-center">
          <p className="text-xs font-bold text-gray-600">🔒 Locked</p>
        </div>
      )}
    </div>
  );
};

export default BadgeCard;
