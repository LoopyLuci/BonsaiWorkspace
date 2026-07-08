//! Repository pattern for database access

use crate::database::*;
use chrono::Utc;
use std::collections::HashMap;

/// Result type for repository operations
pub type RepoResult<T> = Result<T, RepositoryError>;

/// Repository errors
#[derive(Debug, Clone)]
pub enum RepositoryError {
    NotFound(String),
    ConflictExists(String),
    InvalidInput(String),
    DatabaseError(String),
    SerializationError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::ConflictExists(msg) => write!(f, "Conflict: {}", msg),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

/// In-memory repository implementation (for testing)
/// In production, replace with actual database queries using sqlx
pub struct AppRepository {
    apps: HashMap<String, AppRecord>,
}

impl AppRepository {
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
        }
    }

    pub fn create(&mut self, app: AppRecord) -> RepoResult<AppRecord> {
        if self.apps.contains_key(&app.id) {
            return Err(RepositoryError::ConflictExists(format!(
                "App with ID {} already exists",
                app.id
            )));
        }

        self.apps.insert(app.id.clone(), app.clone());
        Ok(app)
    }

    pub fn get(&self, id: &str) -> RepoResult<AppRecord> {
        self.apps
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(format!("App with ID {} not found", id)))
    }

    pub fn get_all(&self) -> Vec<AppRecord> {
        self.apps.values().cloned().collect()
    }

    pub fn update(&mut self, id: &str, app: AppRecord) -> RepoResult<AppRecord> {
        if !self.apps.contains_key(id) {
            return Err(RepositoryError::NotFound(format!("App with ID {} not found", id)));
        }

        self.apps.insert(id.to_string(), app.clone());
        Ok(app)
    }

    pub fn delete(&mut self, id: &str) -> RepoResult<()> {
        self.apps
            .remove(id)
            .ok_or_else(|| RepositoryError::NotFound(format!("App with ID {} not found", id)))?;
        Ok(())
    }

    pub fn find_by_name(&self, name: &str) -> Vec<AppRecord> {
        self.apps
            .values()
            .filter(|app| app.name.to_lowercase().contains(&name.to_lowercase()))
            .cloned()
            .collect()
    }

    pub fn find_by_publisher(&self, publisher_id: &str) -> Vec<AppRecord> {
        self.apps
            .values()
            .filter(|app| app.publisher_id == publisher_id)
            .cloned()
            .collect()
    }
}

impl Default for AppRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Module repository
pub struct ModuleRepository {
    modules: HashMap<String, ModuleRecord>,
}

impl ModuleRepository {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn create(&mut self, module: ModuleRecord) -> RepoResult<ModuleRecord> {
        if self.modules.contains_key(&module.id) {
            return Err(RepositoryError::ConflictExists(format!(
                "Module with ID {} already exists",
                module.id
            )));
        }

        self.modules.insert(module.id.clone(), module.clone());
        Ok(module)
    }

    pub fn get(&self, id: &str) -> RepoResult<ModuleRecord> {
        self.modules
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(format!("Module with ID {} not found", id)))
    }

    pub fn get_all(&self) -> Vec<ModuleRecord> {
        self.modules.values().cloned().collect()
    }

    pub fn find_by_app(&self, app_id: &str) -> Vec<ModuleRecord> {
        self.modules
            .values()
            .filter(|m| m.app_id == app_id)
            .cloned()
            .collect()
    }
}

impl Default for ModuleRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Review repository
pub struct ReviewRepository {
    reviews: HashMap<String, ReviewRecord>,
}

impl ReviewRepository {
    pub fn new() -> Self {
        Self {
            reviews: HashMap::new(),
        }
    }

    pub fn create(&mut self, review: ReviewRecord) -> RepoResult<ReviewRecord> {
        if self.reviews.contains_key(&review.id) {
            return Err(RepositoryError::ConflictExists(format!(
                "Review with ID {} already exists",
                review.id
            )));
        }

        self.reviews.insert(review.id.clone(), review.clone());
        Ok(review)
    }

