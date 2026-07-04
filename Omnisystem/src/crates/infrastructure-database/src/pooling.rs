use crate::{
    ConnectionPoolConfig, DatabaseError, DatabaseId, DatabaseResult, PoolStatistics,
    PooledConnection,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct ConnectionPool {
    database_id: DatabaseId,
    config: ConnectionPoolConfig,
    connections: Arc<DashMap<String, PooledConnection>>,
    total_queries: Arc<AtomicU64>,
    active_count: Arc<AtomicU32>,
    idle_count: Arc<AtomicU32>,
    waiting_requests: Arc<AtomicU32>,
}

impl ConnectionPool {
    pub fn new(database_id: DatabaseId, config: ConnectionPoolConfig) -> Self {
        let min_connections = config.min_connections;
        Self {
            database_id,
            config,
            connections: Arc::new(DashMap::new()),
            total_queries: Arc::new(AtomicU64::new(0)),
            active_count: Arc::new(AtomicU32::new(0)),
            idle_count: Arc::new(AtomicU32::new(min_connections)),
            waiting_requests: Arc::new(AtomicU32::new(0)),
        }
    }

    pub async fn acquire(&self) -> DatabaseResult<PooledConnection> {
        // Check if we can acquire a connection
        let current_active = self.active_count.load(Ordering::Relaxed);
        let current_idle = self.idle_count.load(Ordering::Relaxed);
        let total = current_active + current_idle;

        if current_idle > 0 {
            // Use idle connection
            self.idle_count.fetch_sub(1, Ordering::Relaxed);
            self.active_count.fetch_add(1, Ordering::Relaxed);

            let conn_id = Uuid::new_v4().to_string();
            let conn = PooledConnection {
                id: conn_id.clone(),
                database_id: self.database_id.clone(),
                acquired_at: Utc::now(),
                active: true,
                query_count: 0,
            };

            self.connections.insert(conn_id, conn.clone());
            Ok(conn)
        } else if total < self.config.max_connections {
            // Create new connection
            self.active_count.fetch_add(1, Ordering::Relaxed);

            let conn_id = Uuid::new_v4().to_string();
            let conn = PooledConnection {
                id: conn_id.clone(),
                database_id: self.database_id.clone(),
                acquired_at: Utc::now(),
                active: true,
                query_count: 0,
            };

            self.connections.insert(conn_id, conn.clone());
            Ok(conn)
        } else {
            // Pool exhausted
            self.waiting_requests.fetch_add(1, Ordering::Relaxed);
            Err(DatabaseError::PoolExhausted)
        }
    }

    pub async fn release(&self, conn_id: &str) -> DatabaseResult<()> {
        if let Some(mut conn) = self.connections.get_mut(conn_id) {
            conn.active = false;
            self.active_count.fetch_sub(1, Ordering::Relaxed);
            self.idle_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(DatabaseError::ConnectionFailed(
                "Connection not found".to_string(),
            ))
        }
    }

    pub async fn execute_query(&self, conn_id: &str) -> DatabaseResult<()> {
        if let Some(mut conn) = self.connections.get_mut(conn_id) {
            conn.query_count += 1;
            self.total_queries.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(DatabaseError::ConnectionFailed(
                "Connection not found".to_string(),
            ))
        }
    }

    pub async fn get_statistics(&self) -> DatabaseResult<PoolStatistics> {
        let active = self.active_count.load(Ordering::Relaxed);
        let idle = self.idle_count.load(Ordering::Relaxed);
        let waiting = self.waiting_requests.load(Ordering::Relaxed);
        let total_queries = self.total_queries.load(Ordering::Relaxed);

        Ok(PoolStatistics {
            total_connections: active + idle,
            active_connections: active,
            idle_connections: idle,
            waiting_requests: waiting,
            total_queries,
            avg_query_time_ms: if total_queries > 0 { 25.0 } else { 0.0 },
        })
    }

    pub async fn drain(&self) -> DatabaseResult<()> {
        self.connections.clear();
        self.active_count.store(0, Ordering::Relaxed);
        self.idle_count.store(self.config.min_connections, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_connection() {
        let pool = ConnectionPool::new(
            DatabaseId(Uuid::new_v4()),
            ConnectionPoolConfig::default(),
        );

        let conn = pool.acquire().await.unwrap();
        assert!(conn.active);
    }

    #[tokio::test]
    async fn test_release_connection() {
        let pool = ConnectionPool::new(
            DatabaseId(Uuid::new_v4()),
            ConnectionPoolConfig::default(),
        );

        let conn = pool.acquire().await.unwrap();
        let conn_id = conn.id.clone();

        pool.release(&conn_id).await.unwrap();

        // Check stats
        let stats = pool.get_statistics().await.unwrap();
        assert_eq!(stats.idle_connections, 5); // Back to min_connections
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_pool_exhaustion() {
        let mut config = ConnectionPoolConfig::default();
        config.max_connections = 2;
        config.min_connections = 0;

        let pool = ConnectionPool::new(DatabaseId(Uuid::new_v4()), config);

        let _conn1 = pool.acquire().await.unwrap();
        let _conn2 = pool.acquire().await.unwrap();

        let result = pool.acquire().await;
        assert!(matches!(result, Err(DatabaseError::PoolExhausted)));
    }

    #[tokio::test]
    async fn test_execute_query() {
        let pool = ConnectionPool::new(
            DatabaseId(Uuid::new_v4()),
            ConnectionPoolConfig::default(),
        );

        let conn = pool.acquire().await.unwrap();
        let conn_id = conn.id.clone();

        pool.execute_query(&conn_id).await.unwrap();

        let stats = pool.get_statistics().await.unwrap();
        assert_eq!(stats.total_queries, 1);
    }

    #[tokio::test]
    async fn test_pool_statistics() {
        let pool = ConnectionPool::new(
            DatabaseId(Uuid::new_v4()),
            ConnectionPoolConfig::default(),
        );

        let _conn = pool.acquire().await.unwrap();

        let stats = pool.get_statistics().await.unwrap();
        assert_eq!(stats.total_connections, 5); // 4 idle + 1 active = 5 total
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.idle_connections, 4);
    }

    #[tokio::test]
    async fn test_drain_pool() {
        let pool = ConnectionPool::new(
            DatabaseId(Uuid::new_v4()),
            ConnectionPoolConfig::default(),
        );

        let _conn = pool.acquire().await.unwrap();

        pool.drain().await.unwrap();

        let stats = pool.get_statistics().await.unwrap();
        assert_eq!(stats.total_connections, 5); // Only min_connections
    }
}
