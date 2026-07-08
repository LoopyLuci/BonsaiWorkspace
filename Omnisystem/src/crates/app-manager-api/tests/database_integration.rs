//! Integration tests for database layer

use app_manager_api::{
    database::*,
    repository::*,
};
use chrono::Utc;

#[test]
fn test_database_schema_sql_strings() {
    assert!(!CREATE_APPS_TABLE.is_empty());
    assert!(!CREATE_MODULES_TABLE.is_empty());
    assert!(!CREATE_REVIEWS_TABLE.is_empty());
    assert!(!CREATE_INSTALLATIONS_TABLE.is_empty());
    assert!(!CREATE_SETTINGS_TABLE.is_empty());
    assert!(!CREATE_CONFIG_TABLE.is_empty());
    assert!(!CREATE_DEPENDENCIES_TABLE.is_empty());

    assert!(CREATE_APPS_TABLE.contains("CREATE TABLE IF NOT EXISTS apps"));
    assert!(CREATE_REVIEWS_TABLE.contains("rating INTEGER CHECK (rating >= 1 AND rating <= 5)"));
}

#[test]
fn test_database_config_connection_string() {
    let config = DatabaseConfig {
        host: "db.example.com".to_string(),
        port: 5432,
        user: "admin".to_string(),
        password: "secret".to_string(),
        database: "mydb".to_string(),
        max_connections: 20,
    };

    let conn_str = config.connection_string();
    assert!(conn_str.contains("postgres://"));
    assert!(conn_str.contains("admin:secret"));
    assert!(conn_str.contains("db.example.com:5432"));
    assert!(conn_str.contains("mydb"));
}

