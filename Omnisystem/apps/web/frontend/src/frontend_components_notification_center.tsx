// PATHFINDER Frontend - NotificationCenter Component
// Display and manage recent notifications

import React, { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';
import type { RootState } from '../store';
import apiClient from '../api-client';
import { Bell, X, Trash2, Archive } from 'lucide-react';

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

interface NotificationCenterProps {
  isOpen: boolean;
  onClose: () => void;
}

const NotificationCenter: React.FC<NotificationCenterProps> = ({ isOpen, onClose }) => {
  const { user } = useSelector((state: RootState) => state.auth);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [filter, setFilter] = useState<'all' | 'unread' | 'archived'>('all');

  // Load notifications
  useEffect(() => {
    if (!isOpen || !user) return;

    const loadNotifications = async () => {
      try {
        setIsLoading(true);

        const response = await apiClient.get('/v1/notifications?limit=20', {
          headers: { 'X-User-ID': user.id },
        });

        setNotifications(response.data.notifications || []);
      } catch (error) {
        console.error('Failed to load notifications:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadNotifications();
  }, [isOpen, user]);

  // Mark as opened
  const handleMarkOpened = async (notificationId: string) => {
    if (!user) return;

    try {
      await apiClient.post(
        '/v1/notifications/mark-opened',
        {},
        {
          headers: { 'X-User-ID': user.id },
          params: { id: notificationId },
        }
      );

      setNotifications((prev) =>
        prev.map((n) =>
          n.id === notificationId ? { ...n, opened_at: new Date().toISOString() } : n
        )
      );
    } catch (error) {
      console.error('Failed to mark opened:', error);
    }
  };

  // Delete notification
  const handleDelete = async (notificationId: string) => {
    if (!user) return;

    try {
      await apiClient.delete('/v1/notifications/delete', {
        headers: { 'X-User-ID': user.id },
        params: { id: notificationId },
      });

      setNotifications((prev) => prev.filter((n) => n.id !== notificationId));
    } catch (error) {
      console.error('Failed to delete notification:', error);
    }
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'mastery':
        return '🏆';
      case 'alert':
        return '⚠️';
      case 'summary':
        return '📊';
      case 'achievement':
        return '🎉';
      default:
        return '📧';
    }
  };

  const getChannelColor = (channel: string) => {
    switch (channel) {
      case 'email':
        return 'bg-blue-100 text-blue-800';
      case 'push':
        return 'bg-purple-100 text-purple-800';
      case 'sms':
        return 'bg-green-100 text-green-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const filteredNotifications = notifications.filter((n) => {
    if (filter === 'unread') return !n.opened_at;
    return true;
  });

  const unreadCount = notifications.filter((n) => !n.opened_at).length;

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-end sm:items-center justify-center">
      <div className="bg-white rounded-t-lg sm:rounded-lg w-full sm:max-w-md sm:max-h-96 flex flex-col shadow-lg">
        {/* HEADER */}
        <div className="flex items-center justify-between p-4 border-b border-gray-200">
          <div className="flex items-center gap-2">
            <Bell size={20} className="text-indigo-600" />
            <h2 className="text-lg font-bold text-gray-900">Notifications</h2>
            {unreadCount > 0 && (
              <span className="ml-2 px-2 py-1 bg-red-100 text-red-800 text-xs font-bold rounded-full">
                {unreadCount}
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-lg transition"
          >
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* FILTERS */}
        <div className="flex gap-2 px-4 py-3 border-b border-gray-200">
          {(['all', 'unread'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 rounded-full text-sm font-medium transition ${
                filter === f
                  ? 'bg-indigo-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>

        {/* NOTIFICATIONS LIST */}
        <div className="flex-1 overflow-y-auto">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="text-center">
                <div className="w-8 h-8 border-4 border-gray-300 border-t-indigo-600 rounded-full animate-spin mx-auto mb-2"></div>
                <p className="text-sm text-gray-600">Loading...</p>
              </div>
            </div>
          ) : filteredNotifications.length === 0 ? (
            <div className="flex items-center justify-center py-8 text-center">
              <div>
                <Bell size={32} className="text-gray-300 mx-auto mb-2" />
                <p className="text-gray-600">No notifications</p>
              </div>
            </div>
          ) : (
            <div className="divide-y divide-gray-200">
              {filteredNotifications.map((notification) => (
                <div
                  key={notification.id}
                  className={`p-4 hover:bg-gray-50 transition cursor-pointer ${
                    !notification.opened_at ? 'bg-indigo-50' : ''
                  }`}
                  onClick={() => !notification.opened_at && handleMarkOpened(notification.id)}
                >
                  <div className="flex gap-3">
                    {/* ICON */}
                    <div className="text-2xl flex-shrink-0">
                      {getTypeIcon(notification.type)}
                    </div>

                    {/* CONTENT */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-2">
                        <h3 className="font-semibold text-gray-900 line-clamp-2">
                          {notification.subject}
                        </h3>
                        {!notification.opened_at && (
                          <div className="w-2 h-2 rounded-full bg-indigo-600 flex-shrink-0 mt-2"></div>
                        )}
                      </div>

                      <p className="text-sm text-gray-700 line-clamp-2 mt-1">
                        {notification.message}
                      </p>

                      <div className="flex items-center gap-2 mt-2">
                        <span
                          className={`text-xs px-2 py-1 rounded-full font-medium ${getChannelColor(
                            notification.channel
                          )}`}
                        >
                          {notification.channel}
                        </span>
                        <span className="text-xs text-gray-500">
                          {formatTime(notification.created_at)}
                        </span>
                      </div>
                    </div>

                    {/* ACTIONS */}
                    <div className="flex gap-1 flex-shrink-0">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDelete(notification.id);
                        }}
                        className="p-1 hover:bg-gray-200 rounded transition"
                        title="Delete"
                      >
                        <Trash2 size={16} className="text-gray-500 hover:text-red-600" />
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* FOOTER */}
        <div className="p-4 border-t border-gray-200 text-center">
          <a
            href="/parent/notifications"
            className="text-sm font-semibold text-indigo-600 hover:text-indigo-700"
          >
            View all notifications →
          </a>
        </div>
      </div>
    </div>
  );
};

function formatTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;

  return date.toLocaleDateString();
}

export default NotificationCenter;
