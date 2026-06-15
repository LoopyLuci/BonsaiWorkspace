// PATHFINDER Frontend - SettingsPage
// User preferences, GDPR data export, account management

import React, { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { authActions, uiActions } from '../store';
import apiClient from '../api-client';
import ConfirmDialog from '../components/ConfirmDialog';
import { Download, LogOut, Trash2, Moon, Globe, Bell } from 'lucide-react';

const SettingsPage: React.FC = () => {
  const navigate = useNavigate();
  const dispatch = useDispatch<AppDispatch>();
  const { user } = useSelector((state: RootState) => state.auth);
  const [isLoading, setIsLoading] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showLogoutConfirm, setShowLogoutConfirm] = useState(false);

  // Settings state
  const [email, setEmail] = useState(user?.email || '');
  const [firstName, setFirstName] = useState(user?.first_name || '');
  const [lastName, setLastName] = useState(user?.last_name || '');
  const [language, setLanguage] = useState('en');
  const [timezone, setTimezone] = useState('UTC');
  const [darkMode, setDarkMode] = useState(false);
  const [emailNotifications, setEmailNotifications] = useState(true);
  const [pushNotifications, setPushNotifications] = useState(true);

  // Handle profile update
  const handleUpdateProfile = async () => {
    if (!user) return;

    try {
      setIsLoading(true);
      await apiClient.updateProfile(user.id, {
        first_name: firstName,
        last_name: lastName,
      });

      dispatch(
        uiActions.showNotification({
          message: 'Profile updated successfully',
          type: 'success',
        })
      );
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to update profile',
          type: 'error',
        })
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Handle GDPR data export
  const handleExportData = async () => {
    if (!user) return;

    try {
      setIsLoading(true);

      // Fetch data export
      const response = await apiClient.exportData(user.id);

      // Create blob and download
      const blob = new Blob([JSON.stringify(response, null, 2)], {
        type: 'application/json',
      });
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `pathfinder-data-${new Date().toISOString().split('T')[0]}.json`;
      link.click();
      window.URL.revokeObjectURL(url);

      dispatch(
        uiActions.showNotification({
          message: 'Your data has been exported',
          type: 'success',
        })
      );
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to export data',
          type: 'error',
        })
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Handle account deletion
  const handleDeleteAccount = async () => {
    if (!user) return;

    try {
      setIsLoading(true);
      await apiClient.deleteAccount(user.id);

      // Clear auth
      dispatch(authActions.logout());

      dispatch(
        uiActions.showNotification({
          message: 'Account deleted successfully',
          type: 'success',
        })
      );

      navigate('/login');
    } catch (error) {
      dispatch(
        uiActions.showNotification({
          message: 'Failed to delete account',
          type: 'error',
        })
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Handle logout
  const handleLogout = async () => {
    try {
      setIsLoading(true);
      await apiClient.logout();
      dispatch(authActions.logout());
      navigate('/login');
    } catch (error) {
      dispatch(authActions.logout());
      navigate('/login');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="space-y-8 max-w-2xl">
      {/* HEADER */}
      <div className="bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg p-8 text-white">
        <h1 className="text-3xl font-bold mb-2">Settings</h1>
        <p className="text-indigo-100">Manage your account and preferences</p>
      </div>

      {/* PROFILE SECTION */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Profile Information</h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Email Address
            </label>
            <input
              type="email"
              value={email}
              disabled
              className="w-full px-4 py-2 border border-gray-300 rounded-lg bg-gray-50 text-gray-600"
            />
            <p className="text-xs text-gray-500 mt-1">Email cannot be changed</p>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                First Name
              </label>
              <input
                type="text"
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                disabled={isLoading}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Last Name
              </label>
              <input
                type="text"
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                disabled={isLoading}
              />
            </div>
          </div>

          <button
            onClick={handleUpdateProfile}
            disabled={isLoading}
            className="px-6 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-400 text-white font-semibold rounded-lg transition"
          >
            Save Changes
          </button>
        </div>
      </div>

      {/* PREFERENCES SECTION */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Preferences</h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <Globe className="inline mr-2" size={16} />
              Language
            </label>
            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              <option value="en">English</option>
              <option value="es">Español (Spanish)</option>
              <option value="fr">Français (French)</option>
              <option value="de">Deutsch (German)</option>
              <option value="ja">日本語 (Japanese)</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Timezone
            </label>
            <select
              value={timezone}
              onChange={(e) => setTimezone(e.target.value)}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              <option value="UTC">UTC</option>
              <option value="EST">Eastern Standard Time</option>
              <option value="CST">Central Standard Time</option>
              <option value="MST">Mountain Standard Time</option>
              <option value="PST">Pacific Standard Time</option>
            </select>
          </div>

          <div className="flex items-center justify-between">
            <label className="flex items-center gap-3">
              <Moon size={18} className="text-gray-600" />
              <span className="text-sm font-medium text-gray-700">Dark Mode</span>
            </label>
            <input
              type="checkbox"
              checked={darkMode}
              onChange={(e) => setDarkMode(e.target.checked)}
              className="w-5 h-5"
            />
          </div>
        </div>
      </div>

      {/* NOTIFICATIONS SECTION */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Notifications</h2>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <label className="flex items-center gap-3">
              <Bell size={18} className="text-gray-600" />
              <div>
                <p className="text-sm font-medium text-gray-700">Email Notifications</p>
                <p className="text-xs text-gray-500">Updates about your learning progress</p>
              </div>
            </label>
            <input
              type="checkbox"
              checked={emailNotifications}
              onChange={(e) => setEmailNotifications(e.target.checked)}
              className="w-5 h-5"
            />
          </div>

          <div className="flex items-center justify-between border-t pt-4">
            <label className="flex items-center gap-3">
              <Bell size={18} className="text-gray-600" />
              <div>
                <p className="text-sm font-medium text-gray-700">Push Notifications</p>
                <p className="text-xs text-gray-500">Daily reminders and alerts</p>
              </div>
            </label>
            <input
              type="checkbox"
              checked={pushNotifications}
              onChange={(e) => setPushNotifications(e.target.checked)}
              className="w-5 h-5"
            />
          </div>
        </div>
      </div>

      {/* DATA & PRIVACY SECTION */}
      <div className="bg-white rounded-lg p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Data & Privacy (GDPR)</h2>

        <div className="space-y-3">
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4">
            <p className="text-sm text-gray-700">
              Your data is your property. You can export, port, or delete it anytime.
              PATHFINDER never sells or shares personal information.
            </p>
          </div>

          <button
            onClick={handleExportData}
            disabled={isLoading}
            className="w-full flex items-center justify-center gap-2 px-6 py-3 border border-indigo-600 rounded-lg text-indigo-600 font-semibold hover:bg-indigo-50 disabled:opacity-50"
          >
            <Download size={20} />
            Export My Data (GDPR)
          </button>

          <p className="text-xs text-gray-600">
            Download all your personal data, learning progress, and exercise history in JSON format.
          </p>
        </div>
      </div>

      {/* ACCOUNT ACTIONS SECTION */}
      <div className="bg-white rounded-lg p-6 shadow border-t-4 border-red-500">
        <h2 className="text-xl font-bold text-gray-900 mb-6">Account Actions</h2>

        <div className="space-y-3">
          <button
            onClick={() => setShowLogoutConfirm(true)}
            disabled={isLoading}
            className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-gray-100 hover:bg-gray-200 text-gray-900 font-semibold rounded-lg disabled:opacity-50"
          >
            <LogOut size={20} />
            Log Out
          </button>

          <button
            onClick={() => setShowDeleteConfirm(true)}
            disabled={isLoading}
            className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-red-50 hover:bg-red-100 text-red-700 font-semibold rounded-lg disabled:opacity-50"
          >
            <Trash2 size={20} />
            Delete Account
          </button>

          <p className="text-xs text-gray-600">
            Deleting your account will permanently remove all data. This action cannot be undone.
          </p>
        </div>
      </div>

      {/* CONFIRMATION DIALOGS */}
      <ConfirmDialog
        title="Log Out?"
        message="You will be signed out of your account."
        confirmText="Log Out"
        onConfirm={handleLogout}
        isOpen={showLogoutConfirm}
        onCancel={() => setShowLogoutConfirm(false)}
        isDangerous={false}
      />

      <ConfirmDialog
        title="Delete Account?"
        message="This will permanently delete your account and all learning data. This action cannot be undone. Are you sure?"
        confirmText="Delete Account"
        onConfirm={handleDeleteAccount}
        isOpen={showDeleteConfirm}
        onCancel={() => setShowDeleteConfirm(false)}
        isDangerous={true}
      />
    </div>
  );
};

export default SettingsPage;
