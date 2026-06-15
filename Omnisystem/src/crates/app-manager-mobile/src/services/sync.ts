import AsyncStorage from '@react-native-async-storage/async-storage';
import { ChangeLog, Device, SyncConflict } from '../types';
import { storageService } from './storage';

/**
 * Sync Engine for cloud synchronization
 */
class SyncService {
  private baseUrl = 'https://api.app-manager.cloud';
  private accessToken: string | null = null;

  /**
   * Initialize sync service with access token
   */
  async initialize(token: string): Promise<void> {
    this.accessToken = token;
  }

  /**
   * Get pending changes from local storage
   */
  async getPendingChanges(): Promise<ChangeLog[]> {
    return storageService.getPendingChanges();
  }

  /**
   * Push local changes to cloud
   */
  async pushChanges(changes: ChangeLog[]): Promise<{ synced: ChangeLog[]; conflicts: SyncConflict[] }> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const response = await fetch(`${this.baseUrl}/sync/push`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.accessToken}`,
        },
        body: JSON.stringify({
          changes,
          timestamp: new Date().toISOString(),
        }),
      });

      if (!response.ok) {
        throw new Error(`Push failed: ${response.statusText}`);
      }

      const result = await response.json();

      // Mark synced changes in local storage
      const syncedIds = result.synced?.map((c: ChangeLog) => c.id) || [];
      if (syncedIds.length > 0) {
        await storageService.markChangesSynced(syncedIds);
      }

      return {
        synced: result.synced || [],
        conflicts: result.conflicts || [],
      };
    } catch (err) {
      console.error('Push changes failed:', err);
      throw err;
    }
  }

  /**
   * Pull remote changes from cloud
   */
  async pullChanges(): Promise<ChangeLog[]> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const lastSync = await this.getLastSyncTime();
      const response = await fetch(
        `${this.baseUrl}/sync/pull?since=${lastSync || new Date(0).toISOString()}`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${this.accessToken}`,
          },
        }
      );

      if (!response.ok) {
        throw new Error(`Pull failed: ${response.statusText}`);
      }

      const result = await response.json();
      return result.changes || [];
    } catch (err) {
      console.error('Pull changes failed:', err);
      throw err;
    }
  }

  /**
   * Merge remote changes into local storage
   */
  async mergeChanges(remoteChanges: ChangeLog[]): Promise<void> {
    // Group changes by resource
    const changesByResource = new Map<string, ChangeLog[]>();

    for (const change of remoteChanges) {
      const key = `${change.resourceType}:${change.resourceId}`;
      if (!changesByResource.has(key)) {
        changesByResource.set(key, []);
      }
      changesByResource.get(key)!.push(change);
    }

    // Apply changes in order
    for (const changes of changesByResource.values()) {
      for (const change of changes) {
        await this.applyChange(change);
      }
    }
  }

  /**
   * Apply a single remote change to local state
   */
  private async applyChange(change: ChangeLog): Promise<void> {
    switch (change.resourceType) {
      case 'favorite':
        if (change.type === 'create') {
          await storageService.addFavorite({
            appId: change.resourceId,
            addedAt: change.timestamp,
            synced: true,
            syncedAt: new Date().toISOString(),
          });
        } else if (change.type === 'delete') {
          await storageService.removeFavorite(change.resourceId);
        }
        break;

      case 'installation':
        // Handle installation updates
        if (change.type === 'create') {
          // Installation created on another device
          // UI will handle displaying to user
        } else if (change.type === 'delete') {
          // Installation removed on another device
        }
        break;

      case 'setting':
        // Handle setting updates
        if (change.type === 'update') {
          const settings = await storageService.getSettings();
          if (settings) {
            const updatedSettings = {
              ...settings,
              ...change.data,
              synced: true,
            };
            await storageService.saveSettings(updatedSettings);
          }
        }
        break;
    }
  }

  /**
   * Resolve a sync conflict
   */
  async resolveConflict(
    conflictId: string,
    resolution: 'local' | 'remote' | 'merged'
  ): Promise<void> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const response = await fetch(`${this.baseUrl}/sync/conflicts/${conflictId}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.accessToken}`,
        },
        body: JSON.stringify({ resolution }),
      });

      if (!response.ok) {
        throw new Error(`Conflict resolution failed: ${response.statusText}`);
      }
    } catch (err) {
      console.error('Conflict resolution failed:', err);
      throw err;
    }
  }

  /**
   * Register current device
   */
  async registerDevice(name: string): Promise<Device> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const response = await fetch(`${this.baseUrl}/devices`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.accessToken}`,
        },
        body: JSON.stringify({
          name,
          type: 'mobile',
          platform: 'react-native',
        }),
      });

      if (!response.ok) {
        throw new Error(`Device registration failed: ${response.statusText}`);
      }

      const device: Device = await response.json();
      await AsyncStorage.setItem('@current_device_id', device.id);
      return device;
    } catch (err) {
      console.error('Device registration failed:', err);
      throw err;
    }
  }

  /**
   * Remove a registered device
   */
  async removeDevice(deviceId: string): Promise<void> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const response = await fetch(`${this.baseUrl}/devices/${deviceId}`, {
        method: 'DELETE',
        headers: {
          Authorization: `Bearer ${this.accessToken}`,
        },
      });

      if (!response.ok) {
        throw new Error(`Device removal failed: ${response.statusText}`);
      }
    } catch (err) {
      console.error('Device removal failed:', err);
      throw err;
    }
  }

  /**
   * Get list of registered devices
   */
  async getDevices(): Promise<Device[]> {
    if (!this.accessToken) {
      throw new Error('Access token not set');
    }

    try {
      const response = await fetch(`${this.baseUrl}/devices`, {
        method: 'GET',
        headers: {
          Authorization: `Bearer ${this.accessToken}`,
        },
      });

      if (!response.ok) {
        throw new Error(`Get devices failed: ${response.statusText}`);
      }

      const devices: Device[] = await response.json();
      return devices;
    } catch (err) {
      console.error('Get devices failed:', err);
      return [];
    }
  }

  /**
   * Get last sync timestamp
   */
  async getLastSyncTime(): Promise<string | null> {
    try {
      return await AsyncStorage.getItem('@last_sync_time');
    } catch (err) {
      console.error('Failed to get last sync time:', err);
      return null;
    }
  }

  /**
   * Set last sync timestamp
   */
  async setLastSyncTime(timestamp: string): Promise<void> {
    try {
      await AsyncStorage.setItem('@last_sync_time', timestamp);
    } catch (err) {
      console.error('Failed to set last sync time:', err);
      throw err;
    }
  }

  /**
   * Get sync status
   */
  async getSyncStatus(): Promise<{
    lastSync: string | null;
    pendingChanges: number;
    conflicts: number;
  }> {
    try {
      const lastSync = await this.getLastSyncTime();
      const changes = await this.getPendingChanges();
      const stats = await storageService.getCacheStats();

      return {
        lastSync,
        pendingChanges: changes.length,
        conflicts: 0, // Would be fetched from server
      };
    } catch (err) {
      console.error('Failed to get sync status:', err);
      return {
        lastSync: null,
        pendingChanges: 0,
        conflicts: 0,
      };
    }
  }

  /**
   * Clear sync data (for logout)
   */
  async clearSyncData(): Promise<void> {
    try {
      await AsyncStorage.removeItem('@last_sync_time');
      await AsyncStorage.removeItem('@current_device_id');
      this.accessToken = null;
    } catch (err) {
      console.error('Failed to clear sync data:', err);
      throw err;
    }
  }
}

export const syncService = new SyncService();
