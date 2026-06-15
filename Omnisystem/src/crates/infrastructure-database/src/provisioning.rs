use crate::{
    Database, DatabaseConfig, DatabaseId, DatabaseManager, DatabaseResult,
    DatabaseStatus, DatabaseMetrics, DatabaseError,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use chrono::Utc;

pub struct DatabaseProvisioner {
    databases: Arc<DashMap<String, Database>>,
    configs: Arc<DashMap<String, DatabaseConfig>>,
}

impl DatabaseProvisioner {
    pub fn new() -> Self {
        Self {
            databases: Arc::new(DashMap::new()),
            configs: Arc::new(DashMap::new()),
        }
    }

    pub fn instance_count(&self) -> usize {
        self.databases.len()
    }

    pub fn running_count(&self) -> usize {
        self.databases
            .iter()
            .filter(|entry| entry.status == DatabaseStatus::Running)
            .count()
    }
}

impl Default for DatabaseProvisioner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseManager for DatabaseProvisioner {
    async fn create_database(&self, config: DatabaseConfig) -> DatabaseResult<Database> {
        if self.databases.contains_key(&config.id.0.to_string()) {
            return Err(DatabaseError::DatabaseAlreadyExists(config.name.clone()));
        }

        let now = Utc::now();
        let database = Database {
            id: config.id.clone(),
            name: config.name.clone(),
            engine: config.engine,
            version: config.version.clone(),
            status: DatabaseStatus::Running,
            size_bytes: 0,
            connection_count: 0,
            created_at: now,
            updated_at: now,
            last_backup: None,
            tags: config.tags.clone(),
        };

        self.configs.insert(config.id.0.to_string(), config);
        self.databases.insert(database.id.0.to_string(), database.clone());

        Ok(database)
    }

    async fn delete_database(&self, id: &DatabaseId) -> DatabaseResult<()> {
        if !self.databases.contains_key(&id.0.to_string()) {
            return Err(DatabaseError::DatabaseNotFound(id.0.to_string()));
        }

        self.databases.remove(&id.0.to_string());
        self.configs.remove(&id.0.to_string());

        Ok(())
    }

    async fn get_database(&self, id: &DatabaseId) -> DatabaseResult<Database> {
        self.databases
            .get(&id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| DatabaseError::DatabaseNotFound(id.0.to_string()))
    }

    async fn list_databases(&self) -> DatabaseResult<Vec<Database>> {
        Ok(self
            .databases
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn start_database(&self, id: &DatabaseId) -> DatabaseResult<()> {
        if let Some(mut db) = self.databases.get_mut(&id.0.to_string()) {
            db.status = DatabaseStatus::Running;
            db.updated_at = Utc::now();
            Ok(())
        } else {
            Err(DatabaseError::DatabaseNotFound(id.0.to_string()))
        }
    }

    async fn stop_database(&self, id: &DatabaseId) -> DatabaseResult<()> {
        if let Some(mut db) = self.databases.get_mut(&id.0.to_string()) {
            db.status = DatabaseStatus::Stopped;
            db.updated_at = Utc::now();
            db.connection_count = 0;
            Ok(())
        } else {
            Err(DatabaseError::DatabaseNotFound(id.0.to_string()))
        }
    }

    async fn restart_database(&self, id: &DatabaseId) -> DatabaseResult<()> {
        if let Some(mut db) = self.databases.get_mut(&id.0.to_string()) {
            db.status = DatabaseStatus::Restarting;
            db.updated_at = Utc::now();
            // Simulate restart by setting back to running
            db.status = DatabaseStatus::Running;
            db.connection_count = 0;
            Ok(())
        } else {
            Err(DatabaseError::DatabaseNotFound(id.0.to_string()))
        }
    }

    async fn get_metrics(&self, id: &DatabaseId) -> DatabaseResult<DatabaseMetrics> {
        let db = self.get_database(id).await?;

        Ok(DatabaseMetrics {
            database_id: id.clone(),
            timestamp: Utc::now(),
            connection_count: db.connection_count,
            query_count: 0,
            slow_queries: 0,
            cache_hit_rate: 0.95,
            disk_usage_bytes: db.size_bytes,
            memory_usage_bytes: 512 * 1024 * 1024,
            avg_query_time_ms: 10.5,
        })
    }

    async fn execute_query(
        &self,
        id: &DatabaseId,
        query: &str,
    ) -> DatabaseResult<String> {
        let db = self.get_database(id).await?;

        if db.status != DatabaseStatus::Running {
            return Err(DatabaseError::ConnectionFailed(
                "Database is not running".to_string(),
            ));
        }

        // Simulate query execution
        Ok(format!("Executed query on {}: {}", db.name, query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseEngine;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_database() {
        let provisioner = DatabaseProvisioner::new();
        let config = DatabaseConfig {
            id: DatabaseId(Uuid::new_v4()),
            name: "test_db".to_string(),
            engine: DatabaseEngine::PostgreSQL,
            version: "14.5".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "admin".to_string(),
            password: "password".to_string(),
            database_name: "test_db".to_string(),
            max_connections: 20,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Default::default(),
        };

        let db = provisioner.create_database(config.clone()).await.unwrap();
        assert_eq!(db.name, "test_db");
        assert_eq!(db.status, DatabaseStatus::Running);
    }

    #[tokio::test]
    async fn test_duplicate_database() {
        let provisioner = DatabaseProvisioner::new();
        let id = DatabaseId(Uuid::new_v4());
        let config = DatabaseConfig {
            id: id.clone(),
            name: "test".to_string(),
            engine: DatabaseEngine::MySQL,
            version: "8.0".to_string(),
            host: "localhost".to_string(),
            port: 3306,
            username: "user".to_string(),
            password: "pass".to_string(),
            database_name: "test".to_string(),
            max_connections: 20,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Default::default(),
        };

        provisioner.create_database(config.clone()).await.unwrap();
        let result = provisioner.create_database(config).await;
        assert!(matches!(
            result,
            Err(DatabaseError::DatabaseAlreadyExists(_))
        ));
    }

    #[tokio::test]
    async fn test_start_stop_database() {
        let provisioner = DatabaseProvisioner::new();
        let config = DatabaseConfig {
            id: DatabaseId(Uuid::new_v4()),
            name: "db".to_string(),
            engine: DatabaseEngine::PostgreSQL,
            version: "14".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "user".to_string(),
            password: "pass".to_string(),
            database_name: "db".to_string(),
            max_connections: 20,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Default::default(),
        };

        let db = provisioner.create_database(config).await.unwrap();
        assert_eq!(db.status, DatabaseStatus::Running);

        provisioner.stop_database(&db.id).await.unwrap();
        let stopped = provisioner.get_database(&db.id).await.unwrap();
        assert_eq!(stopped.status, DatabaseStatus::Stopped);

        provisioner.start_database(&db.id).await.unwrap();
        let restarted = provisioner.get_database(&db.id).await.unwrap();
        assert_eq!(restarted.status, DatabaseStatus::Running);
    }

    #[tokio::test]
    async fn test_list_databases() {
        let provisioner = DatabaseProvisioner::new();

        for i in 0..3 {
            let config = DatabaseConfig {
                id: DatabaseId(Uuid::new_v4()),
                name: format!("db{}", i),
                engine: DatabaseEngine::PostgreSQL,
                version: "14".to_string(),
                host: "localhost".to_string(),
                port: 5432 + i,
                username: "user".to_string(),
                password: "pass".to_string(),
                database_name: format!("db{}", i),
                max_connections: 20,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: Default::default(),
            };
            provisioner.create_database(config).await.unwrap();
        }

        let databases = provisioner.list_databases().await.unwrap();
        assert_eq!(databases.len(), 3);
    }

    #[tokio::test]
    async fn test_execute_query() {
        let provisioner = DatabaseProvisioner::new();
        let config = DatabaseConfig {
            id: DatabaseId(Uuid::new_v4()),
            name: "test".to_string(),
            engine: DatabaseEngine::PostgreSQL,
            version: "14".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "user".to_string(),
            password: "pass".to_string(),
            database_name: "test".to_string(),
            max_connections: 20,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Default::default(),
        };

        let db = provisioner.create_database(config).await.unwrap();
        let result = provisioner
            .execute_query(&db.id, "SELECT * FROM users")
            .await
            .unwrap();
        assert!(result.contains("SELECT * FROM users"));
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let provisioner = DatabaseProvisioner::new();
        let config = DatabaseConfig {
            id: DatabaseId(Uuid::new_v4()),
            name: "test".to_string(),
            engine: DatabaseEngine::PostgreSQL,
            version: "14".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "user".to_string(),
            password: "pass".to_string(),
            database_name: "test".to_string(),
            max_connections: 20,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: Default::default(),
        };

        let db = provisioner.create_database(config).await.unwrap();
        let metrics = provisioner.get_metrics(&db.id).await.unwrap();
        assert_eq!(metrics.database_id, db.id);
        assert!(metrics.cache_hit_rate > 0.0);
    }
}
