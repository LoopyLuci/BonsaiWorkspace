import { describe, it, expect, beforeEach } from '@jest/globals';
import { storageService } from '../src/services/storage';
import { AppMetadata, LocalFavorite } from '../src/types';

describe('Storage Service', () => {
  beforeEach(async () => {
    await storageService.initialize();
    await storageService.clearCache();
  });

  describe('Database Initialization', () => {
    it('should initialize database successfully', async () => {
      const stats = await storageService.getCacheStats();
      expect(stats).toHaveProperty('appCount');
      expect(stats).toHaveProperty('favoriteCount');
      expect(stats).toHaveProperty('pendingChanges');
    });

    it('should create required tables', async () => {
      const stats = await storageService.getCacheStats();
      expect(typeof stats.appCount).toBe('number');
    });
  });

  describe('App Caching', () => {
    it('should cache apps successfully', async () => {
      const mockApps: AppMetadata[] = [
        {
          id: 'app-1',
          name: 'Test App 1',
          description: 'A test app',
          version: '1.0.0',
          rating: 4.5,
          downloads: 1000,
          category: 'productivity',
          icon: 'icon.png',
          size: 50,
          developer: 'Test Dev',
          updatedAt: new Date().toISOString(),
          permissions: [],
          appState: {
            status: 'available',
            isFavorite: false,
          },
        },
      ];

      await storageService.cacheApps(mockApps);
      const cached = await storageService.getCachedApps();

      expect(cached.length).toBe(1);
      expect(cached[0].id).toBe('app-1');
      expect(cached[0].name).toBe('Test App 1');
    });

    it('should retrieve cached apps within 24 hours', async () => {
      const mockApps: AppMetadata[] = [
        {
          id: 'app-2',
          name: 'Test App 2',
          description: 'Another test app',
          version: '2.0.0',
          rating: 4.8,
          downloads: 5000,
          category: 'entertainment',
          icon: 'icon2.png',
          size: 100,
          developer: 'Test Dev 2',
          updatedAt: new Date().toISOString(),
          permissions: [],
          appState: {
            status: 'installed',
            isFavorite: true,
          },
        },
      ];

      await storageService.cacheApps(mockApps);
      const cached = await storageService.getCachedApps();

      expect(cached.length).toBeGreaterThan(0);
      expect(cached.some(a => a.id === 'app-2')).toBe(true);
    });

    it('should handle multiple apps', async () => {
      const mockApps = Array.from({ length: 50 }, (_, i) => ({
        id: `app-${i}`,
        name: `App ${i}`,
        description: `Test app ${i}`,
        version: '1.0.0',
        rating: 4.0 + (i % 5) * 0.2,
        downloads: 1000 * (i + 1),
        category: ['productivity', 'entertainment', 'utilities', 'development', 'social'][i % 5] as any,
        icon: 'icon.png',
        size: 50 + i * 10,
        developer: `Dev ${i}`,
        updatedAt: new Date().toISOString(),
        permissions: [],
        appState: {
          status: 'available',
          isFavorite: false,
        },
      }));

      await storageService.cacheApps(mockApps);
      const cached = await storageService.getCachedApps();

      expect(cached.length).toBe(50);
    });
  });

  describe('Favorites Management', () => {
    it('should add favorite', async () => {
      const favorite: LocalFavorite = {
        appId: 'app-1',
        addedAt: new Date().toISOString(),
        synced: false,
      };

      await storageService.addFavorite(favorite);
      const favorites = await storageService.getFavorites();

      expect(favorites.length).toBe(1);
      expect(favorites[0].appId).toBe('app-1');
    });

    it('should remove favorite', async () => {
      const favorite: LocalFavorite = {
        appId: 'app-2',
        addedAt: new Date().toISOString(),
        synced: false,
      };

      await storageService.addFavorite(favorite);
      await storageService.removeFavorite('app-2');
      const favorites = await storageService.getFavorites();

      expect(favorites.length).toBe(0);
    });

    it('should track multiple favorites', async () => {
      const favorites = [
        { appId: 'app-1', addedAt: new Date().toISOString(), synced: false },
        { appId: 'app-2', addedAt: new Date().toISOString(), synced: false },
        { appId: 'app-3', addedAt: new Date().toISOString(), synced: false },
      ];

      for (const fav of favorites) {
        await storageService.addFavorite(fav);
      }

      const stored = await storageService.getFavorites();
      expect(stored.length).toBe(3);
    });
  });

  describe('Change Log & Sync Queue', () => {
    it('should queue changes', async () => {
      await storageService.queueChange({
        type: 'create',
        resourceType: 'favorite',
        resourceId: 'app-1',
        timestamp: new Date().toISOString(),
        data: { appId: 'app-1' },
      });

      const changes = await storageService.getPendingChanges();
      expect(changes.length).toBe(1);
      expect(changes[0].type).toBe('create');
    });

    it('should track multiple pending changes', async () => {
      const changeCount = 10;

      for (let i = 0; i < changeCount; i++) {
        await storageService.queueChange({
          type: i % 2 === 0 ? 'create' : 'delete',
          resourceType: 'favorite',
          resourceId: `app-${i}`,
          timestamp: new Date().toISOString(),
          data: {},
        });
      }

      const changes = await storageService.getPendingChanges();
      expect(changes.length).toBe(changeCount);
    });

    it('should mark changes as synced', async () => {
      await storageService.queueChange({
        type: 'create',
        resourceType: 'favorite',
        resourceId: 'app-1',
        timestamp: new Date().toISOString(),
        data: {},
      });

      const changes = await storageService.getPendingChanges();
      const changeIds = changes.map(c => c.id);

      await storageService.markChangesSynced(changeIds);

      const remainingChanges = await storageService.getPendingChanges();
      expect(remainingChanges.length).toBe(0);
    });
  });

  describe('Cache Statistics', () => {
    it('should return correct cache statistics', async () => {
      const mockApps = Array.from({ length: 10 }, (_, i) => ({
        id: `app-${i}`,
        name: `App ${i}`,
        description: 'Test',
        version: '1.0.0',
        rating: 4.0,
        downloads: 1000,
        category: 'productivity' as const,
        icon: 'icon.png',
        size: 50,
        developer: 'Dev',
        updatedAt: new Date().toISOString(),
        permissions: [],
        appState: { status: 'available' as const, isFavorite: false },
      }));

      await storageService.cacheApps(mockApps);

      const favorite: LocalFavorite = {
        appId: 'app-1',
        addedAt: new Date().toISOString(),
        synced: false,
      };
      await storageService.addFavorite(favorite);

      await storageService.queueChange({
        type: 'create',
        resourceType: 'favorite',
        resourceId: 'app-1',
        timestamp: new Date().toISOString(),
        data: {},
      });

      const stats = await storageService.getCacheStats();

      expect(stats.appCount).toBe(10);
      expect(stats.favoriteCount).toBeGreaterThan(0);
      expect(stats.pendingChanges).toBeGreaterThan(0);
    });
  });

  describe('Cache Clearing', () => {
    it('should clear cache completely', async () => {
      const mockApps = Array.from({ length: 5 }, (_, i) => ({
        id: `app-${i}`,
        name: `App ${i}`,
        description: 'Test',
        version: '1.0.0',
        rating: 4.0,
        downloads: 1000,
        category: 'productivity' as const,
        icon: 'icon.png',
        size: 50,
        developer: 'Dev',
        updatedAt: new Date().toISOString(),
        permissions: [],
        appState: { status: 'available' as const, isFavorite: false },
      }));

      await storageService.cacheApps(mockApps);
      await storageService.clearCache();

      const stats = await storageService.getCacheStats();
      expect(stats.appCount).toBe(0);
    });
  });

  describe('Error Handling', () => {
    it('should handle empty cache gracefully', async () => {
      const cached = await storageService.getCachedApps();
      expect(Array.isArray(cached)).toBe(true);
    });

    it('should handle empty favorites gracefully', async () => {
      const favorites = await storageService.getFavorites();
      expect(Array.isArray(favorites)).toBe(true);
      expect(favorites.length).toBe(0);
    });

    it('should handle empty pending changes gracefully', async () => {
      const changes = await storageService.getPendingChanges();
      expect(Array.isArray(changes)).toBe(true);
    });
  });
});