#[test]
fn test_app_record_full_lifecycle() {
    let app = AppRecord {
        id: "app-123".to_string(),
        publisher_id: "pub-456".to_string(),
        name: "Productivity Pro".to_string(),
        version: "2.1.0".to_string(),
        description: "Boost your productivity".to_string(),
        icon_url: "https://example.com/icons/prod.png".to_string(),
        rating: 4.7,
        review_count: 523,
        download_count: 125000,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(app.name, "Productivity Pro");
    assert_eq!(app.rating, 4.7);
    assert!(app.download_count > 100000);
}

#[test]
fn test_app_repository_operations() {
    let mut repo = AppRepository::new();

    // Create
    let app1 = AppRecord {
        id: "app-1".to_string(),
        publisher_id: "pub-1".to_string(),
        name: "App One".to_string(),
        version: "1.0.0".to_string(),
        description: "First app".to_string(),
        icon_url: "icon1.png".to_string(),
        rating: 4.0,
        review_count: 10,
        download_count: 100,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created = repo.create(app1).expect("Failed to create app");
    assert_eq!(created.id, "app-1");

    // Get
    let retrieved = repo.get("app-1").expect("Failed to get app");
    assert_eq!(retrieved.name, "App One");

    // Get all
    let app2 = AppRecord {
        id: "app-2".to_string(),
        publisher_id: "pub-1".to_string(),
        name: "App Two".to_string(),
        version: "1.0.0".to_string(),
        description: "Second app".to_string(),
        icon_url: "icon2.png".to_string(),
        rating: 4.5,
        review_count: 20,
        download_count: 200,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    repo.create(app2).expect("Failed to create app 2");
    let all = repo.get_all();
    assert_eq!(all.len(), 2);

    // Find by name
    let found = repo.find_by_name("App");
    assert_eq!(found.len(), 2);

    // Find by publisher
    let from_pub = repo.find_by_publisher("pub-1");
    assert_eq!(from_pub.len(), 2);

    // Update
    let mut updated_app = retrieved;
    updated_app.rating = 4.8;
    repo.update("app-1", updated_app).expect("Failed to update");
    let updated = repo.get("app-1").expect("Failed to get updated app");
    assert_eq!(updated.rating, 4.8);

    // Delete
    repo.delete("app-2").expect("Failed to delete");
    let after_delete = repo.get_all();
    assert_eq!(after_delete.len(), 1);
}

#[test]
fn test_module_repository_operations() {
    let mut repo = ModuleRepository::new();

    let module = ModuleRecord {
        id: "mod-1".to_string(),
        app_id: "app-1".to_string(),
        name: "Core Module".to_string(),
        version: "1.0.0".to_string(),
        module_type: "library".to_string(),
        status: "loaded".to_string(),
        file_hash: "abc123".to_string(),
        file_size: 1024000,
        created_at: Utc::now(),
    };

    repo.create(module).expect("Failed to create module");

    // Get
    let retrieved = repo.get("mod-1").expect("Failed to get module");
    assert_eq!(retrieved.name, "Core Module");

    // Find by app
    let modules = repo.find_by_app("app-1");
    assert_eq!(modules.len(), 1);
}

#[test]
fn test_review_repository_rating_calculation() {
    let mut repo = ReviewRepository::new();

    let reviews = vec![
        ReviewRecord {
            id: "rev-1".to_string(),
            app_id: "app-1".to_string(),
            user_id: "user-1".to_string(),
            rating: 5,
            title: "Excellent!".to_string(),
            content: "Best app ever".to_string(),
            helpful_count: 42,
            created_at: Utc::now(),
        },
        ReviewRecord {
            id: "rev-2".to_string(),
            app_id: "app-1".to_string(),
            user_id: "user-2".to_string(),
            rating: 4,
            title: "Great".to_string(),
            content: "Very good".to_string(),
            helpful_count: 25,
            created_at: Utc::now(),
        },
        ReviewRecord {
            id: "rev-3".to_string(),
            app_id: "app-1".to_string(),
            user_id: "user-3".to_string(),
            rating: 3,
            title: "Good".to_string(),
            content: "Okay".to_string(),
            helpful_count: 10,
            created_at: Utc::now(),
        },
    ];

    for review in reviews {
        repo.create(review).expect("Failed to create review");
    }

    // Average rating
    let avg = repo.average_rating("app-1");
    assert_eq!(avg, 4.0); // (5 + 4 + 3) / 3 = 4.0

    // Count
    let count = repo.count_by_app("app-1");
    assert_eq!(count, 3);

    // Find by app
    let app_reviews = repo.find_by_app("app-1");
    assert_eq!(app_reviews.len(), 3);

    // Find by user
    let user_reviews = repo.find_by_user("user-1");
    assert_eq!(user_reviews.len(), 1);
    assert_eq!(user_reviews[0].rating, 5);
}

#[test]
fn test_installation_repository_operations() {
    let mut repo = InstallationRepository::new();

    let install = InstallationRecord {
        id: "inst-1".to_string(),
        app_id: "app-1".to_string(),
        version: "1.0.0".to_string(),
        location: "/opt/apps/app-1".to_string(),
        status: "pending".to_string(),
        installed_at: Utc::now(),
    };

    repo.create(install).expect("Failed to create installation");

    // Get
    let retrieved = repo.get("inst-1").expect("Failed to get installation");
    assert_eq!(retrieved.status, "pending");

    // Update status
    repo.update_status("inst-1", "completed".to_string())
        .expect("Failed to update status");
    let updated = repo.get("inst-1").expect("Failed to get updated installation");
    assert_eq!(updated.status, "completed");

    // Find by app
    let installs = repo.find_by_app("app-1");
    assert_eq!(installs.len(), 1);
}

#[test]
fn test_dependency_repository_operations() {
    let mut repo = DependencyRepository::new();

    let dep = DependencyRecord {
        id: "dep-1".to_string(),
        module_id: "mod-1".to_string(),
        depends_on: "mod-2".to_string(),
        version_constraint: "^1.0.0".to_string(),
        optional: false,
    };

    repo.create(dep).expect("Failed to create dependency");

    // Find by module
    let deps = repo.find_by_module("mod-1");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].depends_on, "mod-2");

    // Find dependents
    let dependents = repo.find_dependents("mod-2");
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].module_id, "mod-1");
}

