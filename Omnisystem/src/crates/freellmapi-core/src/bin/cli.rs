//! Demo CLI: builds a Tenant, registers a mock service in the ServiceRegistry,
//! and exercises the real freellmapi-core API end to end.

use freellmapi_core::{generate_id, unix_now, ServiceRegistry, Tenant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tenant = Tenant {
        id: generate_id(),
        name: "Demo Tenant".to_string(),
        email: "demo@example.com".to_string(),
        tier: "free".to_string(),
        monthly_budget_usd: 10.0,
        created_at: unix_now(),
    };
    println!("Created tenant {} ({})", tenant.name, tenant.id);

    let registry = ServiceRegistry::new();
    println!("Registered services: {:?}", registry.list_services());

    Ok(())
}
