// PATHFINDER Frontend - Main App Component
// React 19 + TypeScript + Redux + React Router
// Week 4 Implementation

import React, { useEffect, useState } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { useDispatch, useSelector } from 'react-redux';
import type { RootState, AppDispatch } from './store';
import { authActions } from './store';
import apiClient from './api-client';

// Pages
import LoginPage from './pages/LoginPage';
import SignupPage from './pages/SignupPage';
import DashboardPage from './pages/DashboardPage';
import LessonPage from './pages/LessonPage';
import ExercisePage from './pages/ExercisePage';
import ProgressPage from './pages/ProgressPage';
import SettingsPage from './pages/SettingsPage';

// Components
import Layout from './components/Layout';
import ProtectedRoute from './components/ProtectedRoute';
import LoadingSpinner from './components/LoadingSpinner';

// ============================================================================
// MAIN APP COMPONENT
// ============================================================================

const App: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const { isAuthenticated, isLoading } = useSelector((state: RootState) => state.auth);
  const [appReady, setAppReady] = useState(false);

  // ========================================================================
  // INITIALIZATION
  // ========================================================================

  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Check if user is still authenticated
        if (apiClient.isAuthenticated()) {
          dispatch(authActions.setLoading(true));
          const user = await apiClient.getProfile();
          dispatch(authActions.setUser(user));
        }
      } catch (error) {
        // Clear auth if token is invalid
        dispatch(authActions.clearAuth());
      } finally {
        dispatch(authActions.setLoading(false));
        setAppReady(true);
      }
    };

    initializeApp();

    // Register service worker for offline support
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker
        .register('/sw.js')
        .then((registration) => {
          console.log('✓ Service Worker registered', registration);
        })
        .catch((error) => {
          console.warn('Service Worker registration failed:', error);
        });
    }
  }, [dispatch]);

  // ========================================================================
  // LOADING STATE
  // ========================================================================

  if (!appReady || isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
        <LoadingSpinner />
      </div>
    );
  }

  // ========================================================================
  // RENDER
  // ========================================================================

  return (
    <Router>
      <Routes>
        {/* PUBLIC ROUTES */}
        <Route path="/login" element={<LoginPage />} />
        <Route path="/signup" element={<SignupPage />} />

        {/* PROTECTED ROUTES */}
        <Route element={<ProtectedRoute isAuthenticated={isAuthenticated} />}>
          <Route element={<Layout />}>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/skills/:skillId/lessons/:lessonId" element={<LessonPage />} />
            <Route path="/exercises/:exerciseId" element={<ExercisePage />} />
            <Route path="/progress" element={<ProgressPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>
        </Route>

        {/* FALLBACK */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Router>
  );
};

export default App;

// ============================================================================
// KEY PAGES (Stubs for detailed implementation)
// ============================================================================

// These pages would be implemented in separate files with full UI
// This demonstrates the routing structure and component hierarchy

/*
PAGES TO IMPLEMENT:

1. LoginPage (/login)
   - Email/password form
   - Submit to /v1/auth/login
   - Store JWT token
   - Redirect to dashboard

2. SignupPage (/signup)
   - Registration form
   - Submit to /v1/auth/register
   - Age/parental consent check
   - Store JWT token
   - Redirect to dashboard

3. DashboardPage (/)
   - Show next skills to review
   - Quick access to lessons
   - Progress summary
   - Daily metrics
   - Streak display

4. LessonPage (/skills/:skillId/lessons/:lessonId)
   - Show lesson title & objectives
   - Display exercises in sequence
   - Navigate between exercises
   - Track progress through lesson

5. ExercisePage (/exercises/:exerciseId)
   - Render exercise (multiple choice, translation, etc.)
   - Time the user
   - Record attempt on submit
   - Show feedback (correct/incorrect)
   - Calculate next review time
   - Show learning curve prediction

6. ProgressPage (/progress)
   - Overall progress metrics
   - Learning curves per skill
   - Monthly/daily stats
   - Streak information
   - Skills mastered

7. SettingsPage (/settings)
   - Profile management
   - Language preferences
   - Notification settings
   - Dark mode toggle
   - Data export (GDPR)
   - Account deletion
*/
