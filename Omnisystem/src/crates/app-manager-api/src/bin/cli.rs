//! Small offline CLI for app-manager-api's auth/validation helpers.
//!
//! The crate's real HTTP server lives in `src/main.rs` (binary
//! `app-manager-api`, started with `start_server`); this second binary is a
//! lightweight tool for exercising the crate's real Claims/TokenManager and
//! validation logic without booting the server, instead of the dead generic
//! Component template it used to reference.

use app_manager_api::{Claims, EmailValidator, PasswordValidator, TokenManager};

fn main() {
    let email = std::env::args().nth(1).unwrap_or_else(|| "user@example.com".to_string());
    let password = std::env::args().nth(2).unwrap_or_else(|| "Sup3rSecret!".to_string());

    match EmailValidator::validate(&email) {
        Ok(()) => println!("email '{email}' is valid"),
        Err(e) => println!("email '{email}' is invalid: {e:?}"),
    }

    match PasswordValidator::validate(&password) {
        Ok(()) => println!("password meets policy requirements"),
        Err(e) => println!("password rejected: {e:?}"),
    }

    let claims = Claims::new("user-1".to_string(), email, vec!["user".to_string()]);
    let token = TokenManager::generate_token(&claims).expect("mock token generation cannot fail");
    println!("issued token: {token}");

    match TokenManager::verify_token(&token) {
        Ok(decoded) => println!("verified token for user_id={} expired={}", decoded.user_id, decoded.is_expired()),
        Err(e) => println!("token verification failed: {e}"),
    }
}