#[test]
fn test_settings_repository_defaults() {
    let mut repo = SettingsRepository::new();

    // Get or create defaults
    let settings = repo.get_or_default("user-1");
    assert_eq!(settings.theme, "dark");
    assert_eq!(settings.notifications_enabled, true);
    assert_eq!(settings.auto_update, true);
    assert_eq!(settings.language, "en");

    // Update
    let mut updated = settings;
    updated.theme = "light".to_string();
    repo.update("user-1", updated).expect("Failed to update");

    let retrieved = repo.get_or_default("user-1");
    assert_eq!(retrieved.theme, "light");
}

#[test]
fn test_config_repository_key_value_operations() {
    let mut repo = ConfigRepository::new();

    // Set multiple configs
    repo.set("app-1", "timeout_ms", "5000".to_string())
        .expect("Failed to set config");
    repo.set("app-1", "debug_mode", "false".to_string())
        .expect("Failed to set config");
    repo.set("app-1", "max_memory_mb", "512".to_string())
        .expect("Failed to set config");

    // Get individual
    let timeout = repo.get("app-1", "timeout_ms").expect("Failed to get config");
    assert_eq!(timeout, "5000");

    // Get all
    let all = repo.get_all_for_app("app-1");
    assert_eq!(all.len(), 3);
    assert!(all.contains_key("debug_mode"));

    // Delete
    repo.delete("app-1", "debug_mode")
        .expect("Failed to delete config");
    let after_delete = repo.get_all_for_app("app-1");
    assert_eq!(after_delete.len(), 2);
}

#[test]
fn test_repository_error_display() {
    let error = RepositoryError::NotFound("test".to_string());
    assert!(error.to_string().contains("Not found"));

    let error = RepositoryError::ConflictExists("test".to_string());
    assert!(error.to_string().contains("Conflict"));

    let error = RepositoryError::InvalidInput("test".to_string());
    assert!(error.to_string().contains("Invalid input"));
}

#[test]
fn test_concurrent_app_operations() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let repo = Arc::new(Mutex::new(AppRepository::new()));

    let mut handles = vec![];

    // Create 10 apps in parallel
    for i in 0..10 {
        let repo_clone = Arc::clone(&repo);
        let handle = thread::spawn(move || {
            let app = AppRecord {
                id: format!("app-{}", i),
                publisher_id: "pub-1".to_string(),
                name: format!("App {}", i),
                version: "1.0.0".to_string(),
                description: "Test app".to_string(),
                icon_url: "icon.png".to_string(),
                rating: 4.0,
                review_count: 0,
                download_count: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let mut repo = repo_clone.lock().unwrap();
            repo.create(app)
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify all apps created
    let repo = repo.lock().unwrap();
    let all = repo.get_all();
    assert_eq!(all.len(), 10);
}

#[test]
fn test_review_distribution_calculation() {
    let mut repo = ReviewRepository::new();

    // Create reviews with various ratings
    let rating_counts = vec![(5, 10), (4, 8), (3, 3), (2, 2), (1, 1)];

    for (rating, count) in rating_counts {
        for i in 0..count {
            let review = ReviewRecord {
                id: format!("rev-{}-{}", rating, i),
                app_id: "app-1".to_string(),
                user_id: format!("user-{}-{}", rating, i),
                rating,
                title: format!("Review {}", i),
                content: "Content".to_string(),
                helpful_count: 0,
                created_at: Utc::now(),
            };
            repo.create(review).expect("Failed to create review");
        }
    }

    let avg = repo.average_rating("app-1");
    let expected = (5 * 10 + 4 * 8 + 3 * 3 + 2 * 2 + 1 * 1) as f32 / 24.0;
    assert!((avg - expected).abs() < 0.01);
}
