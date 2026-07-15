//! app-manager-security CLI
//!
//! Small demo/utility binary: register a security policy for an app,
//! grant it a permission, and log an audit event for the action.

use app_manager_security::{
    AuditLogger, Permission, PermissionManager, ResourceLimits, SandboxLevel, SecurityPolicy,
};
use app_manager_core::types::AppId;
use std::collections::HashSet;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let app_name = args.get(1).map(String::as_str).unwrap_or("demo-app");

    let app_id = AppId::new(app_name)?;

    let permissions = PermissionManager::new();
    permissions.register_policy(SecurityPolicy {
        app_id: app_id.clone(),
        permissions: HashSet::new(),
        resource_limits: ResourceLimits {
            cpu_percent: 25,
            memory_mb: 256,
            disk_quota_mb: 512,
            network_bandwidth_mbps: 10,
        },
        sandbox_level: SandboxLevel::Basic,
        trusted: false,
    })?;

    permissions.grant_permission(&app_id, Permission::FilesystemRead)?;
    let granted = permissions.has_permission(&app_id, Permission::FilesystemRead)?;
    println!("{} granted FilesystemRead: {}", app_id, granted);

    let audit = AuditLogger::new();
    audit.log(app_id.clone(), "grant_permission", "cli", true);
    println!("audit events for {}: {}", app_id, audit.get_events_for_app(&app_id).len());

    Ok(())
}
