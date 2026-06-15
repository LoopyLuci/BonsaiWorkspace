import React, { useState, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TextInput,
  ScrollView,
  TouchableOpacity,
  FlatList,
  ActivityIndicator,
  RefreshControl,
} from 'react-native';
import { useApps } from '../../hooks/useApps';
import { AppMetadata, AppCategory } from '../../types';

interface BrowseScreenProps {
  onSelectApp?: (appId: string) => void;
}

const CATEGORIES: { label: string; value: AppCategory }[] = [
  { label: 'All', value: 'productivity' },
  { label: 'Productivity', value: 'productivity' },
  { label: 'Entertainment', value: 'entertainment' },
  { label: 'Utilities', value: 'utilities' },
  { label: 'Development', value: 'development' },
  { label: 'Social', value: 'social' },
  { label: 'Business', value: 'business' },
];

const SORT_OPTIONS = [
  { label: 'Name', value: 'name' },
  { label: 'Rating', value: 'rating' },
  { label: 'Downloads', value: 'downloads' },
  { label: 'Recent', value: 'recent' },
];

export const BrowseScreen: React.FC<BrowseScreenProps> = ({ onSelectApp }) => {
  const { apps, loading, error, searchApps, refreshApps } = useApps();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<AppCategory>('productivity');
  const [selectedSort, setSelectedSort] = useState('name');
  const [showFilters, setShowFilters] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [minRating, setMinRating] = useState(0);

  // Filter and search apps
  const filteredApps = apps.filter(app => {
    const matchesSearch =
      !searchQuery ||
      app.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      app.description.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesCategory = app.category === selectedCategory;
    const matchesRating = app.rating >= minRating;

    return matchesSearch && matchesCategory && matchesRating;
  });

  // Sort apps
  const sortedApps = [...filteredApps].sort((a, b) => {
    switch (selectedSort) {
      case 'rating':
        return b.rating - a.rating;
      case 'downloads':
        return b.downloads - a.downloads;
      case 'recent':
        return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
      case 'name':
      default:
        return a.name.localeCompare(b.name);
    }
  });

  const handleSearch = (text: string) => {
    setSearchQuery(text);
    if (text) {
      searchApps({ q: text, category: selectedCategory, minRating });
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await refreshApps();
    } catch (err) {
      console.error('Refresh failed:', err);
    } finally {
      setRefreshing(false);
    }
  };

  const renderAppItem = (app: AppMetadata) => (
    <TouchableOpacity
      key={app.id}
      style={styles.appItem}
      onPress={() => onSelectApp?.(app.id)}
    >
      <View style={styles.appIcon}>
        <Text style={styles.appIconText}>
          {app.name.charAt(0).toUpperCase()}
        </Text>
      </View>

      <View style={styles.appDetails}>
        <Text style={styles.appName} numberOfLines={1}>
          {app.name}
        </Text>
        <Text style={styles.appCategory}>{app.category}</Text>
        <Text style={styles.appDescription} numberOfLines={1}>
          {app.description}
        </Text>
      </View>

      <View style={styles.appMeta}>
        <Text style={styles.appRating}>★ {app.rating.toFixed(1)}</Text>
        <Text style={styles.appDownloads}>
          {(app.downloads / 1000).toFixed(0)}K
        </Text>
      </View>
    </TouchableOpacity>
  );

  return (
    <View style={styles.container}>
      {/* Search Bar */}
      <View style={styles.searchBar}>
        <TextInput
          style={styles.searchInput}
          placeholder="Search apps..."
          placeholderTextColor="#666"
          value={searchQuery}
          onChangeText={handleSearch}
        />
        <TouchableOpacity onPress={() => setShowFilters(!showFilters)}>
          <Text style={styles.filterIcon}>⚙️</Text>
        </TouchableOpacity>
      </View>

      {/* Filters Panel */}
      {showFilters && (
        <View style={styles.filtersPanel}>
          {/* Category Filter */}
          <View style={styles.filterSection}>
            <Text style={styles.filterTitle}>Category</Text>
            <ScrollView
              horizontal
              showsHorizontalScrollIndicator={false}
              style={styles.categoryScroll}
            >
              {CATEGORIES.map(cat => (
                <TouchableOpacity
                  key={cat.value}
                  style={[
                    styles.categoryChip,
                    selectedCategory === cat.value && styles.categoryChipActive,
                  ]}
                  onPress={() => setSelectedCategory(cat.value)}
                >
                  <Text
                    style={[
                      styles.categoryChipText,
                      selectedCategory === cat.value &&
                        styles.categoryChipTextActive,
                    ]}
                  >
                    {cat.label}
                  </Text>
                </TouchableOpacity>
              ))}
            </ScrollView>
          </View>

          {/* Sort Filter */}
          <View style={styles.filterSection}>
            <Text style={styles.filterTitle}>Sort By</Text>
            <View style={styles.sortOptions}>
              {SORT_OPTIONS.map(opt => (
                <TouchableOpacity
                  key={opt.value}
                  style={[
                    styles.sortOption,
                    selectedSort === opt.value && styles.sortOptionActive,
                  ]}
                  onPress={() => setSelectedSort(opt.value)}
                >
                  <Text
                    style={[
                      styles.sortOptionText,
                      selectedSort === opt.value && styles.sortOptionTextActive,
                    ]}
                  >
                    {opt.label}
                  </Text>
                </TouchableOpacity>
              ))}
            </View>
          </View>

          {/* Rating Filter */}
          <View style={styles.filterSection}>
            <Text style={styles.filterTitle}>
              Minimum Rating: {minRating.toFixed(1)} ★
            </Text>
            <View style={styles.ratingSlider}>
              {[0, 1, 2, 3, 4, 5].map(rating => (
                <TouchableOpacity
                  key={rating}
                  style={[
                    styles.ratingButton,
                    minRating === rating && styles.ratingButtonActive,
                  ]}
                  onPress={() => setMinRating(rating)}
                >
                  <Text style={styles.ratingButtonText}>{rating}</Text>
                </TouchableOpacity>
              ))}
            </View>
          </View>
        </View>
      )}

      {/* App List */}
      <FlatList
        data={sortedApps}
        renderItem={({ item }) => renderAppItem(item)}
        keyExtractor={item => item.id}
        style={styles.appList}
        ListEmptyComponent={
          <View style={styles.emptyContainer}>
            <Text style={styles.emptyText}>
              {loading ? 'Loading apps...' : 'No apps found'}
            </Text>
            {error && <Text style={styles.errorText}>{error}</Text>}
          </View>
        }
        ListHeaderComponent={
          loading && !refreshing ? (
            <ActivityIndicator size="large" color="#2563eb" style={styles.loader} />
          ) : null
        }
        refreshControl={
          <RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />
        }
      />

      {/* Results Count */}
      {!loading && (
        <View style={styles.resultCount}>
          <Text style={styles.resultCountText}>
            {sortedApps.length} app{sortedApps.length !== 1 ? 's' : ''} found
          </Text>
        </View>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#1a1a1a',
  },
  searchBar: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#242424',
    borderBottomWidth: 1,
    borderBottomColor: '#333',
  },
  searchInput: {
    flex: 1,
    backgroundColor: '#2a2a2a',
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 8,
    color: '#fff',
    marginRight: 12,
  },
  filterIcon: {
    fontSize: 20,
  },
  filtersPanel: {
    backgroundColor: '#242424',
    borderBottomWidth: 1,
    borderBottomColor: '#333',
    paddingVertical: 12,
  },
  filterSection: {
    paddingHorizontal: 16,
    paddingVertical: 8,
  },
  filterTitle: {
    color: '#ccc',
    fontSize: 12,
    fontWeight: '600',
    marginBottom: 8,
    textTransform: 'uppercase',
  },
  categoryScroll: {
    marginRight: -16,
  },
  categoryChip: {
    backgroundColor: '#2a2a2a',
    borderRadius: 16,
    paddingHorizontal: 12,
    paddingVertical: 6,
    marginRight: 8,
    borderWidth: 1,
    borderColor: '#404040',
  },
  categoryChipActive: {
    backgroundColor: '#2563eb',
    borderColor: '#2563eb',
  },
  categoryChipText: {
    color: '#999',
    fontSize: 12,
    fontWeight: '500',
  },
  categoryChipTextActive: {
    color: '#fff',
  },
  sortOptions: {
    flexDirection: 'row',
    gap: 8,
    flexWrap: 'wrap',
  },
  sortOption: {
    backgroundColor: '#2a2a2a',
    borderRadius: 6,
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderWidth: 1,
    borderColor: '#404040',
  },
  sortOptionActive: {
    backgroundColor: '#2563eb',
    borderColor: '#2563eb',
  },
  sortOptionText: {
    color: '#999',
    fontSize: 12,
  },
  sortOptionTextActive: {
    color: '#fff',
  },
  ratingSlider: {
    flexDirection: 'row',
    gap: 8,
  },
  ratingButton: {
    backgroundColor: '#2a2a2a',
    borderRadius: 6,
    paddingHorizontal: 10,
    paddingVertical: 6,
    borderWidth: 1,
    borderColor: '#404040',
  },
  ratingButtonActive: {
    backgroundColor: '#2563eb',
    borderColor: '#2563eb',
  },
  ratingButtonText: {
    color: '#fff',
    fontSize: 12,
    fontWeight: '600',
  },
  appList: {
    flex: 1,
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  appItem: {
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
    width: 56,
    height: 56,
    backgroundColor: '#2563eb',
    borderRadius: 8,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: 12,
  },
  appIconText: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#fff',
  },
  appDetails: {
    flex: 1,
  },
  appName: {
    fontSize: 14,
    fontWeight: '600',
    color: '#fff',
    marginBottom: 2,
  },
  appCategory: {
    fontSize: 11,
    color: '#666',
    marginBottom: 4,
    textTransform: 'capitalize',
  },
  appDescription: {
    fontSize: 12,
    color: '#999',
  },
  appMeta: {
    alignItems: 'flex-end',
  },
  appRating: {
    fontSize: 12,
    color: '#fbbf24',
    fontWeight: '600',
    marginBottom: 2,
  },
  appDownloads: {
    fontSize: 11,
    color: '#666',
  },
  loader: {
    marginVertical: 20,
  },
  emptyContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: 40,
  },
  emptyText: {
    color: '#666',
    fontSize: 16,
    marginBottom: 8,
  },
  errorText: {
    color: '#ff6b6b',
    fontSize: 12,
  },
  resultCount: {
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#242424',
    borderTopWidth: 1,
    borderTopColor: '#333',
  },
  resultCountText: {
    color: '#999',
    fontSize: 12,
  },
});
