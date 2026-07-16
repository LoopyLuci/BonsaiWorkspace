//! CLI that exercises the real bcrypt/JWT auth primitives end to end.
//!
//! Requires `JWT_SECRET` (>= 32 characters) to be set in the environment;
//! there is no hardcoded fallback secret.

use pathfinder_user_service::auth;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let password = "correct horse battery staple";
    let hashed = auth::hash_password(password)?;
    println!("Hashed password: {hashed}");

    auth::verify_password(password, &hashed)?;
    println!("Password verified successfully");

    let token = auth::generate_jwt("demo-user")?;
    println!("Issued JWT: {token}");

    let claims = auth::validate_jwt(&token)?;
    println!(
        "Validated JWT for subject '{}' issued by '{}'",
        claims.sub, claims.iss
    );

    Ok(())
}