    pub fn get(&self, id: &str) -> RepoResult<ReviewRecord> {
        self.reviews
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(format!("Review with ID {} not found", id)))
    }

    pub fn find_by_app(&self, app_id: &str) -> Vec<ReviewRecord> {
        let mut reviews: Vec<_> = self
            .reviews
            .values()
            .filter(|r| r.app_id == app_id)
            .cloned()
            .collect();

        reviews.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        reviews
    }

    pub fn find_by_user(&self, user_id: &str) -> Vec<ReviewRecord> {
        self.reviews
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect()
    }

    pub fn count_by_app(&self, app_id: &str) -> usize {
        self.reviews.values().filter(|r| r.app_id == app_id).count()
    }

    pub fn average_rating(&self, app_id: &str) -> f32 {
        let reviews: Vec<_> = self
            .reviews
            .values()
            .filter(|r| r.app_id == app_id)
            .collect();

        if reviews.is_empty() {
            return 0.0;
        }

        let sum: i32 = reviews.iter().map(|r| r.rating as i32).sum();
        sum as f32 / reviews.len() as f32
    }
}

impl Default for ReviewRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Installation repository
pub struct InstallationRepository {
    installations: HashMap<String, InstallationRecord>,
}

impl InstallationRepository {
    pub fn new() -> Self {
        Self {
            installations: HashMap::new(),
        }
    }

    pub fn create(&mut self, install: InstallationRecord) -> RepoResult<InstallationRecord> {
        if self.installations.contains_key(&install.id) {
            return Err(RepositoryError::ConflictExists(format!(
                "Installation with ID {} already exists",
                install.id
            )));
        }

        self.installations
            .insert(install.id.clone(), install.clone());
        Ok(install)
    }

    pub fn get(&self, id: &str) -> RepoResult<InstallationRecord> {
        self.installations
            .get(id)
            .cloned()
            .ok_or_else(|| {
                RepositoryError::NotFound(format!("Installation with ID {} not found", id))
            })
    }

    pub fn find_by_app(&self, app_id: &str) -> Vec<InstallationRecord> {
        self.installations
            .values()
            .filter(|i| i.app_id == app_id)
            .cloned()
            .collect()
    }

    pub fn update_status(&mut self, id: &str, status: String) -> RepoResult<()> {
        if let Some(install) = self.installations.get_mut(id) {
            install.status = status;
            Ok(())
        } else {
            Err(RepositoryError::NotFound(format!(
                "Installation with ID {} not found",
                id
            )))
        }
    }
}

impl Default for InstallationRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency repository
pub struct DependencyRepository {
    dependencies: HashMap<String, DependencyRecord>,
}

impl DependencyRepository {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn create(&mut self, dep: DependencyRecord) -> RepoResult<DependencyRecord> {
        if self.dependencies.contains_key(&dep.id) {
            return Err(RepositoryError::ConflictExists(format!(
                "Dependency with ID {} already exists",
                dep.id
            )));
        }

        self.dependencies.insert(dep.id.clone(), dep.clone());
        Ok(dep)
    }

    pub fn find_by_module(&self, module_id: &str) -> Vec<DependencyRecord> {
        self.dependencies
            .values()
            .filter(|d| d.module_id == module_id)
            .cloned()
            .collect()
    }

    pub fn find_dependents(&self, module_id: &str) -> Vec<DependencyRecord> {
        self.dependencies
            .values()
            .filter(|d| d.depends_on == module_id)
            .cloned()
            .collect()
    }
}

impl Default for DependencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Settings repository
pub struct SettingsRepository {
    settings: HashMap<String, SettingsRecord>,
}

impl SettingsRepository {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn get_or_default(&mut self, user_id: &str) -> SettingsRecord {
        self.settings
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| SettingsRecord {
                user_id: user_id.to_string(),
                theme: "dark".to_string(),
                notifications_enabled: true,
                auto_update: true,
                language: "en".to_string(),
                updated_at: Utc::now(),
            })
    }

    pub fn update(&mut self, user_id: &str, settings: SettingsRecord) -> RepoResult<SettingsRecord> {
        let mut settings = settings;
        settings.user_id = user_id.to_string();
        settings.updated_at = Utc::now();
        self.settings
            .insert(user_id.to_string(), settings.clone());
        Ok(settings)
    }
}

