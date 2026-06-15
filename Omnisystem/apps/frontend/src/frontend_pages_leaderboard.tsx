// PATHFINDER Frontend - LeaderboardPage
// Global leaderboard and ranking system

import React, { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { Trophy, Medal, Award } from 'lucide-react';

interface LeaderboardEntry {
  user_id: string;
  user_name: string;
  rank: number;
  points: number;
  achievements: number;
  mastery_percent: number;
  streak_days: number;
}

const LeaderboardPage: React.FC = () => {
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [userRank, setUserRank] = useState<LeaderboardEntry | null>(null);
  const [timeRange, setTimeRange] = useState<'week' | 'month' | 'all'>('all');
  const [sortBy, setSortBy] = useState<'points' | 'achievements' | 'mastery'>('points');

  // Load leaderboard
  useEffect(() => {
    const loadLeaderboard = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        const response = await apiClient.get(
          `/v1/leaderboard?limit=100`,
          { headers: { 'X-User-ID': user.id } }
        );

        setLeaderboard(response.data.leaderboard || []);

        // Find user's rank
        const userEntry = response.data.leaderboard?.find(
          (entry: LeaderboardEntry) => entry.user_id === user.id
        );
        setUserRank(userEntry || null);
      } catch (error) {
        console.error('Failed to load leaderboard:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadLeaderboard();
  }, [user, timeRange, sortBy]);

  const getMedalIcon = (rank: number) => {
    switch (rank) {
      case 1:
        return '🥇';
      case 2:
        return '🥈';
      case 3:
        return '🥉';
      default:
        return null;
    }
  };

  const sortedLeaderboard = [...leaderboard].sort((a, b) => {
    switch (sortBy) {
      case 'achievements':
        return b.achievements - a.achievements;
      case 'mastery':
        return b.mastery_percent - a.mastery_percent;
      default:
        return b.points - a.points;
    }
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-yellow-500 to-orange-600 rounded-lg p-8 text-white">
        <div className="flex items-center gap-3 mb-2">
          <Trophy size={32} />
          <h1 className="text-3xl font-bold">Global Leaderboard</h1>
        </div>
        <p className="text-yellow-100">
          See how you rank against other learners worldwide
        </p>
      </div>

      {/* YOUR RANK */}
      {userRank && (
        <div className="bg-gradient-to-r from-purple-50 to-pink-50 rounded-lg p-6 border-2 border-purple-300">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-600 mb-1">Your Rank</p>
              <h2 className="text-3xl font-bold text-gray-900 mb-2">
                #{userRank.rank}
              </h2>
              <p className="text-gray-700">
                You're in the top <strong>{Math.round((userRank.rank / leaderboard.length) * 100)}%</strong> of learners!
              </p>
            </div>

            <div className="grid grid-cols-3 gap-4 text-center">
              <div>
                <p className="text-xs text-gray-600 mb-1">Points</p>
                <p className="text-2xl font-bold text-gray-900">{userRank.points}</p>
              </div>
              <div>
                <p className="text-xs text-gray-600 mb-1">Achievements</p>
                <p className="text-2xl font-bold text-gray-900">{userRank.achievements}</p>
              </div>
              <div>
                <p className="text-xs text-gray-600 mb-1">Mastery</p>
                <p className="text-2xl font-bold text-gray-900">{Math.round(userRank.mastery_percent)}%</p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* FILTERS */}
      <div className="flex gap-4 items-center justify-between bg-white rounded-lg p-4 shadow">
        <div className="flex gap-2">
          {(['week', 'month', 'all'] as const).map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`px-4 py-2 rounded-lg font-semibold transition ${
                timeRange === range
                  ? 'bg-yellow-500 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              This {range === 'week' ? 'Week' : range === 'month' ? 'Month' : 'All Time'}
            </button>
          ))}
        </div>

        <div className="flex gap-2">
          {(['points', 'achievements', 'mastery'] as const).map((sort) => (
            <button
              key={sort}
              onClick={() => setSortBy(sort)}
              className={`px-4 py-2 rounded-lg font-semibold transition text-sm ${
                sortBy === sort
                  ? 'bg-purple-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {sort.charAt(0).toUpperCase() + sort.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* TOP 3 FEATURED */}
      {sortedLeaderboard.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {sortedLeaderboard.slice(0, 3).map((entry) => (
            <div
              key={entry.user_id}
              className={`rounded-lg p-6 text-white text-center shadow-lg ${
                entry.rank === 1
                  ? 'bg-gradient-to-br from-yellow-400 to-yellow-600'
                  : entry.rank === 2
                  ? 'bg-gradient-to-br from-gray-300 to-gray-500'
                  : 'bg-gradient-to-br from-orange-400 to-orange-600'
              }`}
            >
              <div className="text-5xl mb-3">{getMedalIcon(entry.rank)}</div>
              <h3 className="text-xl font-bold mb-2">{entry.user_name}</h3>
              <p className="text-sm opacity-90 mb-4">Rank #{entry.rank}</p>

              <div className="space-y-2 text-sm">
                <div className="bg-white bg-opacity-20 rounded-lg p-2">
                  <p className="opacity-75">Points</p>
                  <p className="text-2xl font-bold">{entry.points}</p>
                </div>
                <div className="bg-white bg-opacity-20 rounded-lg p-2">
                  <p className="opacity-75">Achievements</p>
                  <p className="text-xl font-bold">{entry.achievements}</p>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* FULL LEADERBOARD */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-50 border-b border-gray-200">
              <tr>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Rank</th>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Learner</th>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Points</th>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Achievements</th>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Mastery</th>
                <th className="px-6 py-4 text-left text-sm font-bold text-gray-700">Streak</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200">
              {sortedLeaderboard.map((entry, index) => (
                <tr
                  key={entry.user_id}
                  className={`hover:bg-gray-50 transition ${
                    entry.user_id === user?.id ? 'bg-purple-50' : ''
                  }`}
                >
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-3">
                      {getMedalIcon(entry.rank) && (
                        <span className="text-2xl">{getMedalIcon(entry.rank)}</span>
                      )}
                      <span className="font-bold text-gray-900">#{entry.rank}</span>
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2">
                      <div className="w-10 h-10 bg-gradient-to-br from-purple-400 to-pink-400 rounded-full flex items-center justify-center text-white font-bold">
                        {entry.user_name.charAt(0).toUpperCase()}
                      </div>
                      <div>
                        <p className="font-semibold text-gray-900">{entry.user_name}</p>
                        {entry.user_id === user?.id && (
                          <p className="text-xs text-purple-600">You</p>
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <span className="font-bold text-gray-900">{entry.points}</span>
                  </td>
                  <td className="px-6 py-4">
                    <span className="font-bold text-gray-900">{entry.achievements}</span>
                  </td>
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2">
                      <div className="w-20 bg-gray-200 rounded-full h-2">
                        <div
                          className="h-2 rounded-full bg-gradient-to-r from-green-400 to-green-600"
                          style={{ width: `${entry.mastery_percent}%` }}
                        ></div>
                      </div>
                      <span className="font-bold text-gray-900 text-sm w-10">
                        {Math.round(entry.mastery_percent)}%
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <span className="font-bold text-gray-900">🔥 {entry.streak_days}d</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* TIPS */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-6">
        <h3 className="text-lg font-bold text-gray-900 mb-4">🎯 Climb the Leaderboard</h3>
        <ul className="space-y-2 text-gray-700">
          <li>
            <strong>✨ Earn Points:</strong> Complete exercises, master skills, and unlock achievements to gain XP.
          </li>
          <li>
            <strong>🔥 Build Streaks:</strong> Consistent daily learning increases your streak and boost your rank.
          </li>
          <li>
            <strong>🎖️ Unlock Achievements:</strong> Special achievements and rare badges are worth more points.
          </li>
          <li>
            <strong>📚 Increase Mastery:</strong> Aim for 85%+ mastery on skills to maximize your score.
          </li>
        </ul>
      </div>
    </div>
  );
};

export default LeaderboardPage;
