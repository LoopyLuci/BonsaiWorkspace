use crate::{
    Backup, BackupConfig, BackupStatus, Database, DatabaseConfig, DatabaseId,
    DatabaseMetrics, DatabaseResult, ReplicationConfig, ReplicationStatus,
};
use async_trait::async_trait;

#[async_trait]
pub trait DatabaseManager: Send + Sync {
    async fn create_database(&self, config: DatabaseConfig) -> DatabaseResult<Database>;

    async fn delete_database(&self, id: &DatabaseId) -> DatabaseResult<()>;

    async fn get_database(&self, id: &DatabaseId) -> DatabaseResult<Database>;

    async fn list_databases(&self) -> DatabaseResult<Vec<Database>>;

    async fn start_database(&self, id: &DatabaseId) -> DatabaseResult<()>;

    async fn stop_database(&self, id: &DatabaseId) -> DatabaseResult<()>;

    async fn restart_database(&self, id: &DatabaseId) -> DatabaseResult<()>;

    async fn get_metrics(&self, id: &DatabaseId) -> DatabaseResult<DatabaseMetrics>;

    async fn execute_query(
        &self,
        id: &DatabaseId,
        query: &str,
    ) -> DatabaseResult<String>;
}

#[async_trait]
pub trait BackupManager: Send + Sync {
    async fn create_backup(
        &self,
        database_id: &DatabaseId,
        config: &BackupConfig,
    ) -> DatabaseResult<Backup>;

    async fn restore_backup(
        &self,
        backup_id: &str,
        target_database_id: &DatabaseId,
    ) -> DatabaseResult<()>;

    async fn list_backups(&self, database_id: &DatabaseId) -> DatabaseResult<Vec<Backup>>;

    async fn delete_backup(&self, backup_id: &str) -> DatabaseResult<()>;

    async fn verify_backup(&self, backup_id: &str) -> DatabaseResult<bool>;

    async fn get_backup_status(&self, backup_id: &str) -> DatabaseResult<BackupStatus>;

    async fn schedule_backups(
        &self,
        database_id: &DatabaseId,
        config: &BackupConfig,
    ) -> DatabaseResult<()>;
}

#[async_trait]
pub trait ReplicationManager: Send + Sync {
    async fn configure_replication(
        &self,
        database_id: &DatabaseId,
        config: ReplicationConfig,
    ) -> DatabaseResult<()>;

    async fn get_replication_status(
        &self,
        database_id: &DatabaseId,
    ) -> DatabaseResult<ReplicationStatus>;

    async fn promote_replica(
        &self,
        replica_id: &str,
    ) -> DatabaseResult<DatabaseId>;

    async fn add_replica(
        &self,
        primary_id: &DatabaseId,
        replica_host: String,
    ) -> DatabaseResult<String>;

    async fn remove_replica(&self, replica_id: &str) -> DatabaseResult<()>;

    async fn check_replication_lag(
        &self,
        database_id: &DatabaseId,
    ) -> DatabaseResult<u64>;

    async fn trigger_failover(&self, database_id: &DatabaseId) -> DatabaseResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_result_ok() {
        let result: DatabaseResult<String> = Ok("success".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_database_result_err() {
        let result: DatabaseResult<String> =
            Err(crate::DatabaseError::DatabaseNotFound("test".to_string()));
        assert!(result.is_err());
    }
}
