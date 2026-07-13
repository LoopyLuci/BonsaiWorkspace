/**
 * Global Type Definitions for App Manager Mobile
 */

// User & Authentication
export interface User {
  id: string;
  email: string;
  name?: string;
  avatar?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AuthToken {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
  tokenType: 'Bearer';
}

export interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  token: AuthToken | null;
  loading: boolean;
  error: string | null;
}

// App Data
export interface AppMetadata {
  id: string;
  name: string;
  description: string;
  version: string;
  rating: number;
  downloads: number;
  category: AppCategory;
  icon: string;
  size: number;
  developer: string;
  updatedAt: string;
  permissions: Permission[];
}

export type AppCategory =
  | 'productivity'
  | 'entertainment'
  | 'utilities'
  | 'development'
  | 'social'
  | 'business';

export interface Permission {
  id: string;
  name: string;
  description: string;
  riskLevel: 'critical' | 'high' | 'medium' | 'low';
}

export interface AppState {
  status: 'installed' | 'available' | 'updating' | 'failed';
  installedVersion?: string;
  lastUpdated?: string;
  isFavorite: boolean;
  installationProgress?: number;
}

// Device & Sync
export interface Device {
  id: string;
  name: string;
  type: 'mobile' | 'desktop' | 'web';
  platform: 'ios' | 'android' | 'web' | 'macos' | 'windows';
  lastSync: string;
  isCurrentDevice: boolean;
}

export interface SyncState {
  isSyncing: boolean;
  lastSync: string | null;
  nextSync: string | null;
  pendingChanges: number;
  conflicts: SyncConflict[];
  status: 'idle' | 'syncing' | 'error' | 'paused';
}

export interface SyncConflict {
  id: string;
  resourceType: 'favorite' | 'setting' | 'installation';
  resourceId: string;
  localVersion: any;
  remoteVersion: any;
  timestamp: string;
  resolution?: 'local' | 'remote' | 'merged';
}

export interface ChangeLog {
  id: string;
  type: 'create' | 'update' | 'delete';
  resourceType: string;
  resourceId: string;
  timestamp: string;
  data: any;
  synced: boolean;
}

// Storage
export interface StoredApp extends AppMetadata {
  appState: AppState;
  cachedAt: string;
  isCached: boolean;
}

export interface LocalFavorite {
  appId: string;
  addedAt: string;
  synced: boolean;
  syncedAt?: string;
}

export interface LocalSettings {
  theme: 'light' | 'dark' | 'auto';
  language: 'en' | 'es' | 'fr' | 'de' | 'ja' | 'zh';
  notificationsEnabled: boolean;
  autoUpdate: boolean;
  syncFrequency: 'manual' | '1h' | '6h' | '24h';
  downloadQuality: 'low' | 'medium' | 'high';
  lastUpdated: string;
  synced: boolean;
}

// Network
export interface NetworkState {
  isConnected: boolean;
  isInternetReachable: boolean;
  type: 'wifi' | 'cellular' | 'none' | 'unknown';
  strength: number; // 0-100
}

// API Responses
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}

// Search & Filter
export interface SearchQuery {
  q: string;
  category?: AppCategory;
  minRating?: number;
  maxPrice?: number;
  sortBy?: 'name' | 'rating' | 'downloads' | 'recent';
  page?: number;
  limit?: number;
}

export interface SearchResult {
  items: AppMetadata[];
  total: number;
  query: string;
  executionTime: number;
}

// Notifications
export interface PushNotification {
  id: string;
  title: string;
  body: string;
  type: 'app_update' | 'favorite_update' | 'installation_complete' | 'settings_sync' | 'security_alert';
  data: Record<string, any>;
  timestamp: string;
  read: boolean;
}

// Review & Rating
export interface AppReview {
  id: string;
  appId: string;
  userId: string;
  rating: number;
  title: string;
  content: string;
  helpfulCount: number;
  timestamp: string;
}

// Installation
export interface Installation {
  id: string;
  appId: string;
  version: string;
  installDate: string;
  lastUsed?: string;
  size: number;
  updateAvailable: boolean;
  latestVersion?: string;
}

// Error & Exception
export interface AppError {
  code: string;
  message: string;
  details?: Record<string, any>;
  timestamp: string;
  retryable: boolean;
}

// UI State
export interface UIState {
  selectedTab: 'home' | 'browse' | 'favorites' | 'settings';
  loading: boolean;
  refreshing: boolean;
  modalVisible: boolean;
  selectedApp: AppMetadata | null;
  toast: {
    visible: boolean;
    message: string;
    type: 'info' | 'success' | 'error' | 'warning';
  };
}

// Navigation
export type RootStackParamList = {
  Auth: undefined;
  Main: undefined;
  AppDetails: { appId: string };
  Settings: undefined;
};

export type MainTabParamList = {
  Home: undefined;
  Browse: undefined;
  Favorites: undefined;
  Account: undefined;
};

// Offline Queue
export interface QueuedAction {
  id: string;
  type: 'sync_favorite' | 'sync_setting' | 'sync_installation';
  timestamp: string;
  data: any;
  retries: number;
  lastError?: string;
}

// Performance Metrics
export interface PerformanceMetrics {
  syncDuration: number;
  cacheHitRate: number;
  apiLatency: number;
  memoryUsage: number;
  batteryDrain: number;
}
