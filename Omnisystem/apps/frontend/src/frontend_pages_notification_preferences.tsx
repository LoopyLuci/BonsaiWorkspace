// PATHFINDER Frontend - NotificationPreferencesPage
// Parent notification preferences and settings

import React, { useEffect, useState } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { uiActions } from '../store';
import apiClient from '../api-client';
import LoadingSpinner from '../components/LoadingSpinner';
import { Bell, Clock, Mail, Smartphone, Volume2 } from 'lucide-react';

interface NotificationPreferences {
  id: string;
  user_id: string;
  notify_mastery: boolean;
  notify_alerts: boolean;
  notify_daily_summary: boolean;
  notify_weekly_report: boolean;
  notify_achievements: boolean;
  email_frequency: string;
  quiet_hours_enabled: boolean;
  quiet_hours_start: string;
  quiet_hours_end: string;
  timezone: string;
  updated_at: string;
}

const NotificationPreferencesPage: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);

  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [preferences, setPreferences] = useState<NotificationPreferences>({
    id: '',
    user_id: user?.id || '',
    notify_mastery: true,
    notify_alerts: true,
    notify_daily_summary: true,
    notify_weekly_report: false,
    notify_achievements: true,
    email_frequency: 'daily',
    quiet_hours_enabled: false,
    quiet_hours_start: '22:00',
    quiet_hours_end: '08:00',
    timezone: 'UTC',
    updated_at: '',
  });

  // Load preferences
  useEffect(() => {
    const loadPreferences = async () => {
      if (!user) return;

      try {
        setIsLoading(true);

        const response = await apiClient.get('/v1/notifications/preferences', {
          headers: { 'X-User-ID': user.id },
        });

        setPreferences(response.data);
      } catch (error) {
        console.error('Failed to load preferences:', error);
        // Use defaults if not found
      } finally {
        setIsLoading(false);
      }
    };

    loadPreferences();
  }, [user]);

  // Save preferences
  const handleSave = async () => {
    if (!user) return;

    try {
      setIsSaving(true);

      await apiClient.post('/v1/notifications/preferences', preferences, {
        headers: { 'X-User-ID': user.id },
      });

      dispatch(
        uiActions.showNotification({
          message: 'Notification preferences saved',
          type: 'success',
        })
      );
    } catch (error) {
      console.error('Failed to save preferences:', error);
      dispatch(
        uiActions.showNotification({
          message: 'Failed to save preferences',
          type: 'error',
        })
      );
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto space-y-8">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <div className="flex items-center gap-3 mb-2">
          <Bell size={32} />
          <h1 className="text-3xl font-bold">Notification Preferences</h1>
        </div>
        <p className="text-indigo-100">
          Customize how and when you receive updates about your children's learning
        </p>
      </div>

      {/* NOTIFICATION TYPES */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">What to Notify About</h2>

        <div className="space-y-4">
          {/* Mastery */}
          <div className="flex items-start gap-4 p-4 border border-gray-200 rounded-lg">
            <div className="mt-1">
              <input
                type="checkbox"
                id="notify_mastery"
                checked={preferences.notify_mastery}
                onChange={(e) =>
                  setPreferences({ ...preferences, notify_mastery: e.target.checked })
                }
                className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
              />
            </div>
            <div className="flex-1">
              <label htmlFor="notify_mastery" className="block text-lg font-semibold text-gray-900 cursor-pointer">
                Skill Mastery
              </label>
              <p className="text-sm text-gray-600 mt-1">
                Get notified when your child masters a new skill (reaches 85% mastery)
              </p>
            </div>
          </div>

          {/* Alerts */}
          <div className="flex items-start gap-4 p-4 border border-gray-200 rounded-lg">
            <div className="mt-1">
              <input
                type="checkbox"
                id="notify_alerts"
                checked={preferences.notify_alerts}
                onChange={(e) =>
                  setPreferences({ ...preferences, notify_alerts: e.target.checked })
                }
                className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
              />
            </div>
            <div className="flex-1">
              <label htmlFor="notify_alerts" className="block text-lg font-semibold text-gray-900 cursor-pointer">
                Struggling Alerts
              </label>
              <p className="text-sm text-gray-600 mt-1">
                Get notified when your child needs help or has been stuck on a skill
              </p>
            </div>
          </div>

          {/* Daily Summary */}
          <div className="flex items-start gap-4 p-4 border border-gray-200 rounded-lg">
            <div className="mt-1">
              <input
                type="checkbox"
                id="notify_daily_summary"
                checked={preferences.notify_daily_summary}
                onChange={(e) =>
                  setPreferences({ ...preferences, notify_daily_summary: e.target.checked })
                }
                className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
              />
            </div>
            <div className="flex-1">
              <label htmlFor="notify_daily_summary" className="block text-lg font-semibold text-gray-900 cursor-pointer">
                Daily Summary
              </label>
              <p className="text-sm text-gray-600 mt-1">
                Receive a daily recap of what your child learned and accomplished
              </p>
            </div>
          </div>

          {/* Weekly Report */}
          <div className="flex items-start gap-4 p-4 border border-gray-200 rounded-lg">
            <div className="mt-1">
              <input
                type="checkbox"
                id="notify_weekly_report"
                checked={preferences.notify_weekly_report}
                onChange={(e) =>
                  setPreferences({ ...preferences, notify_weekly_report: e.target.checked })
                }
                className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
              />
            </div>
            <div className="flex-1">
              <label htmlFor="notify_weekly_report" className="block text-lg font-semibold text-gray-900 cursor-pointer">
                Weekly Report
              </label>
              <p className="text-sm text-gray-600 mt-1">
                Get a comprehensive weekly report with trends and insights
              </p>
            </div>
          </div>

          {/* Achievements */}
          <div className="flex items-start gap-4 p-4 border border-gray-200 rounded-lg">
            <div className="mt-1">
              <input
                type="checkbox"
                id="notify_achievements"
                checked={preferences.notify_achievements}
                onChange={(e) =>
                  setPreferences({ ...preferences, notify_achievements: e.target.checked })
                }
                className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
              />
            </div>
            <div className="flex-1">
              <label htmlFor="notify_achievements" className="block text-lg font-semibold text-gray-900 cursor-pointer">
                Achievements
              </label>
              <p className="text-sm text-gray-600 mt-1">
                Celebrate special milestones like streaks and badges with your child
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* EMAIL FREQUENCY */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
          <Mail size={24} className="text-indigo-600" />
          Email Frequency
        </h2>

        <div className="space-y-3">
          {['immediate', 'daily', 'weekly', 'never'].map((freq) => (
            <label key={freq} className="flex items-center gap-3 p-4 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50">
              <input
                type="radio"
                name="email_frequency"
                value={freq}
                checked={preferences.email_frequency === freq}
                onChange={(e) =>
                  setPreferences({ ...preferences, email_frequency: e.target.value })
                }
                className="w-4 h-4 text-indigo-600"
              />
              <div>
                <p className="font-semibold text-gray-900 capitalize">{freq}</p>
                <p className="text-sm text-gray-600">
                  {freq === 'immediate'
                    ? 'Notify me right away when something important happens'
                    : freq === 'daily'
                    ? 'Receive one email each day with all updates'
                    : freq === 'weekly'
                    ? 'Receive one email per week with all updates'
                    : 'Never send me emails'}
                </p>
              </div>
            </label>
          ))}
        </div>
      </div>

      {/* QUIET HOURS */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
          <Clock size={24} className="text-indigo-600" />
          Quiet Hours
        </h2>

        <div className="mb-6">
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={preferences.quiet_hours_enabled}
              onChange={(e) =>
                setPreferences({ ...preferences, quiet_hours_enabled: e.target.checked })
              }
              className="w-5 h-5 text-indigo-600 rounded focus:ring-2 focus:ring-indigo-500"
            />
            <span className="font-semibold text-gray-900">Enable quiet hours</span>
          </label>
          <p className="text-sm text-gray-600 mt-2 ml-8">
            Pause notifications during specific times (e.g., nights, family time)
          </p>
        </div>

        {preferences.quiet_hours_enabled && (
          <div className="grid grid-cols-2 gap-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Quiet Hours Start
              </label>
              <input
                type="time"
                value={preferences.quiet_hours_start}
                onChange={(e) =>
                  setPreferences({ ...preferences, quiet_hours_start: e.target.value })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
              <p className="text-xs text-gray-500 mt-1">e.g., 22:00 (10 PM)</p>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Quiet Hours End
              </label>
              <input
                type="time"
                value={preferences.quiet_hours_end}
                onChange={(e) =>
                  setPreferences({ ...preferences, quiet_hours_end: e.target.value })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
              <p className="text-xs text-gray-500 mt-1">e.g., 08:00 (8 AM)</p>
            </div>
          </div>
        )}
      </div>

      {/* TIMEZONE */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-2xl font-bold text-gray-900 mb-6 flex items-center gap-2">
          <Volume2 size={24} className="text-indigo-600" />
          Timezone
        </h2>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Your Timezone
          </label>
          <select
            value={preferences.timezone}
            onChange={(e) =>
              setPreferences({ ...preferences, timezone: e.target.value })
            }
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
          >
            <option value="UTC">UTC</option>
            <option value="EST">Eastern (EST)</option>
            <option value="CST">Central (CST)</option>
            <option value="MST">Mountain (MST)</option>
            <option value="PST">Pacific (PST)</option>
            <option value="GMT">GMT</option>
            <option value="CET">Central European (CET)</option>
            <option value="IST">Indian Standard (IST)</option>
            <option value="JST">Japan Standard (JST)</option>
            <option value="AEST">Australian Eastern (AEST)</option>
          </select>
          <p className="text-sm text-gray-600 mt-2">
            Used to calculate quiet hours and daily summary times
          </p>
        </div>
      </div>

      {/* SAVE BUTTON */}
      <div className="flex gap-3">
        <button
          onClick={handleSave}
          disabled={isSaving}
          className="flex-1 px-6 py-3 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition"
        >
          {isSaving ? 'Saving...' : 'Save Preferences'}
        </button>
        <button
          onClick={() => window.history.back()}
          className="flex-1 px-6 py-3 bg-white border border-gray-300 hover:bg-gray-50 text-gray-700 font-semibold rounded-lg transition"
        >
          Cancel
        </button>
      </div>

      {/* INFO BOX */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <p className="text-sm text-blue-800">
          💡 <strong>Tip:</strong> You can manage these settings at any time. Quiet hours respect your timezone setting, so notifications will pause during your specified times regardless of when you access the platform.
        </p>
      </div>
    </div>
  );
};

export default NotificationPreferencesPage;
