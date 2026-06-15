import React, { createContext, useCallback, useState, useEffect } from 'react';
import { AppMetadata, SearchQuery, SearchResult, LocalFavorite } from '../types';
import * as AppService from '../services/api';
import * as StorageService from '../services/storage';

interface AppContextType {
  apps: AppMetadata[];
  favorites: LocalFavorite[];
  searchResults: SearchResult | null;
  loading: boolean;
  error: string | null;

  // App operations
  getApps: (page?: number, limit?: number) => Promise<void>;
  searchApps: (query: SearchQuery) => Promise<void>;
  getAppDetails: (appId: string) => Promise<AppMetadata | null>;

  // Favorites
  toggleFavorite: (appId: string) => Promise<void>;
  isFavorited: (appId: string) => boolean;
  getFavorites: () => Promise<AppMetadata[]>;

  // Installation
  installApp: (appId: string) => Promise<void>;
  uninstallApp: (appId: string) => Promise<void>;
  getInstalledApps: () => Promise<AppMetadata[]>;

  // Ratings & Reviews
  rateApp: (appId: string, rating: number) => Promise<void>;
  getReviews: (appId: string) => Promise<any[]>;

  // Cache
  refreshApps: () => Promise<void>;
  clearCache: () => Promise<void>;
  getCachedApps: () => Promise<AppMetadata[]>;
}

export const AppContext = createContext<AppContextType | undefined>(undefined);

interface AppProviderProps {
  children: React.ReactNode;
}

export const AppProvider: React.FC<AppProviderProps> = ({ children }) => {
  const [apps, setApps] = useState<AppMetadata[]>([]);
  const [favorites, setFavorites] = useState<LocalFavorite[]>([]);
  const [searchResults, setSearchResults] = useState<SearchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load initial data
  useEffect(() => {
    const loadInitialData = async () => {
      try {
        // Load cached apps first
        const cachedApps = await StorageService.getCachedApps();
        setApps(cachedApps);

        // Load favorites
        const favs = await StorageService.getFavorites();
        setFavorites(favs);

        // Try to fetch fresh data
        await getApps();
      } catch (err) {
        console.error('Failed to load initial data:', err);
      }
    };

    loadInitialData();
  }, []);

  const getApps = useCallback(async (page = 1, limit = 50) => {
    setLoading(true);
    setError(null);

    try {
      const result = await AppService.getApps(page, limit);
      setApps(result.items);

      // Cache apps
      await StorageService.cacheApps(result.items);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch apps');
      // Fall back to cached apps
      const cachedApps = await StorageService.getCachedApps();
      setApps(cachedApps);
    } finally {
      setLoading(false);
    }
  }, []);

  const searchApps = useCallback(async (query: SearchQuery) => {
    setLoading(true);
    setError(null);

    try {
      const result = await AppService.searchApps(query);
      setSearchResults(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
      setSearchResults(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const getAppDetails = useCallback(
    async (appId: string): Promise<AppMetadata | null> => {
      try {
        const app = await AppService.getAppDetails(appId);
        // Update in cache
        setApps(prev =>
          prev.map(a => (a.id === appId ? app : a))
        );
        return app;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch app details');
        return null;
      }
    },
    []
  );

  const toggleFavorite = useCallback(
    async (appId: string) => {
      const isFav = favorites.some(f => f.appId === appId);

      try {
        if (isFav) {
          await StorageService.removeFavorite(appId);
          setFavorites(prev => prev.filter(f => f.appId !== appId));
        } else {
          const favorite: LocalFavorite = {
            appId,
            addedAt: new Date().toISOString(),
            synced: false,
          };
          await StorageService.addFavorite(favorite);
          setFavorites(prev => [...prev, favorite]);
        }

        // Queue for sync
        await StorageService.queueChange({
          type: isFav ? 'delete' : 'create',
          resourceType: 'favorite',
          resourceId: appId,
          data: { appId },
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update favorite');
        throw err;
      }
    },
    [favorites]
  );

  const isFavorited = useCallback(
    (appId: string) => favorites.some(f => f.appId === appId),
    [favorites]
  );

  const getFavorites = useCallback(async () => {
    try {
      const favApps = await Promise.all(
        favorites.map(fav => AppService.getAppDetails(fav.appId))
      );
      return favApps.filter(app => app !== null) as AppMetadata[];
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch favorites');
      return [];
    }
  }, [favorites]);

  const installApp = useCallback(async (appId: string) => {
    try {
      await AppService.installApp(appId);

      // Queue for sync
      await StorageService.queueChange({
        type: 'create',
        resourceType: 'installation',
        resourceId: appId,
        data: { appId, timestamp: new Date().toISOString() },
      });

      // Update local state
      setApps(prev =>
        prev.map(a =>
          a.id === appId
            ? { ...a, appState: { ...a.appState, status: 'installed' } }
            : a
        )
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Installation failed');
      throw err;
    }
  }, []);

  const uninstallApp = useCallback(async (appId: string) => {
    try {
      await AppService.uninstallApp(appId);

      // Queue for sync
      await StorageService.queueChange({
        type: 'delete',
        resourceType: 'installation',
        resourceId: appId,
        data: { appId, timestamp: new Date().toISOString() },
      });

      // Update local state
      setApps(prev =>
        prev.map(a =>
          a.id === appId
            ? { ...a, appState: { ...a.appState, status: 'available' } }
            : a
        )
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Uninstall failed');
      throw err;
    }
  }, []);

  const getInstalledApps = useCallback(async () => {
    try {
      return apps.filter(a => a.appState.status === 'installed');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch installed apps');
      return [];
    }
  }, [apps]);

  const rateApp = useCallback(async (appId: string, rating: number) => {
    try {
      await AppService.rateApp(appId, rating);

      // Queue for sync
      await StorageService.queueChange({
        type: 'create',
        resourceType: 'review',
        resourceId: `${appId}-${Date.now()}`,
        data: { appId, rating, timestamp: new Date().toISOString() },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Rating failed');
      throw err;
    }
  }, []);

  const getReviews = useCallback(
    async (appId: string) => {
      try {
        return await AppService.getReviews(appId);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch reviews');
        return [];
      }
    },
    []
  );

  const refreshApps = useCallback(async () => {
    await getApps();
  }, [getApps]);

  const clearCache = useCallback(async () => {
    try {
      await StorageService.clearCache();
      setApps([]);
      setFavorites([]);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to clear cache');
      throw err;
    }
  }, []);

  const getCachedApps = useCallback(
    async () => StorageService.getCachedApps(),
    []
  );

  const value: AppContextType = {
    apps,
    favorites,
    searchResults,
    loading,
    error,
    getApps,
    searchApps,
    getAppDetails,
    toggleFavorite,
    isFavorited,
    getFavorites,
    installApp,
    uninstallApp,
    getInstalledApps,
    rateApp,
    getReviews,
    refreshApps,
    clearCache,
    getCachedApps,
  };

  return (
    <AppContext.Provider value={value}>
      {children}
    </AppContext.Provider>
  );
};
