import { useContext } from 'react';
import { AuthContext } from '../context/AuthContext';

/**
 * Custom hook for accessing authentication context
 */
export function useAuth() {
  const context = useContext(AuthContext);

  if (!context) {
    throw new Error('useAuth must be used within AuthProvider');
  }

  return {
    ...context,
    isLoggedIn: context.auth.isAuthenticated,
    user: context.auth.user,
    token: context.auth.token,
    isLoading: context.auth.loading,
    error: context.auth.error,
  };
}

/**
 * Hook for checking authentication status
 */
export function useIsAuthenticated() {
  const { auth } = useAuth();
  return auth.isAuthenticated;
}

/**
 * Hook for getting current user
 */
export function useCurrentUser() {
  const { auth } = useAuth();
  return auth.user;
}

/**
 * Hook for getting auth token
 */
export function useAuthToken() {
  const { auth } = useAuth();
  return auth.token?.accessToken;
}
