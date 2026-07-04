//! Application discovery service with filtering and search

use crate::app::{RegisteredApp, AppId};
use crate::registry::SearchIndex;
use dashmap::DashMap;
use std::sync::Arc;

/// Discovery service for finding apps by various criteria
pub struct AppDiscoveryService {
    search_index: Arc<SearchIndex>,
    apps_map: Arc<DashMap<AppId, RegisteredApp>>,
}

/// Filter criteria for app discovery
#[derive(Debug, Clone)]
pub struct DiscoveryFilter {
    pub name_contains: Option<String>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub min_rating: Option<f32>,
    pub platforms: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
}

impl DiscoveryFilter {
    pub fn new() -> Self {
        Self {
            name_contains: None,
            categories: None,
            tags: None,
            min_rating: None,
            platforms: None,
            languages: None,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name_contains = Some(name);
        self
    }

    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = Some(categories);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_min_rating(mut self, rating: f32) -> Self {
        self.min_rating = Some(rating);
        self
    }

    pub fn with_platforms(mut self, platforms: Vec<String>) -> Self {
        self.platforms = Some(platforms);
        self
    }

    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = Some(languages);
        self
    }
}

impl Default for DiscoveryFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl AppDiscoveryService {
    pub fn new(
        search_index: Arc<SearchIndex>,
        apps_map: Arc<DashMap<AppId, RegisteredApp>>,
    ) -> Self {
        Self {
            search_index,
            apps_map,
        }
    }

