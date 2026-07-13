//! Demo CLI: creates a temp SQLite-backed StorageManager, writes a tenant and an
//! API key, and reads them back through the real storage API.

use freellmapi_core::{unix_now, ApiKey, Tenant};
use freellmapi_storage::StorageManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("freellmapi-storage-demo-{}", std::process::id()));
    let db_path = dir.join("demo.db");
    let manager = StorageManager::new(db_path.to_str().unwrap()).await?;

    let tenant = Tenant {
        id: "demo-tenant".to_string(),
        name: "Demo Tenant".to_string(),
        email: "demo@example.com".to_string(),
        tier: "free".to_string(),
        monthly_budget_usd: 25.0,
        created_at: unix_now(),
    };
    manager.create_tenant(&tenant).await?;

    let key = ApiKey {
        id: "demo-key".to_string(),
        tenant_id: tenant.id.clone(),
        key_hash: "demo-hash".to_string(),
        scopes: vec!["chat".to_string()],
        created_at: unix_now(),
        expires_at: None,
    };
    manager.create_api_key(&key).await?;

    let fetched = manager.get_tenant(&tenant.id).await?;
    println!("Fetched tenant: {} <{}>", fetched.name, fetched.email);

    let fetched_key = manager.get_api_key_by_hash(&key.key_hash).await?;
    println!("Fetched key scopes: {:?}", fetched_key.scopes);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
