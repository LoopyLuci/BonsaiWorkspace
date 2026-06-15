// PATHFINDER Frontend - useNotifications Hook
// Notification management for parent/student interface

import { useState, useCallback, useEffect } from 'react';
import apiClient from '../api-client';

interface Notification {
  id: string;
  user_id: string;
  type: string;
  channel: string;
  subject: string;
  message: string;
  status: string;
  sent_at?: string;
  opened_at?: string;
  created_at: string;
}

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

interface UseNotificationsReturn {
  // Notifications
  notifications: Notification[];
  unreadCount: number;
  isLoading: boolean;
  error: string | null;

  // Methods
  fetchNotifications: (limit?: number, offset?: number) => Promise<void>;
  sendNotification: (
    userID: string,
    type: string,
    channel: string,
    subject: string,
    message: string,
    data?: Record<string, any>
  ) => Promise<string>;
  sendBatchNotifications: (notifications: any[]) => Promise<void>;
  markAsOpened: (notificationId: string) => Promise<void>;
  deleteNotification: (notificationId: string) => Promise<void>;
  clearAllNotifications: () => Promise<void>;

  // Preferences
  preferences: NotificationPreferences | null;
  preferencesLoading: boolean;
  fetchPreferences: () => Promise<void>;
  updatePreferences: (prefs: Partial<NotificationPreferences>) => Promise<void>;
}

export const useNotifications = (userID: string): UseNotificationsReturn => {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [preferences, setPreferences] = useState<NotificationPreferences | null>(null);
  const [preferencesLoading, setPreferencesLoading] = useState(false);

  // Fetch notifications
  const fetchNotifications = useCallback(
    async (limit = 50, offset = 0) => {
      if (!userID) return;

      try {
        setIsLoading(true);
        setError(null);

        const response = await apiClient.get(
          `/v1/notifications?limit=${limit}&offset=${offset}`,
          {
            headers: { 'X-User-ID': userID },
          }
        );

        setNotifications(response.data.notifications || []);
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Failed to load notifications';
        setError(message);
        console.error('Error fetching notifications:', err);
      } finally {
        setIsLoading(false);
      }
    },
    [userID]
  );

  // Send notification
  const sendNotification = useCallback(
    async (
      recipientUserID: string,
      type: string,
      channel: string,
      subject: string,
      message: string,
      data?: Record<string, any>
    ): Promise<string> => {
      try {
        const response = await apiClient.post(
          '/v1/notifications/send',
          {
            user_id: recipientUserID,
            type,
            channel,
            subject,
            message,
            data: data || {},
          },
          {
            headers: { 'X-User-ID': userID },
          }
        );

        return response.data.id;
      } catch (err) {
        console.error('Error sending notification:', err);
        throw err;
      }
    },
    [userID]
  );

  // Send batch notifications
  const sendBatchNotifications = useCallback(
    async (notificationsToSend: any[]) => {
      try {
        await apiClient.post('/v1/notifications/batch', notificationsToSend, {
          headers: { 'X-User-ID': userID },
        });
      } catch (err) {
        console.error('Error sending batch notifications:', err);
        throw err;
      }
    },
    [userID]
  );

  // Mark as opened
  const markAsOpened = useCallback(
    async (notificationId: string) => {
      try {
        await apiClient.post(
          '/v1/notifications/mark-opened',
          {},
          {
            headers: { 'X-User-ID': userID },
            params: { id: notificationId },
          }
        );

        setNotifications((prev) =>
          prev.map((n) =>
            n.id === notificationId ? { ...n, opened_at: new Date().toISOString() } : n
          )
        );
      } catch (err) {
        console.error('Error marking notification as opened:', err);
        throw err;
      }
    },
    [userID]
  );

  // Delete notification
  const deleteNotification = useCallback(
    async (notificationId: string) => {
      try {
        await apiClient.delete('/v1/notifications/delete', {
          headers: { 'X-User-ID': userID },
          params: { id: notificationId },
        });

        setNotifications((prev) => prev.filter((n) => n.id !== notificationId));
      } catch (err) {
        console.error('Error deleting notification:', err);
        throw err;
      }
    },
    [userID]
  );

  // Clear all notifications
  const clearAllNotifications = useCallback(async () => {
    try {
      await Promise.all(notifications.map((n) => deleteNotification(n.id)));
    } catch (err) {
      console.error('Error clearing notifications:', err);
      throw err;
    }
  }, [notifications, deleteNotification]);

  // Fetch preferences
  const fetchPreferences = useCallback(async () => {
    if (!userID) return;

    try {
      setPreferencesLoading(true);

      const response = await apiClient.get('/v1/notifications/preferences', {
        headers: { 'X-User-ID': userID },
      });

      setPreferences(response.data);
    } catch (err) {
      // Use defaults if not found
      setPreferences({
        id: '',
        user_id: userID,
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
        updated_at: new Date().toISOString(),
      });
      console.error('Error fetching preferences:', err);
    } finally {
      setPreferencesLoading(false);
    }
  }, [userID]);

  // Update preferences
  const updatePreferences = useCallback(
    async (updates: Partial<NotificationPreferences>) => {
      if (!preferences) return;

      try {
        const updated = { ...preferences, ...updates };

        await apiClient.post('/v1/notifications/preferences', updated, {
          headers: { 'X-User-ID': userID },
        });

        setPreferences(updated);
      } catch (err) {
        console.error('Error updating preferences:', err);
        throw err;
      }
    },
    [preferences, userID]
  );

  // Initial load
  useEffect(() => {
    if (userID) {
      fetchNotifications();
      fetchPreferences();
    }
  }, [userID, fetchNotifications, fetchPreferences]);

  const unreadCount = notifications.filter((n) => !n.opened_at).length;

  return {
    notifications,
    unreadCount,
    isLoading,
    error,
    fetchNotifications,
    sendNotification,
    sendBatchNotifications,
    markAsOpened,
    deleteNotification,
    clearAllNotifications,
    preferences,
    preferencesLoading,
    fetchPreferences,
    updatePreferences,
  };
};

export default useNotifications;
