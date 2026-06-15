// PATHFINDER Frontend - Redux Store
// State management for authentication, skills, exercises, progress

import { configureStore, createSlice, PayloadAction } from '@reduxjs/toolkit';
import type { User, Skill, LearnerSkillState, ProgressMetrics } from './api-client';

// ============================================================================
// AUTH SLICE
// ============================================================================

interface AuthState {
  user: User | null;
  token: string | null;
  isLoading: boolean;
  error: string | null;
  isAuthenticated: boolean;
}

const authSlice = createSlice({
  name: 'auth',
  initialState: {
    user: null,
    token: localStorage.getItem('auth_token'),
    isLoading: false,
    error: null,
    isAuthenticated: !!localStorage.getItem('auth_token'),
  } as AuthState,
  reducers: {
    setUser: (state, action: PayloadAction<User>) => {
      state.user = action.payload;
      state.isAuthenticated = true;
    },
    setToken: (state, action: PayloadAction<string>) => {
      state.token = action.payload;
      state.isAuthenticated = true;
      localStorage.setItem('auth_token', action.payload);
    },
    clearAuth: (state) => {
      state.user = null;
      state.token = null;
      state.isAuthenticated = false;
      localStorage.removeItem('auth_token');
    },
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.isLoading = action.payload;
    },
    setError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },
  },
});

// ============================================================================
// SKILLS SLICE
// ============================================================================

interface SkillsState {
  all: Skill[];
  current: Skill | null;
  isLoading: boolean;
  error: string | null;
}

const skillsSlice = createSlice({
  name: 'skills',
  initialState: {
    all: [],
    current: null,
    isLoading: false,
    error: null,
  } as SkillsState,
  reducers: {
    setSkills: (state, action: PayloadAction<Skill[]>) => {
      state.all = action.payload;
    },
    setCurrentSkill: (state, action: PayloadAction<Skill>) => {
      state.current = action.payload;
    },
    setSkillsLoading: (state, action: PayloadAction<boolean>) => {
      state.isLoading = action.payload;
    },
    setSkillsError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },
  },
});

// ============================================================================
// LEARNER STATE SLICE
// ============================================================================

interface LearnerStateSlice {
  skillStates: LearnerSkillState[];
  nextSkillsToReview: LearnerSkillState[];
  progress: ProgressMetrics | null;
  isLoading: boolean;
  error: string | null;
}

const learnerStateSlice = createSlice({
  name: 'learnerState',
  initialState: {
    skillStates: [],
    nextSkillsToReview: [],
    progress: null,
    isLoading: false,
    error: null,
  } as LearnerStateSlice,
  reducers: {
    setSkillStates: (state, action: PayloadAction<LearnerSkillState[]>) => {
      state.skillStates = action.payload;
    },
    updateSkillState: (state, action: PayloadAction<LearnerSkillState>) => {
      const index = state.skillStates.findIndex(s => s.id === action.payload.id);
      if (index !== -1) {
        state.skillStates[index] = action.payload;
      } else {
        state.skillStates.push(action.payload);
      }
    },
    setNextSkillsToReview: (state, action: PayloadAction<LearnerSkillState[]>) => {
      state.nextSkillsToReview = action.payload;
    },
    setProgress: (state, action: PayloadAction<ProgressMetrics>) => {
      state.progress = action.payload;
    },
    setLearnerLoading: (state, action: PayloadAction<boolean>) => {
      state.isLoading = action.payload;
    },
    setLearnerError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },
  },
});

// ============================================================================
// UI SLICE
// ============================================================================

interface UIState {
  sidebarOpen: boolean;
  darkMode: boolean;
  currentPage: string;
  notification: {
    show: boolean;
    message: string;
    type: 'success' | 'error' | 'info' | 'warning';
  } | null;
}

const uiSlice = createSlice({
  name: 'ui',
  initialState: {
    sidebarOpen: true,
    darkMode: localStorage.getItem('darkMode') === 'true',
    currentPage: 'dashboard',
    notification: null,
  } as UIState,
  reducers: {
    toggleSidebar: (state) => {
      state.sidebarOpen = !state.sidebarOpen;
    },
    toggleDarkMode: (state) => {
      state.darkMode = !state.darkMode;
      localStorage.setItem('darkMode', state.darkMode.toString());
    },
    setCurrentPage: (state, action: PayloadAction<string>) => {
      state.currentPage = action.payload;
    },
    showNotification: (
      state,
      action: PayloadAction<{
        message: string;
        type: 'success' | 'error' | 'info' | 'warning';
      }>
    ) => {
      state.notification = {
        show: true,
        message: action.payload.message,
        type: action.payload.type,
      };
    },
    clearNotification: (state) => {
      state.notification = null;
    },
  },
});

// ============================================================================
// CONFIGURE STORE
// ============================================================================

export const store = configureStore({
  reducer: {
    auth: authSlice.reducer,
    skills: skillsSlice.reducer,
    learnerState: learnerStateSlice.reducer,
    ui: uiSlice.reducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: ['auth/setToken'],
      },
    }),
});

// Export type definitions
export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

// Export action creators
export const authActions = authSlice.actions;
export const skillsActions = skillsSlice.actions;
export const learnerStateActions = learnerStateSlice.actions;
export const uiActions = uiSlice.actions;

export default store;
