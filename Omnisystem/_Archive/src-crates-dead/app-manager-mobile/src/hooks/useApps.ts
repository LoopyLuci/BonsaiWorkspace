import { useContext } from 'react';
import { AppContext } from '../context/AppContext';
import { AppMetadata, SearchQuery } from '../types';

/**
 * Custom hook for accessing app context
 */
export function useApps() {
  const context = useContext(AppContext);

  if (!context) {
    throw new Error('useApps must be used within AppProvider');
  }

  return context;
}

/**
 * Hook for fetching apps list
 */
export function useAppsList() {
  const { apps, loading, error, getApps } = useApps();

  return {
    apps,
    loading,
    error,
    refetch: getApps,
  };
}

/**
 * Hook for searching apps
 */
export function useAppSearch() {
  const { searchResults, loading, error, searchApps } = useApps();

  return {
    results: searchResults?.items || [],
    total: searchResults?.total || 0,
    loading,
    error,
    search: searchApps,
  };
}

/**
 * Hook for app favorites
 */
export function useFavorites() {
  const { favorites, toggleFavorite, isFavorited } = useApps();

  return {
    favorites,
    toggleFavorite,
    isFavorited,
    count: favorites.length,
  };
}

/**
 * Hook for app installation
 */
export function useInstallation(appId: string) {
  const { apps, installApp, uninstallApp } = useApps();

  const app = apps.find(a => a.id === appId);
  const isInstalled = app?.appState.status === 'installed';
  const installProgress = app?.appState.installationProgress;

  return {
    isInstalled,
    installProgress,
    install: () => installApp(appId),
    uninstall: () => uninstallApp(appId),
  };
}

/**
 * Hook for app details
 */
export function useAppDetails(appId: string) {
  const { apps, getAppDetails } = useApps();

  const app = apps.find(a => a.id === appId);

  return {
    app,
    loading: !app,
    fetch: () => getAppDetails(appId),
  };
}

/**
 * Hook for app ratings
 */
export function useAppRating(appId: string) {
  const { apps, rateApp } = useApps();

  const app = apps.find(a => a.id === appId);
  const currentRating = app?.rating || 0;

  return {
    currentRating,
    rate: (rating: number) => rateApp(appId, rating),
  };
}

/**
 * Hook for installed apps
 */
export function useInstalledApps() {
  const { getInstalledApps } = useApps();

  return {
    fetch: getInstalledApps,
  };
}

/**
 * Hook for cache operations
 */
export function useAppCache() {
  const { refreshApps, clearCache, getCachedApps } = useApps();

  return {
    refresh: refreshApps,
    clear: clearCache,
    getCached: getCachedApps,
  };
}
