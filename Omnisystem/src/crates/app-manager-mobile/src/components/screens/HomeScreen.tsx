import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TouchableOpacity,
  RefreshControl,
  ActivityIndicator,
  FlatList,
} from 'react-native';
import { useAuth } from '../../hooks/useAuth';
import { useApps } from '../../hooks/useApps';
import { useSync } from '../../hooks/useSync';
import { AppMetadata } from '../../types';

interface HomeScreenProps {
  onNavigateToApp?: (appId: string) => void;
  onNavigateToBrowse?: () => void;
}

export const HomeScreen: React.FC<HomeScreenProps> = ({
  onNavigateToApp,
  onNavigateToBrowse,
}) => {
  const { user } = useAuth();
  const { apps, loading, refreshApps } = useApps();
  const { syncState, triggerSync } = useSync();
  const [refreshing, setRefreshing] = useState(false);

  // Get top trending apps
  const trendingApps = apps.slice(0, 5);
  const installedApps = apps.filter(a => a.appState.status === 'installed');

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await Promise.all([refreshApps(), triggerSync()]);
    } catch (err) {
      console.error('Refresh failed:', err);
    } finally {
      setRefreshing(false);
    }
  };

  const renderAppCard = (app: AppMetadata) => (
    <TouchableOpacity
      key={app.id}
      style={styles.appCard}
      onPress={() => onNavigateToApp?.(app.id)}
    >
      <View style={styles.appIcon}>
        <Text style={styles.appIconText}>
          {app.name.charAt(0).toUpperCase()}
        </Text>
      </View>
      <View style={styles.appInfo}>
        <Text style={styles.appName} numberOfLines={1}>
          {app.name}
        </Text>
        <Text style={styles.appRating}>★ {app.rating.toFixed(1)}</Text>
      </View>
    </TouchableOpacity>
  );

  return (
    <ScrollView
      style={styles.container}
      refreshControl={
        <RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />
      }
    >
      {/* Header */}
      <View style={styles.header}>
        <View>
          <Text style={styles.greeting}>Hello, {user?.name || user?.email}!</Text>
          <Text style={styles.subtitle}>Welcome back to App Manager</Text>
        </View>
      </View>

      {/* Status Cards */}
      <View style={styles.statusCards}>
        <View style={styles.statusCard}>
          <Text style={styles.statusValue}>{installedApps.length}</Text>
          <Text style={styles.statusLabel}>Apps Installed</Text>
        </View>
        <View style={styles.statusCard}>
          <Text style={styles.statusValue}>{apps.length}</Text>
          <Text style={styles.statusLabel}>Available</Text>
        </View>
      </View>

      {/* Sync Status */}
      <View style={styles.syncSection}>
        <View style={styles.syncHeader}>
          <Text style={styles.sectionTitle}>Sync Status</Text>
          <TouchableOpacity
            style={styles.syncButton}
            onPress={triggerSync}
            disabled={syncState.isSyncing}
          >
            {syncState.isSyncing ? (
              <ActivityIndicator size="small" color="#2563eb" />
            ) : (
              <Text style={styles.syncButtonText}>Sync Now</Text>
            )}
          </TouchableOpacity>
        </View>
        <View style={styles.syncInfo}>
          <Text style={styles.syncStatus}>
            Status: <Text style={styles.syncStatusValue}>{syncState.status}</Text>
          </Text>
          {syncState.lastSync && (
            <Text style={styles.syncTime}>
              Last sync: {new Date(syncState.lastSync).toLocaleDateString()}
            </Text>
          )}
          {syncState.pendingChanges > 0 && (
            <Text style={styles.syncWarning}>
              {syncState.pendingChanges} changes pending
            </Text>
          )}
        </View>
      </View>

      {/* Trending Apps */}
      <View style={styles.section}>
        <View style={styles.sectionHeader}>
          <Text style={styles.sectionTitle}>Trending Apps</Text>
          <TouchableOpacity onPress={onNavigateToBrowse}>
            <Text style={styles.seeAll}>See All</Text>
          </TouchableOpacity>
        </View>

        {loading ? (
          <ActivityIndicator
            size="large"
            color="#2563eb"
            style={styles.loader}
          />
        ) : trendingApps.length > 0 ? (
          <View>
            {trendingApps.map(app => renderAppCard(app))}
          </View>
        ) : (
          <Text style={styles.emptyText}>No apps available</Text>
        )}
      </View>

      {/* Quick Actions */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Quick Actions</Text>
        <View style={styles.actionsGrid}>
          <TouchableOpacity
            style={styles.actionButton}
            onPress={onNavigateToBrowse}
          >
            <Text style={styles.actionIcon}>🔍</Text>
            <Text style={styles.actionLabel}>Browse Apps</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>❤️</Text>
            <Text style={styles.actionLabel}>Favorites</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>⚙️</Text>
            <Text style={styles.actionLabel}>Settings</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.actionButton}>
            <Text style={styles.actionIcon}>📱</Text>
            <Text style={styles.actionLabel}>Devices</Text>
          </TouchableOpacity>
        </View>
      </View>

      {/* Footer */}
      <View style={styles.footer}>
        <Text style={styles.footerText}>
          Keep your apps updated for the best experience
        </Text>
      </View>
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#1a1a1a',
  },
  header: {
    paddingHorizontal: 20,
    paddingVertical: 20,
    backgroundColor: '#242424',
    borderBottomWidth: 1,
    borderBottomColor: '#333',
  },
  greeting: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#fff',
    marginBottom: 4,
  },
  subtitle: {
    fontSize: 14,
    color: '#999',
  },
  statusCards: {
    flexDirection: 'row',
    paddingHorizontal: 20,
    paddingVertical: 20,
    gap: 12,
  },
  statusCard: {
    flex: 1,
    backgroundColor: '#2a2a2a',
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: '#404040',
  },
  statusValue: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#2563eb',
    marginBottom: 4,
  },
  statusLabel: {
    fontSize: 12,
    color: '#999',
  },
  syncSection: {
    marginHorizontal: 20,
    marginVertical: 16,
    backgroundColor: '#2a2a2a',
    borderRadius: 12,
    padding: 16,
    borderWidth: 1,
    borderColor: '#404040',
  },
  syncHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  syncButton: {
    backgroundColor: '#2563eb',
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 6,
  },
  syncButtonText: {
    color: '#fff',
    fontSize: 12,
    fontWeight: '600',
  },
  syncInfo: {
    gap: 8,
  },
  syncStatus: {
    fontSize: 14,
    color: '#ccc',
  },
  syncStatusValue: {
    color: '#4ade80',
    fontWeight: '600',
  },
  syncTime: {
    fontSize: 12,
    color: '#999',
  },
  syncWarning: {
    fontSize: 12,
    color: '#fbbf24',
    fontWeight: '500',
  },
  section: {
    paddingHorizontal: 20,
    paddingVertical: 16,
  },
  sectionHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#fff',
  },
  seeAll: {
    fontSize: 14,
    color: '#2563eb',
    fontWeight: '500',
  },
  appCard: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: '#2a2a2a',
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
    borderWidth: 1,
    borderColor: '#404040',
  },
  appIcon: {
    width: 48,
    height: 48,
    backgroundColor: '#2563eb',
    borderRadius: 8,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: 12,
  },
  appIconText: {
    fontSize: 20,
    fontWeight: 'bold',
    color: '#fff',
  },
  appInfo: {
    flex: 1,
  },
  appName: {
    fontSize: 14,
    fontWeight: '600',
    color: '#fff',
    marginBottom: 4,
  },
  appRating: {
    fontSize: 12,
    color: '#fbbf24',
  },
  loader: {
    marginVertical: 20,
  },
  emptyText: {
    color: '#666',
    fontSize: 14,
    textAlign: 'center',
    paddingVertical: 20,
  },
  actionsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  actionButton: {
    flex: 1,
    minWidth: '45%',
    backgroundColor: '#2a2a2a',
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: '#404040',
  },
  actionIcon: {
    fontSize: 28,
    marginBottom: 8,
  },
  actionLabel: {
    fontSize: 12,
    color: '#ccc',
    fontWeight: '500',
  },
  footer: {
    paddingHorizontal: 20,
    paddingVertical: 30,
    alignItems: 'center',
  },
  footerText: {
    fontSize: 12,
    color: '#666',
    textAlign: 'center',
  },
});
