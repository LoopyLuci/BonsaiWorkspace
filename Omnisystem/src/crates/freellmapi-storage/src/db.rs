use freellmapi_core::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;

pub struct StorageManager {
    pool: SqlitePool,
}

impl StorageManager {
    pub async fn new(db_url: &str) -> Result<Self> {
        // Accept either a bare filesystem path (Windows or Unix style) or an
        // already-formed sqlite:// URL. Bare paths get their parent directory
        // created and are opened via SqliteConnectOptions (which understands
        // native path separators), rather than being hand-assembled into a
        // URL string - that broke on Windows paths like `C:\foo\bar.db`.
        let connect_options = if db_url.starts_with("sqlite://") || db_url.starts_with("sqlite:") {
            SqliteConnectOptions::from_str(db_url)?
        } else {
            if let Some(parent) = std::path::Path::new(db_url).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            SqliteConnectOptions::new().filename(db_url)
        }
        .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await?;

        // Enable WAL mode
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        // Create tables
        Self::create_tables(&pool).await?;

        Ok(StorageManager { pool })
    }

    async fn create_tables(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                tier TEXT NOT NULL,
                monthly_budget_usd REAL NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                scopes TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                FOREIGN KEY(tenant_id) REFERENCES tenants(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS request_log (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                api_key_id TEXT NOT NULL,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                tokens_in INTEGER NOT NULL,
                tokens_out INTEGER NOT NULL,
                cost_usd REAL NOT NULL,
                latency_ms INTEGER NOT NULL,
                status_code INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(tenant_id) REFERENCES tenants(id),
                FOREIGN KEY(api_key_id) REFERENCES api_keys(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS webhooks (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                url TEXT NOT NULL,
                events TEXT NOT NULL,
                secret_key TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(tenant_id) REFERENCES tenants(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_tenant ON api_keys(tenant_id)")
            .execute(pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_request_log_tenant ON request_log(tenant_id)")
            .execute(pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_request_log_created ON request_log(created_at)")
            .execute(pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_request_log_tenant_created ON request_log(tenant_id, created_at)",
        )
        .execute(pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_webhooks_tenant ON webhooks(tenant_id)")
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn create_tenant(&self, tenant: &Tenant) -> Result<()> {
        sqlx::query(
            "INSERT INTO tenants (id, name, email, tier, monthly_budget_usd, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.email)
        .bind(&tenant.tier)
        .bind(tenant.monthly_budget_usd)
        .bind(tenant.created_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_tenant(&self, id: &str) -> Result<Tenant> {
        let row = sqlx::query(
            "SELECT id, name, email, tier, monthly_budget_usd, created_at FROM tenants WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("Tenant not found"))?;

        Ok(Tenant {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            tier: row.get(3),
            monthly_budget_usd: row.get(4),
            created_at: row.get::<i64, _>(5) as u64,
        })
    }

    pub async fn create_api_key(&self, key: &ApiKey) -> Result<()> {
        let scopes_json = serde_json::to_string(&key.scopes)?;

        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, scopes, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&key.id)
        .bind(&key.tenant_id)
        .bind(&key.key_hash)
        .bind(&scopes_json)
        .bind(key.created_at as i64)
        .bind(key.expires_at.map(|t| t as i64))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_api_key_by_hash(&self, hash: &str) -> Result<ApiKey> {
        let row = sqlx::query(
            "SELECT id, tenant_id, key_hash, scopes, created_at, expires_at FROM api_keys WHERE key_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("API key not found"))?;

        let scopes_json: String = row.get(3);
        let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();

        Ok(ApiKey {
            id: row.get(0),
            tenant_id: row.get(1),
            key_hash: row.get(2),
            scopes,
            created_at: row.get::<i64, _>(4) as u64,
            expires_at: row.get::<Option<i64>, _>(5).map(|t| t as u64),
        })
    }

    pub async fn log_request(&self, log: &RequestLog) -> Result<()> {
        sqlx::query(
            "INSERT INTO request_log (id, tenant_id, api_key_id, model, provider, tokens_in, tokens_out, cost_usd, latency_ms, status_code, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.tenant_id)
        .bind(&log.api_key_id)
        .bind(&log.model)
        .bind(&log.provider)
        .bind(log.tokens_in)
        .bind(log.tokens_out)
        .bind(log.cost_usd)
        .bind(log.latency_ms)
        .bind(log.status_code)
        .bind(log.created_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_request_logs(
        &self,
        tenant_id: &str,
        start_time: u64,
        end_time: u64,
        limit: u32,
    ) -> Result<Vec<RequestLog>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, api_key_id, model, provider, tokens_in, tokens_out, cost_usd, latency_ms, status_code, created_at
             FROM request_log WHERE tenant_id = ? AND created_at >= ? AND created_at <= ?
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(tenant_id)
        .bind(start_time as i64)
        .bind(end_time as i64)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        let logs = rows
            .iter()
            .map(|row| RequestLog {
                id: row.get(0),
                tenant_id: row.get(1),
                api_key_id: row.get(2),
                model: row.get(3),
                provider: row.get(4),
                tokens_in: row.get(5),
                tokens_out: row.get(6),
                cost_usd: row.get(7),
                latency_ms: row.get(8),
                status_code: row.get(9),
                created_at: row.get::<i64, _>(10) as u64,
            })
            .collect();

        Ok(logs)
    }

    pub async fn get_tenant_costs(&self, tenant_id: &str, days: u32) -> Result<f64> {
        let cutoff_time = unix_now() - (days as u64 * 86400);

        let row = sqlx::query(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM request_log WHERE tenant_id = ? AND created_at > ?",
        )
        .bind(tenant_id)
        .bind(cutoff_time as i64)
        .fetch_one(&self.pool)
        .await?;

        let cost: f64 = row.get(0);
        Ok(cost)
    }

    pub async fn create_webhook(&self, webhook: &Webhook) -> Result<()> {
        let events_json = serde_json::to_string(&webhook.events)?;

        sqlx::query(
            "INSERT INTO webhooks (id, tenant_id, url, events, secret_key, enabled, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&webhook.id)
        .bind(&webhook.tenant_id)
        .bind(&webhook.url)
        .bind(&events_json)
        .bind(&webhook.secret_key)
        .bind(webhook.enabled as i32)
        .bind(webhook.created_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_webhooks(&self, tenant_id: &str) -> Result<Vec<Webhook>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, url, events, secret_key, enabled, created_at FROM webhooks WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let webhooks = rows
            .iter()
            .map(|row| {
                let events_json: String = row.get(3);
                let events: Vec<String> =
                    serde_json::from_str(&events_json).unwrap_or_default();
                let enabled: i32 = row.get(5);

                Webhook {
                    id: row.get(0),
                    tenant_id: row.get(1),
                    url: row.get(2),
                    events,
                    secret_key: row.get(4),
                    enabled: enabled != 0,
                    created_at: row.get::<i64, _>(6) as u64,
                }
            })
            .collect();

        Ok(webhooks)
    }
}

#[async_trait]
impl OmnisystemService for StorageManager {
    fn service_id(&self) -> &str {
        "freellmapi-storage"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Storage"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        self.health_check().await?;
        tracing::info!("Storage service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Storage health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_manager() -> (StorageManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let manager = StorageManager::new(path.to_str().unwrap()).await.unwrap();
        (manager, dir)
    }

    #[tokio::test]
    async fn test_health_check() {
        let (manager, _dir) = test_manager().await;
        assert!(manager.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_create_and_get_tenant() {
        let (manager, _dir) = test_manager().await;
        let tenant = Tenant {
            id: "tenant-1".to_string(),
            name: "Acme".to_string(),
            email: "acme@example.com".to_string(),
            tier: "pro".to_string(),
            monthly_budget_usd: 500.0,
            created_at: unix_now(),
        };

        manager.create_tenant(&tenant).await.unwrap();
        let fetched = manager.get_tenant("tenant-1").await.unwrap();
        assert_eq!(fetched.name, "Acme");
        assert_eq!(fetched.monthly_budget_usd, 500.0);
    }

    #[tokio::test]
    async fn test_get_missing_tenant_errors() {
        let (manager, _dir) = test_manager().await;
        assert!(manager.get_tenant("does-not-exist").await.is_err());
    }

    #[tokio::test]
    async fn test_create_and_lookup_api_key() {
        let (manager, _dir) = test_manager().await;
        let tenant = Tenant {
            id: "tenant-1".to_string(),
            name: "Acme".to_string(),
            email: "acme@example.com".to_string(),
            tier: "free".to_string(),
            monthly_budget_usd: 10.0,
            created_at: unix_now(),
        };
        manager.create_tenant(&tenant).await.unwrap();

        let key = ApiKey {
            id: "key-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            key_hash: "deadbeef".to_string(),
            scopes: vec!["chat".to_string(), "models".to_string()],
            created_at: unix_now(),
            expires_at: None,
        };
        manager.create_api_key(&key).await.unwrap();

        let fetched = manager.get_api_key_by_hash("deadbeef").await.unwrap();
        assert_eq!(fetched.tenant_id, "tenant-1");
        assert_eq!(fetched.scopes, vec!["chat".to_string(), "models".to_string()]);
    }

    #[tokio::test]
    async fn test_log_request_and_query() {
        let (manager, _dir) = test_manager().await;
        let tenant = Tenant {
            id: "tenant-1".to_string(),
            name: "Acme".to_string(),
            email: "acme@example.com".to_string(),
            tier: "free".to_string(),
            monthly_budget_usd: 10.0,
            created_at: unix_now(),
        };
        manager.create_tenant(&tenant).await.unwrap();
        manager
            .create_api_key(&ApiKey {
                id: "key-1".to_string(),
                tenant_id: "tenant-1".to_string(),
                key_hash: "hash-1".to_string(),
                scopes: vec![],
                created_at: unix_now(),
                expires_at: None,
            })
            .await
            .unwrap();

        let now = unix_now();
        let log = RequestLog {
            id: "req-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            api_key_id: "key-1".to_string(),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            tokens_in: 100,
            tokens_out: 50,
            cost_usd: 0.012,
            latency_ms: 250,
            status_code: 200,
            created_at: now,
        };
        manager.log_request(&log).await.unwrap();

        let logs = manager
            .get_request_logs("tenant-1", now - 10, now + 10, 10)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].model, "gpt-4");

        let cost = manager.get_tenant_costs("tenant-1", 30).await.unwrap();
        assert!((cost - 0.012).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_webhook_roundtrip() {
        let (manager, _dir) = test_manager().await;
        let tenant = Tenant {
            id: "tenant-1".to_string(),
            name: "Acme".to_string(),
            email: "acme@example.com".to_string(),
            tier: "free".to_string(),
            monthly_budget_usd: 10.0,
            created_at: unix_now(),
        };
        manager.create_tenant(&tenant).await.unwrap();

        let webhook = Webhook {
            id: "wh-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            url: "https://example.com/hook".to_string(),
            events: vec!["request.completed".to_string()],
            secret_key: "shh".to_string(),
            enabled: true,
            created_at: unix_now(),
        };
        manager.create_webhook(&webhook).await.unwrap();

        let hooks = manager.get_webhooks("tenant-1").await.unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].url, "https://example.com/hook");
        assert!(hooks[0].enabled);
    }
}
