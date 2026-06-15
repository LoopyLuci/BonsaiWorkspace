// PATHFINDER Frontend - useAuth Hook
// Custom hook for authentication state and functions

import { useSelector, useDispatch } from 'react-redux';
import type { RootState, AppDispatch } from '../store';
import { authActions } from '../store';
import apiClient from '../api-client';

interface UseAuthReturn {
  user: any | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  register: (data: any) => Promise<void>;
  getToken: () => string | null;
  updateProfile: (data: any) => Promise<void>;
}

export const useAuth = (): UseAuthReturn => {
  const dispatch = useDispatch<AppDispatch>();
  const { user, token, isLoading } = useSelector((state: RootState) => state.auth);

  const login = async (email: string, password: string) => {
    try {
      dispatch(authActions.setLoading(true));
      const response = await apiClient.login(email, password);
      dispatch(authActions.setAuthToken(response.token));
      dispatch(authActions.setUser(response.user));
    } finally {
      dispatch(authActions.setLoading(false));
    }
  };

  const logout = async () => {
    try {
      dispatch(authActions.setLoading(true));
      await apiClient.logout();
    } finally {
      dispatch(authActions.logout());
      dispatch(authActions.setLoading(false));
    }
  };

  const register = async (data: any) => {
    try {
      dispatch(authActions.setLoading(true));
      const response = await apiClient.register(data);
      dispatch(authActions.setAuthToken(response.token));
      dispatch(authActions.setUser(response.user));
    } finally {
      dispatch(authActions.setLoading(false));
    }
  };

  const getToken = (): string | null => {
    return token || localStorage.getItem('authToken');
  };

  const updateProfile = async (data: any) => {
    try {
      if (!user) throw new Error('No user logged in');
      dispatch(authActions.setLoading(true));
      const updatedUser = await apiClient.updateProfile(user.id, data);
      dispatch(authActions.setUser(updatedUser));
    } finally {
      dispatch(authActions.setLoading(false));
    }
  };

  return {
    user,
    token,
    isAuthenticated: !!user && !!token,
    isLoading,
    login,
    logout,
    register,
    getToken,
    updateProfile,
  };
};
