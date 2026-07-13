import AsyncStorage from '@react-native-async-storage/async-storage';
import SQLite from 'react-native-sqlite-storage';
import { AppMetadata, LocalFavorite, LocalSettings, ChangeLog, QueuedAction } from '../types';

/**
 * SQLite Database Manager for persistent app cache
 */
class StorageService {
  private db: SQLite.SQLiteDatabase | null = null;
  private initialized = false;

  /**
   * Initialize database connection and create tables
   */
  async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      this.db = await SQLite.openDatabase({
        name: 'app_manager.db',
        location: 'default',
      });

      await this.createTables();
      this.initialized = true;
    } catch (err) {
      console.error('Failed to initialize database:', err);
      throw err;
    }
  }

  /**
   * Create database tables
   */
  private async createTables(): Promise<void> {
    if (!this.db) throw new Error('Database not initialized');

    const tables = [
      `CREATE TABLE IF NOT EXISTS apps (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        version TEXT,
        rating REAL,
        downloads INTEGER,
        category TEXT,
        icon TEXT,
        size INTEGER,
        developer TEXT,
        updated_at TEXT,
        permissions TEXT,
        app_state TEXT,
        cached_at TEXT
      )`,

      `CREATE TABLE IF NOT EXISTS favorites (
        app_id TEXT PRIMARY KEY,
        added_at TEXT NOT NULL,
        synced INTEGER DEFAULT 0,
        synced_at TEXT
      )`,

      `CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        synced INTEGER DEFAULT 0,
        updated_at TEXT
      )`,

      `CREATE TABLE IF NOT EXISTS change_log (
        id TEXT PRIMARY KEY,
        type TEXT NOT NULL,
        resource_type TEXT NOT NULL,
        resource_id TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        data TEXT,
        synced INTEGER DEFAULT 0
      )`,

      `CREATE TABLE IF NOT EXISTS installations (
        id TEXT PRIMARY KEY,
        app_id TEXT NOT NULL,
        version TEXT,
        install_date TEXT,
        last_used TEXT,
        size INTEGER,
        update_available INTEGER,
        latest_version TEXT,
        synced INTEGER DEFAULT 0
      )`,

      `CREATE INDEX IF NOT EXISTS idx_apps_category ON apps(category)`,
      `CREATE INDEX IF NOT EXISTS idx_favorites_app_id ON favorites(app_id)`,
      `CREATE INDEX IF NOT EXISTS idx_change_log_synced ON change_log(synced)`,
      `CREATE INDEX IF NOT EXISTS idx_change_log_timestamp ON change_log(timestamp)`,
    ];

    for (const sql of tables) {
      try {
        await this.db.executeSql(sql);
      } catch (err) {
        console.error('Failed to create table:', err);
      }
    }
  }

  /**
   * Cache apps in SQLite
   */
  async cacheApps(apps: AppMetadata[]): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    const timestamp = new Date().toISOString();

    for (const app of apps) {
      await this.db.executeSql(
        `INSERT OR REPLACE INTO apps
        (id, name, description, version, rating, downloads, category,
         icon, size, developer, updated_at, permissions, app_state, cached_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
        [
          app.id,
          app.name,
          app.description,
          app.version,
          app.rating,
          app.downloads,
          app.category,
          app.icon,
          app.size,
          app.developer,
          app.updatedAt,
          JSON.stringify(app.permissions),
          JSON.stringify(app.appState),
          timestamp,
        ]
      );
    }
  }

  /**
   * Get cached apps from SQLite
   */
  async getCachedApps(): Promise<AppMetadata[]> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    try {
      const result = await this.db.executeSql(
        'SELECT * FROM apps WHERE cached_at > datetime("now", "-24 hours")'
      );

      const apps: AppMetadata[] = [];
      for (let i = 0; i < result.rows.length; i++) {
        const row = result.rows.item(i);
        apps.push({
          id: row.id,
          name: row.name,
          description: row.description,
          version: row.version,
          rating: row.rating,
          downloads: row.downloads,
          category: row.category,
          icon: row.icon,
          size: row.size,
          developer: row.developer,
          updatedAt: row.updated_at,
          permissions: JSON.parse(row.permissions || '[]'),
          appState: JSON.parse(row.app_state || '{}'),
        });
      }
      return apps;
    } catch (err) {
      console.error('Failed to get cached apps:', err);
      return [];
    }
  }

  /**
   * Add favorite
   */
  async addFavorite(favorite: LocalFavorite): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    await this.db.executeSql(
      'INSERT OR REPLACE INTO favorites (app_id, added_at, synced) VALUES (?, ?, ?)',
      [favorite.appId, favorite.addedAt, 0]
    );

    // Queue for sync
    await this.queueChange({
      type: 'create',
      resourceType: 'favorite',
      resourceId: favorite.appId,
      data: favorite,
    });
  }

  /**
   * Remove favorite
   */
  async removeFavorite(appId: string): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    await this.db.executeSql('DELETE FROM favorites WHERE app_id = ?', [appId]);

    // Queue for sync
    await this.queueChange({
      type: 'delete',
      resourceType: 'favorite',
      resourceId: appId,
      data: { appId },
    });
  }

  /**
   * Get all favorites
   */
  async getFavorites(): Promise<LocalFavorite[]> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    try {
      const result = await this.db.executeSql('SELECT * FROM favorites');
      const favorites: LocalFavorite[] = [];

      for (let i = 0; i < result.rows.length; i++) {
        const row = result.rows.item(i);
        favorites.push({
          appId: row.app_id,
          addedAt: row.added_at,
          synced: row.synced === 1,
          syncedAt: row.synced_at,
        });
      }
      return favorites;
    } catch (err) {
      console.error('Failed to get favorites:', err);
      return [];
    }
  }

  /**
   * Save settings to AsyncStorage
   */
  async saveSettings(settings: LocalSettings): Promise<void> {
    try {
      await AsyncStorage.setItem('@app_manager_settings', JSON.stringify(settings));
    } catch (err) {
      console.error('Failed to save settings:', err);
      throw err;
    }
  }

  /**
   * Get settings from AsyncStorage
   */
  async getSettings(): Promise<LocalSettings | null> {
    try {
      const data = await AsyncStorage.getItem('@app_manager_settings');
      return data ? JSON.parse(data) : null;
    } catch (err) {
      console.error('Failed to get settings:', err);
      return null;
    }
  }

  /**
   * Queue a change for sync
   */
  async queueChange(change: Omit<ChangeLog, 'id'>): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    const id = `${change.resourceType}-${change.resourceId}-${Date.now()}`;

    await this.db.executeSql(
      `INSERT INTO change_log (id, type, resource_type, resource_id, timestamp, data, synced)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [
        id,
        change.type,
        change.resourceType,
        change.resourceId,
        change.timestamp || new Date().toISOString(),
        JSON.stringify(change.data),
        0,
      ]
    );
  }

  /**
   * Get pending changes (not yet synced)
   */
  async getPendingChanges(): Promise<ChangeLog[]> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    try {
      const result = await this.db.executeSql(
        'SELECT * FROM change_log WHERE synced = 0 ORDER BY timestamp ASC'
      );

      const changes: ChangeLog[] = [];
      for (let i = 0; i < result.rows.length; i++) {
        const row = result.rows.item(i);
        changes.push({
          id: row.id,
          type: row.type,
          resourceType: row.resource_type,
          resourceId: row.resource_id,
          timestamp: row.timestamp,
          data: JSON.parse(row.data || '{}'),
          synced: row.synced === 1,
        });
      }
      return changes;
    } catch (err) {
      console.error('Failed to get pending changes:', err);
      return [];
    }
  }

  /**
   * Mark changes as synced
   */
  async markChangesSynced(changeIds: string[]): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    for (const id of changeIds) {
      await this.db.executeSql(
        'UPDATE change_log SET synced = 1 WHERE id = ?',
        [id]
      );
    }
  }

  /**
   * Clear cache
   */
  async clearCache(): Promise<void> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    try {
      await this.db.executeSql('DELETE FROM apps');
      await this.db.executeSql('DELETE FROM favorites');
      await this.db.executeSql('DELETE FROM change_log WHERE synced = 1');
    } catch (err) {
      console.error('Failed to clear cache:', err);
      throw err;
    }
  }

  /**
   * Get cache statistics
   */
  async getCacheStats(): Promise<{ appCount: number; favoriteCount: number; pendingChanges: number }> {
    if (!this.db) await this.initialize();
    if (!this.db) throw new Error('Database not initialized');

    try {
      const appResult = await this.db.executeSql('SELECT COUNT(*) as count FROM apps');
      const favResult = await this.db.executeSql('SELECT COUNT(*) as count FROM favorites');
      const changeResult = await this.db.executeSql('SELECT COUNT(*) as count FROM change_log WHERE synced = 0');

      return {
        appCount: appResult.rows.item(0).count,
        favoriteCount: favResult.rows.item(0).count,
        pendingChanges: changeResult.rows.item(0).count,
      };
    } catch (err) {
      console.error('Failed to get cache stats:', err);
      return { appCount: 0, favoriteCount: 0, pendingChanges: 0 };
    }
  }

  /**
   * Close database connection
   */
  async close(): Promise<void> {
    if (this.db) {
      await this.db.close();
      this.db = null;
      this.initialized = false;
    }
  }
}

export const storageService = new StorageService();
