//! Database layer with PostgreSQL integration

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            user: "appmanager".to_string(),
            password: "secure_password".to_string(),
            database: "appmanager_db".to_string(),
            max_connections: 10,
        }
    }
}

/// App record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    pub id: String,
    pub publisher_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon_url: String,
    pub rating: f32,
    pub review_count: i32,
    pub download_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Module record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub module_type: String,
    pub status: String,
    pub file_hash: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Review record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub id: String,
    pub app_id: String,
    pub user_id: String,
    pub rating: i32,
    pub title: String,
    pub content: String,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Installation record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRecord {
    pub id: String,
    pub app_id: String,
    pub version: String,
    pub location: String,
    pub status: String,
    pub installed_at: DateTime<Utc>,
}

/// User settings record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsRecord {
    pub user_id: String,
    pub theme: String,
    pub notifications_enabled: bool,
    pub auto_update: bool,
    pub language: String,
    pub updated_at: DateTime<Utc>,
}

/// App configuration record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRecord {
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// Dependency record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRecord {
    pub id: String,
    pub module_id: String,
    pub depends_on: String,
    pub version_constraint: String,
    pub optional: bool,
}

// ============================================================================
// Database Migration SQL
// ============================================================================

pub const CREATE_APPS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS apps (
    id VARCHAR(36) PRIMARY KEY,
    publisher_id VARCHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT,
    icon_url VARCHAR(500),
    rating FLOAT DEFAULT 0.0,
    review_count INTEGER DEFAULT 0,
    download_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_publisher_id (publisher_id),
    INDEX idx_name (name),
    UNIQUE KEY unique_app_version (id, version)
);
"#;

pub const CREATE_MODULES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS modules (
    id VARCHAR(36) PRIMARY KEY,
    app_id VARCHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    module_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'discovered',
    file_hash VARCHAR(64),
    file_size BIGINT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (app_id) REFERENCES apps(id),
    INDEX idx_app_id (app_id),
    INDEX idx_status (status),
    UNIQUE KEY unique_module_version (id, version)
);
"#;

pub const CREATE_REVIEWS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS reviews (
    id VARCHAR(36) PRIMARY KEY,
    app_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36) NOT NULL,
    rating INTEGER CHECK (rating >= 1 AND rating <= 5),
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    helpful_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (app_id) REFERENCES apps(id),
    INDEX idx_app_id (app_id),
    INDEX idx_user_id (user_id),
    INDEX idx_rating (rating)
);
"#;

pub const CREATE_INSTALLATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS installations (
    id VARCHAR(36) PRIMARY KEY,
    app_id VARCHAR(36) NOT NULL,
    version VARCHAR(50) NOT NULL,
    location VARCHAR(500) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    installed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (app_id) REFERENCES apps(id),
    INDEX idx_app_id (app_id),
    INDEX idx_status (status)
);
"#;

pub const CREATE_SETTINGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    user_id VARCHAR(36) PRIMARY KEY,
    theme VARCHAR(50) DEFAULT 'dark',
    notifications_enabled BOOLEAN DEFAULT true,
    auto_update BOOLEAN DEFAULT true,
    language VARCHAR(10) DEFAULT 'en',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
"#;

pub const CREATE_CONFIG_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS app_config (
    app_id VARCHAR(36),
    key VARCHAR(255),
    value LONGTEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (app_id, key),
    FOREIGN KEY (app_id) REFERENCES apps(id),
    INDEX idx_app_id (app_id)
);
"#;

pub const CREATE_DEPENDENCIES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS module_dependencies (
    id VARCHAR(36) PRIMARY KEY,
    module_id VARCHAR(36) NOT NULL,
    depends_on VARCHAR(36) NOT NULL,
    version_constraint VARCHAR(50) NOT NULL,
    optional BOOLEAN DEFAULT false,
    FOREIGN KEY (module_id) REFERENCES modules(id),
    INDEX idx_module_id (module_id),
    INDEX idx_depends_on (depends_on)
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_database_connection_string() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            user: "user".to_string(),
            password: "pass".to_string(),
            database: "testdb".to_string(),
            max_connections: 10,
        };

        let conn_str = config.connection_string();
        assert!(conn_str.contains("postgres://"));
        assert!(conn_str.contains("user:pass"));
        assert!(conn_str.contains("localhost:5432"));
        assert!(conn_str.contains("testdb"));
    }

    #[test]
    fn test_app_record_structure() {
        let app = AppRecord {
            id: "test-id".to_string(),
            publisher_id: "pub-id".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            description: "A test app".to_string(),
            icon_url: "http://example.com/icon.png".to_string(),
            rating: 4.5,
            review_count: 100,
            download_count: 1000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(app.rating, 4.5);
        assert_eq!(app.download_count, 1000);
    }
}
