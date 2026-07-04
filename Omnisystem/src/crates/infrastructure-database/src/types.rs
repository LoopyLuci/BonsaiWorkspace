use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DatabaseId(pub Uuid);

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum DatabaseEngine {
    PostgreSQL,
    MySQL,
    MongoDB,
    Redis,
}

impl std::fmt::Display for DatabaseEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseEngine::PostgreSQL => write!(f, "PostgreSQL"),
            DatabaseEngine::MySQL => write!(f, "MySQL"),
            DatabaseEngine::MongoDB => write!(f, "MongoDB"),
            DatabaseEngine::Redis => write!(f, "Redis"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub id: DatabaseId,
    pub name: String,
    pub engine: DatabaseEngine,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: String,
    pub max_connections: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Database {
    pub id: DatabaseId,
    pub name: String,
    pub engine: DatabaseEngine,
    pub version: String,
    pub status: DatabaseStatus,
    pub size_bytes: u64,
    pub connection_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_backup: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DatabaseStatus {
    Running,
    Stopped,
    Restarting,
    #[serde(rename = "Backing Up")]
    BackingUp,
    Restoring,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            connection_timeout_secs: 30,
            idle_timeout_secs: 300,
            max_lifetime_secs: 1800,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PooledConnection {
    pub id: String,
    pub database_id: DatabaseId,
    pub acquired_at: DateTime<Utc>,
    pub active: bool,
    pub query_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_requests: u32,
    pub total_queries: u64,
    pub avg_query_time_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub schedule: String,
    pub retention_days: u32,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub destination: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            schedule: "0 2 * * *".to_string(), // 2 AM daily
            retention_days: 30,
            compression_enabled: true,
            encryption_enabled: true,
            destination: "/backups".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Backup {
    pub id: String,
    pub database_id: DatabaseId,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    pub status: BackupStatus,
    pub location: String,
    pub encryption_key_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub enabled: bool,
    pub replication_factor: u32,
    pub replication_lag_tolerance_secs: u64,
    pub failover_enabled: bool,
    pub replicas: Vec<String>,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            replication_factor: 1,
            replication_lag_tolerance_secs: 10,
            failover_enabled: false,
            replicas: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationStatus {
    pub database_id: DatabaseId,
    pub is_primary: bool,
    pub replication_lag_secs: u64,
    pub last_sync: DateTime<Utc>,
    pub healthy_replicas: u32,
    pub total_replicas: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub database_id: DatabaseId,
    pub timestamp: DateTime<Utc>,
    pub connection_count: u32,
    pub query_count: u64,
    pub slow_queries: u32,
    pub cache_hit_rate: f64,
    pub disk_usage_bytes: u64,
    pub memory_usage_bytes: u64,
    pub avg_query_time_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Migration {
    pub id: String,
    pub database_id: DatabaseId,
    pub version: String,
    pub description: String,
    pub executed_at: Option<DateTime<Utc>>,
    pub status: MigrationStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum MigrationStatus {
    Pending,
    Applied,
    Failed,
    #[serde(rename = "Rolled Back")]
    RolledBack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_engine_display() {
        assert_eq!(DatabaseEngine::PostgreSQL.to_string(), "PostgreSQL");
        assert_eq!(DatabaseEngine::MySQL.to_string(), "MySQL");
        assert_eq!(DatabaseEngine::MongoDB.to_string(), "MongoDB");
    }

    #[test]
    fn test_connection_pool_config_defaults() {
        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
    }

    #[test]
    fn test_backup_config_defaults() {
        let config = BackupConfig::default();
        assert!(config.enabled);
        assert_eq!(config.retention_days, 30);
    }

    #[test]
    fn test_database_status_equality() {
        assert_eq!(DatabaseStatus::Running, DatabaseStatus::Running);
        assert_ne!(DatabaseStatus::Running, DatabaseStatus::Stopped);
    }

    #[test]
    fn test_replication_config_defaults() {
        let config = ReplicationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.replication_factor, 1);
    }

    #[test]
    fn test_database_id_uniqueness() {
        let id1 = DatabaseId(Uuid::new_v4());
        let id2 = DatabaseId(Uuid::new_v4());
        assert_ne!(id1, id2);
    }
}