    /// Discover all applications
    pub fn discover_all(&self) -> Vec<RegisteredApp> {
        self.apps_map
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Discover apps matching filter criteria
    pub fn discover(&self, filter: &DiscoveryFilter) -> Vec<RegisteredApp> {
        let mut results = self.discover_all();

        // Filter by name
        if let Some(name_filter) = &filter.name_contains {
            let lower = name_filter.to_lowercase();
            results.retain(|app| app.manifest.name.to_lowercase().contains(&lower));
        }

        // Filter by categories
        if let Some(categories) = &filter.categories {
            results.retain(|app| {
                categories
                    .iter()
                    .any(|cat| app.manifest.categories.contains(cat))
            });
        }

        // Filter by tags
        if let Some(tags) = &filter.tags {
            results.retain(|app| {
                tags.iter()
                    .any(|tag| app.manifest.tags.contains(tag))
            });
        }

        // Filter by minimum rating
        if let Some(min_rating) = filter.min_rating {
            results.retain(|app| app.rating >= min_rating);
        }

        // Filter by platforms
        if let Some(platforms) = &filter.platforms {
            results.retain(|app| {
                platforms
                    .iter()
                    .any(|plat| app.manifest.platforms.contains(plat))
            });
        }

        // Filter by languages
        if let Some(languages) = &filter.languages {
            results.retain(|app| {
                languages
                    .iter()
                    .any(|lang| app.manifest.languages.contains(lang))
            });
        }

        results
    }

    /// Search by category (<50ms)
    pub fn search_by_category(&self, category: &str) -> Vec<RegisteredApp> {
        self.search_index.search_by_category(category)
    }

    /// Search by tag (<50ms)
    pub fn search_by_tag(&self, tag: &str) -> Vec<RegisteredApp> {
        self.search_index.search_by_tag(tag)
    }

    /// Search by multiple tags (OR query)
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<RegisteredApp> {
        self.search_index.search_by_tags(tags)
    }

    /// Get top rated apps
    pub fn get_top_rated(&self, limit: usize) -> Vec<RegisteredApp> {
        let mut apps = self.discover_all();
        apps.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));
        apps.into_iter().take(limit).collect()
    }

    /// Get most downloaded apps
    pub fn get_most_downloaded(&self, limit: usize) -> Vec<RegisteredApp> {
        let mut apps = self.discover_all();
        apps.sort_by(|a, b| b.download_count.cmp(&a.download_count));
        apps.into_iter().take(limit).collect()
    }

    /// Get recently updated apps
    pub fn get_recently_updated(&self, limit: usize) -> Vec<RegisteredApp> {
        let mut apps = self.discover_all();
        apps.sort_by(|a, b| b.manifest.updated_at.cmp(&a.manifest.updated_at));
        apps.into_iter().take(limit).collect()
    }

    /// Discover apps by exact name
    pub fn discover_by_name(&self, name: &str) -> Option<RegisteredApp> {
        self.apps_map
            .iter()
            .find(|entry| entry.value().manifest.name == name)
            .map(|entry| entry.value().clone())
    }

    /// Discover apps by ID
    pub fn discover_by_id(&self, app_id: &AppId) -> Option<RegisteredApp> {
        self.apps_map.get(app_id).map(|entry| entry.clone())
    }

    /// Get total app count
    pub fn count(&self) -> usize {
        self.apps_map.len()
    }

    /// Check if app exists
    pub fn exists(&self, app_id: &AppId) -> bool {
        self.apps_map.contains_key(app_id)
    }

    /// Discover apps by publisher
    pub fn discover_by_publisher(&self, publisher_id: &crate::app::PublisherId) -> Vec<RegisteredApp> {
        self.apps_map
            .iter()
            .filter(|entry| entry.value().manifest.publisher_id == *publisher_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get apps in category sorted by rating
    pub fn category_apps_by_rating(&self, category: &str) -> Vec<RegisteredApp> {
        let mut apps = self.search_by_category(category);
        apps.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));
        apps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppManifest, PublisherId};

    fn create_test_discovery() -> (AppDiscoveryService, Arc<DashMap<AppId, RegisteredApp>>) {
        let apps_map = Arc::new(DashMap::new());
        let search_index = Arc::new(SearchIndex::new(apps_map.clone()));
        let service = AppDiscoveryService::new(search_index, apps_map.clone());
        (service, apps_map)
    }

    fn create_app(name: &str, category: &str) -> RegisteredApp {
        let publisher = PublisherId::new();
        let mut manifest = AppManifest::new(
            name.to_string(),
            semver::Version::new(1, 0, 0),
            publisher,
        );
        manifest.icon_url = "icon.png".to_string();
        manifest.categories = vec![category.to_string()];
        manifest.tags = vec!["tag1".to_string()];
        RegisteredApp::new(manifest)
    }

    #[test]
    fn test_discover_all() {
        let (service, apps_map) = create_test_discovery();

        apps_map.insert(
            AppId::new(),
            create_app("App1", "Productivity"),
        );
        apps_map.insert(
            AppId::new(),
            create_app("App2", "Games"),
        );

        assert_eq!(service.discover_all().len(), 2);
    }

    #[test]
    fn test_discover_by_name_filter() {
        let (service, apps_map) = create_test_discovery();

        let app = create_app("TestApp", "Productivity");
        apps_map.insert(app.manifest.id.clone(), app);

        let filter = DiscoveryFilter::new()
            .with_name("test".to_string());

        let results = service.discover(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].manifest.name, "TestApp");
    }

    #[test]
    fn test_discover_by_category_filter() {
        let (service, apps_map) = create_test_discovery();

        for i in 0..5 {
            let category = if i < 3 { "Productivity" } else { "Games" };
            let app = create_app(&format!("App{}", i), category);
            apps_map.insert(app.manifest.id.clone(), app);
        }

        let filter = DiscoveryFilter::new()
            .with_categories(vec!["Productivity".to_string()]);

        let results = service.discover(&filter);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_get_top_rated() {
        let (service, apps_map) = create_test_discovery();

        for i in 0..10 {
            let mut app = create_app(&format!("App{}", i), "Productivity");
            app.rating = (i as f32) * 0.5;
            apps_map.insert(app.manifest.id.clone(), app);
        }

        let top = service.get_top_rated(3);
        assert_eq!(top.len(), 3);
        assert!(top[0].rating >= top[1].rating);
        assert!(top[1].rating >= top[2].rating);
    }

    #[test]
    fn test_discover_by_name_exact() {
        let (service, apps_map) = create_test_discovery();

        let app = create_app("UniqueApp", "Productivity");
        let app_id = app.manifest.id.clone();
        apps_map.insert(app_id.clone(), app);

        let found = service.discover_by_name("UniqueApp");
        assert!(found.is_some());
        assert_eq!(found.unwrap().manifest.id, app_id);
    }

    #[test]
    fn test_discover_by_id() {
        let (service, apps_map) = create_test_discovery();

        let app = create_app("TestApp", "Productivity");
        let app_id = app.manifest.id.clone();
        apps_map.insert(app_id.clone(), app);

        let found = service.discover_by_id(&app_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().manifest.id, app_id);
    }

    #[test]
    fn test_filter_combined() {
        let (service, apps_map) = create_test_discovery();

        for i in 0..10 {
            let mut app = create_app(&format!("App{}", i), "Productivity");
            app.rating = (i as f32) * 0.5;
            apps_map.insert(app.manifest.id.clone(), app);
        }

        let filter = DiscoveryFilter::new()
            .with_categories(vec!["Productivity".to_string()])
            .with_min_rating(2.0);

        let results = service.discover(&filter);
        assert!(!results.is_empty());
        assert!(results.iter().all(|app| app.rating >= 2.0));
    }

    #[test]
    fn test_count() {
        let (service, apps_map) = create_test_discovery();

        for i in 0..5 {
            let app = create_app(&format!("App{}", i), "Productivity");
            apps_map.insert(app.manifest.id.clone(), app);
        }

        assert_eq!(service.count(), 5);
    }

    #[test]
    fn test_exists() {
        let (service, apps_map) = create_test_discovery();

        let app = create_app("TestApp", "Productivity");
        let app_id = app.manifest.id.clone();
        apps_map.insert(app_id.clone(), app);

        assert!(service.exists(&app_id));
        assert!(!service.exists(&AppId::new()));
    }
}
