//! CLI demo: register a user, authenticate, and check a granted permission.

use auth_system::{AuthenticationManager, AuthorizationManager, Permission, User};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = AuthenticationManager::new();
    let authz = AuthorizationManager::new();

    auth.register(&User {
        user_id: "u1".to_string(),
        username: "alice".to_string(),
        password_hash: "hashed".to_string(),
        created_at: Utc::now(),
    })
    .await?;

    let token = auth.authenticate("u1", "password").await?;
    println!("Authenticated, token: {}", token.token_id);

    authz
        .grant_permission(
            "u1",
            &Permission {
                permission_id: "p1".to_string(),
                resource: "documents".to_string(),
                action: "read".to_string(),
            },
        )
        .await?;

    let allowed = authz.check_permission("u1", "documents", "read").await?;
    println!("Permission granted: {}", allowed);

    Ok(())
}