impl Default for SettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration repository
pub struct ConfigRepository {
    configs: HashMap<String, ConfigRecord>,
}

impl ConfigRepository {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn set(&mut self, app_id: &str, key: &str, value: String) -> RepoResult<()> {
        let composite_key = format!("{}:{}", app_id, key);
        let record = ConfigRecord {
            app_id: app_id.to_string(),
            key: key.to_string(),
            value,
            updated_at: Utc::now(),
        };

        self.configs.insert(composite_key, record);
        Ok(())
    }

    pub fn get(&self, app_id: &str, key: &str) -> RepoResult<String> {
        let composite_key = format!("{}:{}", app_id, key);
        self.configs
            .get(&composite_key)
            .map(|r| r.value.clone())
            .ok_or_else(|| {
                RepositoryError::NotFound(format!(
                    "Config key {} for app {} not found",
                    key, app_id
                ))
            })
    }

    pub fn get_all_for_app(&self, app_id: &str) -> HashMap<String, String> {
        self.configs
            .values()
            .filter(|c| c.app_id == app_id)
            .map(|c| (c.key.clone(), c.value.clone()))
            .collect()
    }

    pub fn delete(&mut self, app_id: &str, key: &str) -> RepoResult<()> {
        let composite_key = format!("{}:{}", app_id, key);
        self.configs
            .remove(&composite_key)
            .ok_or_else(|| {
                RepositoryError::NotFound(format!(
                    "Config key {} for app {} not found",
                    key, app_id
                ))
            })?;
        Ok(())
    }
}

impl Default for ConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_repository_create_and_get() {
        let mut repo = AppRepository::new();
        let app = AppRecord {
            id: "app-1".to_string(),
            publisher_id: "pub-1".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            description: "A test app".to_string(),
            icon_url: "http://example.com/icon.png".to_string(),
            rating: 4.5,
            review_count: 10,
            download_count: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let created = repo.create(app.clone()).unwrap();
        assert_eq!(created.id, "app-1");

        let retrieved = repo.get("app-1").unwrap();
        assert_eq!(retrieved.name, "Test App");
    }

    #[test]
    fn test_app_repository_conflict() {
        let mut repo = AppRepository::new();
        let app = AppRecord {
            id: "app-1".to_string(),
            publisher_id: "pub-1".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            icon_url: "icon".to_string(),
            rating: 0.0,
            review_count: 0,
            download_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        repo.create(app.clone()).unwrap();
        let result = repo.create(app);
        assert!(matches!(result, Err(RepositoryError::ConflictExists(_))));
    }

    #[test]
    fn test_review_repository_average_rating() {
        let mut repo = ReviewRepository::new();

        for i in 1..=5 {
            let review = ReviewRecord {
                id: format!("review-{}", i),
                app_id: "app-1".to_string(),
                user_id: format!("user-{}", i),
                rating: i as i32,
                title: format!("Review {}", i),
                content: "Content".to_string(),
                helpful_count: 0,
                created_at: Utc::now(),
            };
            repo.create(review).unwrap();
        }

        let avg = repo.average_rating("app-1");
        assert_eq!(avg, 3.0); // (1 + 2 + 3 + 4 + 5) / 5 = 3.0
    }

    #[test]
    fn test_config_repository() {
        let mut repo = ConfigRepository::new();

        repo.set("app-1", "timeout", "5000".to_string()).unwrap();
        repo.set("app-1", "debug", "false".to_string()).unwrap();

        let timeout = repo.get("app-1", "timeout").unwrap();
        assert_eq!(timeout, "5000");

        let all = repo.get_all_for_app("app-1");
        assert_eq!(all.len(), 2);
    }
}
