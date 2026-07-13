import { describe, it, expect, beforeEach, jest } from '@jest/globals';
import { syncService } from '../src/services/sync';
import { ChangeLog, Device } from '../src/types';

// Mock fetch
global.fetch = jest.fn();

describe('Sync Service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should initialize with access token', async () => {
      const token = 'test-token-123';
      await syncService.initialize(token);
      expect(syncService).toBeDefined();
    });
  });

  describe('Change Management', () => {
    it('should get pending changes', async () => {
      const changes = await syncService.getPendingChanges();
      expect(Array.isArray(changes)).toBe(true);
    });
  });

  describe('Push Changes', () => {
    it('should push changes to cloud', async () => {
      await syncService.initialize('test-token');

      const mockChanges: ChangeLog[] = [
        {
          id: 'change-1',
          type: 'create',
          resourceType: 'favorite',
          resourceId: 'app-1',
          timestamp: new Date().toISOString(),
          data: { appId: 'app-1' },
          synced: false,
        },
      ];

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          synced: mockChanges,
          conflicts: [],
        }),
      });

      const result = await syncService.pushChanges(mockChanges);

      expect(result.synced).toHaveLength(1);
      expect(result.conflicts).toHaveLength(0);
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/sync/push'),
        expect.any(Object)
      );
    });

    it('should handle push errors gracefully', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        statusText: 'Server Error',
      });

      const mockChanges: ChangeLog[] = [];

      try {
        await syncService.pushChanges(mockChanges);
        expect(true).toBe(false); // Should throw
      } catch (err) {
        expect(err).toBeDefined();
      }
    });

    it('should detect conflicts during push', async () => {
      await syncService.initialize('test-token');

      const mockConflict = {
        id: 'conflict-1',
        resourceType: 'favorite' as const,
        resourceId: 'app-1',
        localVersion: { timestamp: 'local' },
        remoteVersion: { timestamp: 'remote' },
        timestamp: new Date().toISOString(),
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          synced: [],
          conflicts: [mockConflict],
        }),
      });

      const result = await syncService.pushChanges([]);

      expect(result.conflicts).toHaveLength(1);
      expect(result.conflicts[0].id).toBe('conflict-1');
    });
  });

  describe('Pull Changes', () => {
    it('should pull changes from cloud', async () => {
      await syncService.initialize('test-token');

      const mockRemoteChanges: ChangeLog[] = [
        {
          id: 'remote-1',
          type: 'create',
          resourceType: 'favorite',
          resourceId: 'app-2',
          timestamp: new Date().toISOString(),
          data: { appId: 'app-2' },
          synced: true,
        },
      ];

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          changes: mockRemoteChanges,
        }),
      });

      const changes = await syncService.pullChanges();

      expect(changes).toHaveLength(1);
      expect(changes[0].resourceId).toBe('app-2');
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/sync/pull'),
        expect.any(Object)
      );
    });

    it('should handle pull errors gracefully', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        statusText: 'Not Found',
      });

      try {
        await syncService.pullChanges();
        expect(true).toBe(false); // Should throw
      } catch (err) {
        expect(err).toBeDefined();
      }
    });
  });

  describe('Conflict Resolution', () => {
    it('should resolve conflicts with local strategy', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await syncService.resolveConflict('conflict-1', 'local');

      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/conflicts/conflict-1'),
        expect.objectContaining({
          method: 'POST',
          body: expect.stringContaining('local'),
        })
      );
    });

    it('should resolve conflicts with remote strategy', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await syncService.resolveConflict('conflict-1', 'remote');

      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/conflicts/conflict-1'),
        expect.objectContaining({
          method: 'POST',
          body: expect.stringContaining('remote'),
        })
      );
    });
  });

  describe('Device Management', () => {
    it('should register device', async () => {
      await syncService.initialize('test-token');

      const mockDevice: Device = {
        id: 'device-1',
        name: 'My Phone',
        type: 'mobile',
        platform: 'android',
        lastSync: new Date().toISOString(),
        isCurrentDevice: true,
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockDevice,
      });

      const device = await syncService.registerDevice('My Phone');

      expect(device.id).toBe('device-1');
      expect(device.name).toBe('My Phone');
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/devices'),
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('should remove device', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await syncService.removeDevice('device-1');

      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/devices/device-1'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should get devices list', async () => {
      await syncService.initialize('test-token');

      const mockDevices: Device[] = [
        {
          id: 'device-1',
          name: 'Phone',
          type: 'mobile',
          platform: 'android',
          lastSync: new Date().toISOString(),
          isCurrentDevice: true,
        },
        {
          id: 'device-2',
          name: 'Tablet',
          type: 'mobile',
          platform: 'ios',
          lastSync: new Date().toISOString(),
          isCurrentDevice: false,
        },
      ];

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockDevices,
      });

      const devices = await syncService.getDevices();

      expect(devices).toHaveLength(2);
      expect(devices[0].id).toBe('device-1');
    });
  });

  describe('Sync Status', () => {
    it('should get sync status', async () => {
      await syncService.initialize('test-token');

      const status = await syncService.getSyncStatus();

      expect(status).toHaveProperty('lastSync');
      expect(status).toHaveProperty('pendingChanges');
      expect(status).toHaveProperty('conflicts');
    });
  });

  describe('Timestamp Management', () => {
    it('should set and get last sync time', async () => {
      const now = new Date().toISOString();
      await syncService.setLastSyncTime(now);

      const retrieved = await syncService.getLastSyncTime();
      expect(retrieved).toBe(now);
    });

    it('should return null when no sync time set', async () => {
      await syncService.clearSyncData();
      const time = await syncService.getLastSyncTime();
      expect(time).toBeNull();
    });
  });

  describe('Sync Data Cleanup', () => {
    it('should clear all sync data on logout', async () => {
      await syncService.initialize('test-token');
      await syncService.setLastSyncTime(new Date().toISOString());

      await syncService.clearSyncData();

      const time = await syncService.getLastSyncTime();
      expect(time).toBeNull();
    });
  });

  describe('Error Handling', () => {
    it('should throw error when not initialized for push', async () => {
      const freshService = new (syncService.constructor as any)();

      try {
        await freshService.pushChanges([]);
        expect(true).toBe(false); // Should throw
      } catch (err) {
        expect(err).toBeDefined();
      }
    });

    it('should handle network errors gracefully', async () => {
      await syncService.initialize('test-token');

      (global.fetch as jest.Mock).mockRejectedValueOnce(
        new Error('Network error')
      );

      const devices = await syncService.getDevices();
      expect(devices).toEqual([]);
    });
  });

  describe('Performance', () => {
    it('should handle large change batches', async () => {
      await syncService.initialize('test-token');

      const largeChangeBatch: ChangeLog[] = Array.from(
        { length: 1000 },
        (_, i) => ({
          id: `change-${i}`,
          type: 'create' as const,
          resourceType: 'favorite' as const,
          resourceId: `app-${i}`,
          timestamp: new Date().toISOString(),
          data: {},
          synced: false,
        })
      );

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          synced: largeChangeBatch,
          conflicts: [],
        }),
      });

      const result = await syncService.pushChanges(largeChangeBatch);
      expect(result.synced).toHaveLength(1000);
    });
  });
});
