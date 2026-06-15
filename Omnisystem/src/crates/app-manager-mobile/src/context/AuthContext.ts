import React, { createContext, useCallback, useState, useEffect } from 'react';
import { AuthState, AuthToken, User } from '../types';
import * as SecureStorage from '../services/secureStorage';

interface AuthContextType {
  auth: AuthState;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  register: (email: string, password: string, name: string) => Promise<void>;
  refreshToken: () => Promise<void>;
  isLoading: boolean;
  error: string | null;
}

export const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: React.ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [auth, setAuth] = useState<AuthState>({
    isAuthenticated: false,
    user: null,
    token: null,
    loading: true,
    error: null,
  });

  // Initialize auth state from storage
  useEffect(() => {
    const initializeAuth = async () => {
      try {
        const storedToken = await SecureStorage.getToken('accessToken');
        const storedUser = await SecureStorage.getUser();

        if (storedToken && storedUser) {
          setAuth({
            isAuthenticated: true,
            user: storedUser,
            token: storedToken as AuthToken,
            loading: false,
            error: null,
          });
        } else {
          setAuth(prev => ({ ...prev, loading: false }));
        }
      } catch (err) {
        setAuth(prev => ({
          ...prev,
          loading: false,
          error: 'Failed to load auth state',
        }));
      }
    };

    initializeAuth();
  }, []);

  const login = useCallback(
    async (email: string, password: string) => {
      setAuth(prev => ({ ...prev, loading: true, error: null }));

      try {
        // Call to backend API
        const response = await fetch('https://api.app-manager.cloud/auth/login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, password }),
        });

        if (!response.ok) {
          throw new Error('Login failed');
        }

        const { user, token } = await response.json();

        // Store securely
        await SecureStorage.setToken('accessToken', token.accessToken);
        await SecureStorage.setToken('refreshToken', token.refreshToken);
        await SecureStorage.setUser(user);

        setAuth({
          isAuthenticated: true,
          user,
          token,
          loading: false,
          error: null,
        });
      } catch (err) {
        setAuth(prev => ({
          ...prev,
          loading: false,
          error: err instanceof Error ? err.message : 'Login failed',
        }));
        throw err;
      }
    },
    []
  );

  const logout = useCallback(async () => {
    try {
      // Call logout endpoint
      await fetch('https://api.app-manager.cloud/auth/logout', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${auth.token?.accessToken}`,
        },
      }).catch(() => {
        // Ignore errors on logout
      });

      // Clear storage
      await SecureStorage.clearAll();

      setAuth({
        isAuthenticated: false,
        user: null,
        token: null,
        loading: false,
        error: null,
      });
    } catch (err) {
      setAuth(prev => ({
        ...prev,
        error: 'Logout failed',
      }));
    }
  }, [auth.token?.accessToken]);

  const register = useCallback(
    async (email: string, password: string, name: string) => {
      setAuth(prev => ({ ...prev, loading: true, error: null }));

      try {
        const response = await fetch(
          'https://api.app-manager.cloud/auth/register',
          {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, password, name }),
          }
        );

        if (!response.ok) {
          throw new Error('Registration failed');
        }

        const { user, token } = await response.json();

        // Store securely
        await SecureStorage.setToken('accessToken', token.accessToken);
        await SecureStorage.setToken('refreshToken', token.refreshToken);
        await SecureStorage.setUser(user);

        setAuth({
          isAuthenticated: true,
          user,
          token,
          loading: false,
          error: null,
        });
      } catch (err) {
        setAuth(prev => ({
          ...prev,
          loading: false,
          error: err instanceof Error ? err.message : 'Registration failed',
        }));
        throw err;
      }
    },
    []
  );

  const refreshToken = useCallback(async () => {
    if (!auth.token?.refreshToken) {
      throw new Error('No refresh token available');
    }

    try {
      const response = await fetch(
        'https://api.app-manager.cloud/auth/refresh',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${auth.token.refreshToken}`,
          },
        }
      );

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const { token } = await response.json();

      await SecureStorage.setToken('accessToken', token.accessToken);
      await SecureStorage.setToken('refreshToken', token.refreshToken);

      setAuth(prev => ({
        ...prev,
        token,
      }));
    } catch (err) {
      // Clear auth on refresh failure
      await logout();
      throw err;
    }
  }, [auth.token?.refreshToken, logout]);

  const value: AuthContextType = {
    auth,
    login,
    logout,
    register,
    refreshToken,
    isLoading: auth.loading,
    error: auth.error,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};
