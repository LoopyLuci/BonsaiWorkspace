//! CLI demo: register a tenant and check an access-control decision.

use multi_tenant::{AccessControlManager, Tenant, TenantContext, TenantIsolationManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tenants = TenantIsolationManager::new();
    let access = AccessControlManager::new();

    tenants
        .create_tenant(&Tenant {
            tenant_id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            status: "active".to_string(),
            max_users: 50,
            max_storage_gb: 100,
        })
        .await?;

    let tenant = tenants.get_tenant("acme-corp").await?;
    println!("Registered tenant: {} ({})", tenant.name, tenant.status);

    let allowed = access
        .check_access(
            &TenantContext {
                tenant_id: "acme-corp".to_string(),
                user_id: "u1".to_string(),
                roles: vec!["admin".to_string()],
            },
            "documents",
            "read",
        )
        .await?;
    println!("Access allowed: {}", allowed);

    Ok(())
}
